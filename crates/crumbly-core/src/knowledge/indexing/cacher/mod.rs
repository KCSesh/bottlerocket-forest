//! Context-free chunk caching for pre-populating the CAS chunk store
//!
//! Provides [`ChunkCacher`] for caching chunks and embeddings without context
//! association. This enables pre-warming the content-addressed store so that
//! subsequent context creation can reuse existing embeddings.

mod types;

pub use types::CacheResult;

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use bon::Builder;
use snafu::{ResultExt, Snafu};

use super::indexer::pipeline::{EmbeddingPipeline, PipelineHandle, WorkItem};
use super::source::ContentSource;
use super::{BatchConfig, IndexDataProvider, IndexingError, ProgressReporter};
use crate::knowledge::chunking::{ChunkingDispatcher, ChunkingInput, DispatchError};
use crate::knowledge::domain::{
    Chunk, ChunkHash, ChunkSource, ChunkableContent, FileHash, FilePeek, IndexedChunk,
};
use crate::knowledge::storage::{ChunkRepository, StorageError};

/// Send chunks to pipeline, returning error if pipeline failed
fn send_chunks<F, E: std::error::Error + 'static>(
    chunks: Vec<Chunk>,
    existing: &HashSet<ChunkHash>,
    handle: &PipelineHandle<F>,
    embeddings_generated: &mut usize,
) -> Result<(), CacheError<E>> {
    for chunk in chunks
        .into_iter()
        .filter(|c| !existing.contains(&c.chunk_hash))
    {
        *embeddings_generated += 1;
        if handle.sender().send(WorkItem { chunk }).is_err() {
            if let Some(e) = handle.check_error() {
                return Err(CacheError::EmbeddingFailed { source: e });
            }
            break;
        }
    }
    Ok(())
}

/// Caches chunks and embeddings without context association.
#[derive(Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct ChunkCacher<S: ContentSource, R: ChunkRepository> {
    source: S,
    dispatcher: ChunkingDispatcher,
    repository: R,
    provider: Arc<dyn IndexDataProvider>,
    #[builder(default)]
    batch_config: BatchConfig,
    progress: Option<Arc<dyn ProgressReporter>>,
}

impl<S: ContentSource, R: ChunkRepository> ChunkCacher<S, R> {
    /// Cache all content from source, skipping existing embeddings.
    pub fn cache(&mut self) -> Result<CacheResult, CacheError<S::Error>> {
        use cache_error::*;

        let entries = self.source.scan().context(ScanFailedSnafu)?;
        let entries_scanned = entries.len();

        let worker_count = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
        let batch_size = self.batch_config.batch_size.into_inner();

        let storage_batch: Arc<Mutex<Vec<IndexedChunk>>> =
            Arc::new(Mutex::new(Vec::with_capacity(batch_size)));
        let storage_batch_clone = Arc::clone(&storage_batch);
        let chunks_created = Arc::new(AtomicUsize::new(0));
        let chunks_created_clone = Arc::clone(&chunks_created);

        let pipeline = EmbeddingPipeline::builder()
            .provider(Arc::clone(&self.provider))
            .worker_count(worker_count)
            .maybe_progress(self.progress.clone())
            .build();

        let handle = pipeline.start_with_sink(move |chunk: IndexedChunk| {
            if let Ok(mut batch) = storage_batch_clone.lock() {
                batch.push(chunk);
            }
            chunks_created_clone.fetch_add(1, Ordering::Relaxed);
        });

        let mut chunks_skipped: usize = 0;
        let mut embeddings_generated: usize = 0;

        for entry in &entries {
            if let Some(e) = handle.check_error() {
                let _ = handle.join();
                return Err(CacheError::EmbeddingFailed { source: e });
            }

            let content = self.source.fetch(entry).context(FetchFailedSnafu)?;
            let file_hash = FileHash::from_reader(std::io::Cursor::new(content.as_bytes()))
                .context(HashComputeSnafu)?;

            let input = ChunkingInput {
                content: ChunkableContent::new(content),
                source: ChunkSource::builder()
                    .file_path(entry.relative_path.clone())
                    .repo_name(entry.repo_name.clone())
                    .build(),
                file_hash,
            };

            let file_peek = FilePeek::from_path_string(&entry.relative_path.to_string());

            let chunks = match self.dispatcher.chunk_file(&input, &file_peek) {
                Some(Ok(c)) => c,
                Some(Err(e)) => return Err(e).context(ChunkingFailedSnafu),
                None => continue,
            };

            if chunks.is_empty() {
                continue;
            }

            let hashes: Vec<ChunkHash> = chunks.iter().map(|c| c.chunk_hash).collect();
            let existing = self
                .repository
                .has_embedding_batch(&hashes)
                .context(StorageFailedSnafu)?;

            chunks_skipped += existing.len();

            send_chunks(chunks, &existing, &handle, &mut embeddings_generated)?;
        }

        handle.join().context(EmbeddingFailedSnafu)?;

        let mut final_batch = storage_batch.lock().unwrap_or_else(|p| p.into_inner());
        let chunks_created = final_batch.len();

        if !final_batch.is_empty() {
            for batch in final_batch.chunks(batch_size) {
                self.repository
                    .save_batch(batch)
                    .context(StorageFailedSnafu)?;
            }
        }
        final_batch.clear();

        Ok(CacheResult::builder()
            .entries_scanned(entries_scanned)
            .chunks_created(chunks_created)
            .chunks_skipped(chunks_skipped)
            .embeddings_generated(embeddings_generated)
            .build())
    }
}

/// Errors that can occur during chunk caching.
#[derive(Debug, Snafu)]
#[snafu(module, visibility(pub(crate)))]
#[non_exhaustive]
pub enum CacheError<E: std::error::Error + 'static> {
    /// Failed to scan the content source for entries.
    #[snafu(display("Failed to scan content source"))]
    ScanFailed {
        /// Underlying source error.
        source: E,
    },

    /// Failed to fetch content from an entry.
    #[snafu(display("Failed to fetch content"))]
    FetchFailed {
        /// Underlying source error.
        source: E,
    },

    /// Failed to chunk content into segments.
    #[snafu(display("Failed to chunk content"))]
    ChunkingFailed {
        /// Underlying chunking error.
        source: DispatchError,
    },

    /// Failed to generate embeddings for chunks.
    #[snafu(display("Failed to generate embeddings"))]
    EmbeddingFailed {
        /// Underlying indexing error.
        source: IndexingError,
    },

    /// Failed to save chunks to storage.
    #[snafu(display("Failed to save to storage"))]
    StorageFailed {
        /// Underlying storage error.
        source: StorageError,
    },

    /// Failed to compute file content hash.
    #[snafu(display("Failed to compute file hash"))]
    HashCompute {
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to initialize the chunking dispatcher.
    #[snafu(display("Failed to initialize dispatcher"))]
    DispatcherInit {
        /// Underlying dispatch error.
        source: DispatchError,
    },
}

//! Context-free chunk caching for pre-populating the CAS chunk store
//!
//! Provides [`ChunkCacher`] for caching chunks and embeddings without context
//! association. This enables pre-warming the content-addressed store so that
//! subsequent context creation can reuse existing embeddings.

mod types;

pub use types::CacheResult;

use std::num::NonZeroUsize;
use std::sync::Arc;

use bon::Builder;
use snafu::{ResultExt, Snafu};

use super::indexer::pipeline::EmbeddingPipeline;
use super::source::ContentSource;
use super::{BatchConfig, IndexDataProvider, IndexingError, ProgressReporter};
use crate::knowledge::chunking::{ChunkingDispatcher, ChunkingInput, DispatchError};
use crate::knowledge::domain::{Chunk, ChunkHash, ChunkSource, ChunkableContent, FileHash};
use crate::knowledge::storage::{ChunkRepository, StorageError};

/// Caches chunks and embeddings without context association.
#[derive(Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct ChunkCacher<S: ContentSource, R: ChunkRepository> {
    /// Content source to read from.
    source: S,
    /// Dispatcher for chunking content by file type.
    dispatcher: ChunkingDispatcher,
    /// Repository for storing indexed chunks.
    repository: R,
    /// Provider for generating embeddings.
    provider: Arc<dyn IndexDataProvider>,
    /// Configuration for batch operations.
    #[builder(default)]
    batch_config: BatchConfig,
    /// Optional progress reporter.
    progress: Option<Arc<dyn ProgressReporter>>,
}

impl<S: ContentSource, R: ChunkRepository> ChunkCacher<S, R> {
    /// Cache all content from source, skipping existing embeddings.
    pub fn cache(&mut self) -> Result<CacheResult, CacheError<S::Error>> {
        use cache_error::*;

        let entries = self.source.scan().context(ScanFailedSnafu)?;
        let entries_scanned = entries.len();

        // Phase 1: Collect all chunks needing embeddings
        let mut all_chunks: Vec<Chunk> = Vec::new();
        let mut chunks_skipped: usize = 0;

        for entry in &entries {
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

            let chunks = match self.dispatcher.chunk_file(&input) {
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

            let new_chunks: Vec<_> = chunks
                .into_iter()
                .filter(|c| !existing.contains(&c.chunk_hash))
                .collect();

            all_chunks.extend(new_chunks);
        }

        // Phase 2: Generate embeddings via pipeline
        let indexed_chunks = if all_chunks.is_empty() {
            Vec::new()
        } else {
            let worker_count = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);

            let pipeline = EmbeddingPipeline::builder()
                .provider(Arc::clone(&self.provider))
                .worker_count(worker_count)
                .maybe_progress(self.progress.clone())
                .build();

            pipeline
                .run(all_chunks.into_iter())
                .context(EmbeddingFailedSnafu)?
        };

        // Phase 3: Store results in batches
        let chunks_created = indexed_chunks.len();
        let embeddings_generated = chunks_created;
        let batch_size = self.batch_config.batch_size.into_inner();

        for batch in indexed_chunks.chunks(batch_size) {
            self.repository
                .save_batch(batch)
                .context(StorageFailedSnafu)?;
        }

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

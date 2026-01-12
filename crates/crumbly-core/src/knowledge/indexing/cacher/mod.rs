//! Context-free chunk caching for pre-populating the CAS chunk store
//!
//! Provides [`ChunkCacher`] for caching chunks and embeddings without context
//! association. This enables pre-warming the content-addressed store so that
//! subsequent context creation can reuse existing embeddings.

mod types;

pub use types::CacheResult;

use std::sync::Arc;

use bon::Builder;
use snafu::{ResultExt, Snafu};

use super::source::ContentSource;
use super::{BatchConfig, IndexDataError, IndexDataProvider, ProgressReporter};
use crate::knowledge::chunking::{ChunkingDispatcher, ChunkingInput, DispatchError};
use crate::knowledge::domain::{
    ChunkHash, ChunkSource, ChunkableContent, FileHash, IndexedChunk, Timestamp,
};
use crate::knowledge::storage::{ChunkRepository, StorageError};

/// Caches chunks and embeddings without context association.
#[derive(Builder)]
#[builder(on(_, into))]
pub struct ChunkCacher<S: ContentSource, R: ChunkRepository> {
    source: S,
    dispatcher: ChunkingDispatcher,
    repository: R,
    provider: Box<dyn IndexDataProvider>,
    #[builder(default)]
    batch_config: BatchConfig,
    progress: Option<Arc<dyn ProgressReporter>>,
}

impl<S: ContentSource, R: ChunkRepository> ChunkCacher<S, R> {
    fn flush_batch(&mut self, batch: &mut Vec<IndexedChunk>) -> Result<(), CacheError<S::Error>> {
        use cache_error::*;
        if !batch.is_empty() {
            self.repository
                .save_batch(batch)
                .context(StorageFailedSnafu)?;
            batch.clear();
        }
        Ok(())
    }

    /// Cache all content from source, skipping existing embeddings.
    pub fn cache(&mut self) -> Result<CacheResult, CacheError<S::Error>> {
        use cache_error::*;

        let entries = self.source.scan().context(ScanFailedSnafu)?;
        let entries_scanned = entries.len();

        let mut chunks_created: usize = 0;
        let mut chunks_skipped: usize = 0;
        let mut embeddings_generated: usize = 0;
        let mut batch_buffer = Vec::with_capacity(self.batch_config.batch_size.into_inner());

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

            let new_chunks: Vec<_> = chunks
                .into_iter()
                .filter(|c| !existing.contains(&c.chunk_hash))
                .collect();

            chunks_skipped += existing.len();

            if new_chunks.is_empty() {
                continue;
            }

            let texts: Vec<_> = new_chunks.iter().map(|c| c.content.text.as_ref()).collect();
            let progress_ref = self.progress.as_ref().map(|p| p.as_ref());
            let embeddings = self
                .provider
                .generate_batch_with_progress(&texts, progress_ref)
                .context(EmbeddingFailedSnafu)?;

            embeddings_generated += embeddings.len();
            let timestamp = Timestamp::now();

            for (chunk, embedding) in new_chunks.into_iter().zip(embeddings) {
                let indexed = IndexedChunk::builder()
                    .chunk(chunk)
                    .embedding(embedding)
                    .indexed_at(timestamp)
                    .build();
                batch_buffer.push(indexed);
                chunks_created += 1;
            }

            if batch_buffer.len() >= self.batch_config.batch_size.into_inner() {
                self.flush_batch(&mut batch_buffer)?;
            }
        }

        self.flush_batch(&mut batch_buffer)?;

        Ok(CacheResult::builder()
            .entries_scanned(entries_scanned)
            .chunks_created(chunks_created)
            .chunks_skipped(chunks_skipped)
            .embeddings_generated(embeddings_generated)
            .build())
    }
}

#[derive(Debug, Snafu)]
#[snafu(module, visibility(pub))]
#[non_exhaustive]
pub enum CacheError<E: std::error::Error + 'static> {
    #[snafu(display("Failed to scan content source"))]
    ScanFailed { source: E },

    #[snafu(display("Failed to fetch content"))]
    FetchFailed { source: E },

    #[snafu(display("Failed to chunk content"))]
    ChunkingFailed { source: DispatchError },

    #[snafu(display("Failed to generate embeddings"))]
    EmbeddingFailed { source: IndexDataError },

    #[snafu(display("Failed to save to storage"))]
    StorageFailed { source: StorageError },

    #[snafu(display("Failed to compute file hash"))]
    HashCompute { source: std::io::Error },

    #[snafu(display("Failed to initialize dispatcher"))]
    DispatcherInit { source: DispatchError },
}

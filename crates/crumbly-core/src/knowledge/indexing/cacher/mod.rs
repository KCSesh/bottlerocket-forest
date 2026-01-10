//! Context-free chunk caching for pre-populating the CAS chunk store
//!
//! Provides [`ChunkCacher`] for caching chunks and embeddings without context
//! association. This enables pre-warming the content-addressed store so that
//! subsequent context creation can reuse existing embeddings.

mod types;

pub use types::{CacheError, CacheResult};

use std::sync::Arc;

use bon::Builder;
use snafu::ResultExt;

use super::source::ContentSource;
use super::{BatchConfig, IndexDataProvider, ProgressReporter};
use crate::knowledge::chunking::{ChunkingDispatcher, ChunkingInput};
use crate::knowledge::domain::{
    ChunkHash, ChunkSource, ChunkableContent, FileHash, IndexedChunk, Timestamp,
};
use crate::knowledge::storage::ChunkRepository;

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
    /// Cache all content from source, skipping existing embeddings.
    pub fn cache(&mut self) -> Result<CacheResult, CacheError<S::Error>> {
        use types::cache_error::*;

        let entries = self.source.scan().context(ScanFailedSnafu)?;
        let entries_scanned = entries.len();

        let mut chunks_created = 0;
        let mut chunks_skipped = 0;
        let mut embeddings_generated = 0;
        let mut batch_buffer = Vec::with_capacity(self.batch_config.batch_size);

        for entry in &entries {
            let content = self.source.fetch(entry).context(FetchFailedSnafu)?;
            let file_hash = FileHash::from_reader(std::io::Cursor::new(content.as_bytes()))
                .expect("in-memory read cannot fail");

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

                if batch_buffer.len() >= self.batch_config.batch_size {
                    self.repository
                        .save_batch(&batch_buffer)
                        .context(StorageFailedSnafu)?;
                    batch_buffer.clear();
                }
            }
        }

        if !batch_buffer.is_empty() {
            self.repository
                .save_batch(&batch_buffer)
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

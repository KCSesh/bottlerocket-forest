//! Batch processing for chunk indexing and storage.

use std::num::NonZeroUsize;
use std::sync::Arc;

use super::pipeline::EmbeddingPipeline;
use super::types::IndexingError;
use super::*;
use crate::knowledge::domain::{Chunk, ChunkHash, FileHash, IndexRelativePath, Timestamp};
use crate::knowledge::indexing::IndexableFile;

impl<R: ChunkRepository> Indexer<R> {
    /// Generate embeddings and store indexed chunks
    ///
    /// Uses a producer-consumer pipeline for consistent CPU utilization
    /// regardless of file sizes.
    pub(super) fn index_and_store_chunks<'a>(
        &mut self,
        results: impl Iterator<Item = (&'a IndexableFile, FileResult)>,
        modified_paths: &[&IndexRelativePath],
    ) -> Result<(usize, usize, usize), IndexingError> {
        use types::indexing_error::*;

        // Phase 1: Collect files and filter chunks that need embeddings
        let mut all_chunks: Vec<Chunk> = Vec::new();
        let mut files_skipped = 0;
        let mut files_added = 0;

        for (file, result) in results {
            let chunks = match result {
                Ok(chunks) => chunks,
                Err(Ok(())) => {
                    files_skipped += 1;
                    continue;
                }
                Err(Err(e)) => return Err(e),
            };

            files_added += 1;
            if modified_paths.contains(&&file.relative_path) {
                self.repository
                    .remove_indexed_file_from_context(&file.relative_path, &self.context_id)
                    .context(StorageFailedSnafu)?;
            }

            if chunks.is_empty() {
                continue;
            }

            // Filter to only chunks that need embeddings
            let chunk_hashes: Vec<ChunkHash> = chunks.iter().map(|c| c.chunk_hash).collect();
            let existing = self
                .repository
                .has_embedding_batch(&chunk_hashes)
                .context(StorageFailedSnafu)?;

            // Extract file_hash before filtering - chunks may all be cached
            let file_hash = chunks
                .first()
                .map(|c| c.file_hash)
                .unwrap_or(FileHash::new([0u8; 32]));

            let chunks_needing_embeddings: Vec<Chunk> = chunks
                .into_iter()
                .filter(|c| !existing.contains(&c.chunk_hash))
                .collect();

            self.repository
                .track_indexed_file(
                    &file.relative_path,
                    &file_hash,
                    Timestamp::from_secs(file.last_modified.as_secs()),
                    &self.context_id,
                )
                .context(StorageFailedSnafu)?;

            all_chunks.extend(chunks_needing_embeddings);
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

            pipeline.run(all_chunks.into_iter())?
        };

        // Phase 3: Store results
        let chunks_affected = indexed_chunks.len();
        let mut batch_buffer = Vec::with_capacity(self.batch_config.batch_size.into_inner());

        for chunk in indexed_chunks {
            batch_buffer.push(chunk);
            if batch_buffer.len() >= self.batch_config.batch_size.into_inner() {
                self.repository
                    .save_batch(&batch_buffer)
                    .context(StorageFailedSnafu)?;
                batch_buffer.clear();
            }
        }

        if !batch_buffer.is_empty() {
            self.repository
                .save_batch(&batch_buffer)
                .context(StorageFailedSnafu)?;
        }

        Ok((files_added, files_skipped, chunks_affected))
    }
}

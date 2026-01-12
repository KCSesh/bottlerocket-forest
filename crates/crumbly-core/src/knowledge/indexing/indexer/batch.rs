use super::operations;
use super::*;
use crate::knowledge::domain::{IndexRelativePath, Timestamp};
use crate::knowledge::indexing::IndexableFile;

impl<R: ChunkRepository> Indexer<R> {
    /// Generate embeddings and store indexed chunks
    ///
    /// For each file's chunks, checks which already have embeddings in the
    /// repository and only generates embeddings for new chunks. Stores
    /// results in batches.
    #[expect(clippy::excessive_nesting)]
    pub(super) fn index_and_store_chunks<'a>(
        &mut self,
        results: impl Iterator<Item = (&'a IndexableFile, FileResult)>,
        modified_paths: &[&IndexRelativePath],
    ) -> Result<(usize, usize, usize), IndexingError> {
        use types::indexing_error::*;

        let mut files_added = 0;
        let mut files_skipped = 0;
        let mut chunks_affected = 0;
        let mut batch_buffer = Vec::with_capacity(self.batch_config.batch_size.into_inner());

        for (file, result) in results {
            match result {
                Ok(chunks) => {
                    files_added += 1;
                    if modified_paths.contains(&&file.relative_path) {
                        self.repository
                            .remove_indexed_file_from_context(&file.relative_path, &self.context_id)
                            .context(StorageFailedSnafu)?;
                    }

                    if chunks.is_empty() {
                        continue;
                    }

                    let file_hash = chunks[0].file_hash;

                    let progress_ref = self.progress.as_ref().map(|p| p.as_ref());
                    let indexed_chunks = operations::index_chunks_with_reuse(
                        chunks,
                        &self.repository,
                        &*self.provider,
                        progress_ref,
                    )?;

                    self.repository
                        .track_indexed_file(
                            &file.relative_path,
                            &file_hash,
                            Timestamp::from_secs(file.last_modified.as_secs()),
                            &self.context_id,
                        )
                        .context(StorageFailedSnafu)?;

                    if indexed_chunks.is_empty() {
                        continue;
                    }

                    chunks_affected += indexed_chunks.len();

                    for chunk in indexed_chunks {
                        batch_buffer.push(chunk);
                        if batch_buffer.len() >= self.batch_config.batch_size.into_inner() {
                            self.repository
                                .save_batch(&batch_buffer)
                                .context(StorageFailedSnafu)?;
                            batch_buffer.clear();
                        }
                    }
                }
                Err(Ok(())) => {
                    files_skipped += 1;
                }
                Err(Err(e)) => {
                    return Err(e);
                }
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

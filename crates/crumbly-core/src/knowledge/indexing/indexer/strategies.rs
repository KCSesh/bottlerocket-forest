use super::streaming::stream_index_files;
use super::*;
use std::collections::HashSet;
use std::time::Instant;

use crate::knowledge::indexing::IndexableFile;

impl<R: ChunkRepository + Send + 'static> Indexer<R> {
    pub(super) fn build(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;

        let start = Instant::now();
        let files = self.scanner.scan().context(ScanFailedSnafu)?;

        if let Some(progress) = &self.progress {
            progress.chunking_started(files.len());
        }

        if files.is_empty() {
            if let Some(progress) = &self.progress {
                progress.chunking_completed(0);
                progress.embedding_started(0);
                progress.embedding_completed();
                progress.indexing_completed();
            }
            return Ok(IndexResult::builder()
                .files_processed(0)
                .files_added(0)
                .files_updated(0)
                .files_removed(0)
                .files_skipped(0)
                .chunks_affected(0)
                .duration(start.elapsed())
                .build());
        }

        let file_refs: Vec<&IndexableFile> = files.iter().collect();
        let (files_added, files_skipped, chunks_affected) = stream_index_files(
            &file_refs,
            &[],
            &self.dispatcher,
            &mut self.repository,
            &self.provider,
            &self.progress,
            &self.batch_config,
            &self.context_id,
        )?;

        if let Some(progress) = &self.progress {
            progress.embedding_completed();
            progress.indexing_completed();
        }

        Ok(IndexResult::builder()
            .files_processed(files_added)
            .files_added(files_added)
            .files_updated(0)
            .files_removed(0)
            .files_skipped(files_skipped)
            .chunks_affected(chunks_affected)
            .duration(start.elapsed())
            .build())
    }

    pub(super) fn rebuild(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;
        self.repository.clear().context(StorageFailedSnafu)?;
        self.build()
    }

    pub(super) fn incremental(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;

        let start = Instant::now();
        let current_files = self.scanner.scan().context(ScanFailedSnafu)?;
        let indexed_files = self
            .repository
            .get_indexed_files(&self.context_id)
            .context(StorageFailedSnafu)?;
        let current_paths: HashSet<_> = current_files
            .iter()
            .map(|f| f.relative_path.clone())
            .collect();
        let indexed_paths: HashSet<_> = indexed_files.keys().cloned().collect();

        let added: Vec<_> = current_files
            .iter()
            .filter(|f| !indexed_paths.contains(&f.relative_path))
            .collect();
        let modified: Vec<_> = current_files
            .iter()
            .filter(|f| {
                indexed_files
                    .get(&f.relative_path)
                    .map(|&ts| f.last_modified > ts)
                    .unwrap_or(false)
            })
            .collect();
        let deleted: Vec<_> = indexed_paths.difference(&current_paths).cloned().collect();

        for path in &deleted {
            self.repository
                .remove_indexed_file_from_context(path, &self.context_id)
                .context(StorageFailedSnafu)?;
        }

        let files_to_process: Vec<_> = added.iter().chain(modified.iter()).copied().collect();

        if files_to_process.is_empty() {
            if let Some(p) = &self.progress {
                p.chunking_started(0);
                p.chunking_completed(0);
                p.embedding_started(0);
                p.embedding_completed();
                p.indexing_completed();
            }
            return Ok(IndexResult::builder()
                .files_processed(0)
                .files_added(added.len())
                .files_updated(modified.len())
                .files_removed(deleted.len())
                .files_skipped(0)
                .chunks_affected(0)
                .duration(start.elapsed())
                .build());
        }

        if let Some(p) = &self.progress {
            p.chunking_started(files_to_process.len());
        }

        let modified_paths: Vec<_> = modified.iter().map(|f| &f.relative_path).collect();
        let (files_processed, files_skipped, chunks_affected) = stream_index_files(
            &files_to_process,
            &modified_paths,
            &self.dispatcher,
            &mut self.repository,
            &self.provider,
            &self.progress,
            &self.batch_config,
            &self.context_id,
        )?;

        if let Some(p) = &self.progress {
            p.embedding_completed();
            p.indexing_completed();
        }

        Ok(IndexResult::builder()
            .files_processed(files_processed)
            .files_added(added.len())
            .files_updated(modified.len())
            .files_removed(deleted.len())
            .files_skipped(files_skipped)
            .chunks_affected(chunks_affected)
            .duration(start.elapsed())
            .build())
    }
}

#[cfg(test)]
#[path = "strategies_test.rs"]
mod test;

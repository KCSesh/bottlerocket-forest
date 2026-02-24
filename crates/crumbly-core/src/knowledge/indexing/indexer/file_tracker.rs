//! File tracking for crash-safe indexing
//!
//! Tracks in-flight files during indexing to ensure files are only marked as indexed
//! AFTER their embeddings have been flushed to storage.

use std::collections::HashMap;

use snafu::ResultExt;

use super::types::IndexingError;
use crate::knowledge::domain::{ContextId, FileHash, IndexRelativePath, Timestamp};
use crate::knowledge::storage::ChunkRepository;

/// Registration data sent from consumer to collector for file tracking
pub(super) struct FileRegistration {
    pub(super) file_path: IndexRelativePath,
    pub(super) file_hash: FileHash,
    pub(super) mtime: Timestamp,
    pub(super) context_id: ContextId,
    pub(super) expected_chunks: usize,
}

/// Tracks in-flight files and flushes completed ones to storage
///
/// Files are registered with an expected chunk count. As chunks are processed,
/// the counter decrements. When it reaches zero, the file is ready to flush.
/// Flush happens AFTER BatchingSink flush to maintain crash-safety invariant.
pub(super) struct FileTracker<R: ChunkRepository> {
    repository: R,
    pending: HashMap<IndexRelativePath, (FileRegistration, usize)>,
    completed: Vec<FileRegistration>,
    batch_size: usize,
    error: Option<IndexingError>,
}

impl<R: ChunkRepository> FileTracker<R> {
    pub(super) fn new(repository: R, batch_size: usize) -> Self {
        Self {
            repository,
            pending: HashMap::new(),
            completed: Vec::with_capacity(batch_size),
            batch_size,
            error: None,
        }
    }

    pub(super) fn expect_file(&mut self, reg: FileRegistration) {
        if reg.expected_chunks == 0 {
            self.completed.push(reg);
        } else {
            let remaining = reg.expected_chunks;
            let path = reg.file_path.clone();
            self.pending.insert(path, (reg, remaining));
        }
    }

    pub(super) fn record_chunk(&mut self, file_path: &IndexRelativePath) {
        if let Some((reg, remaining)) = self.pending.remove(file_path) {
            let new_remaining = remaining.saturating_sub(1);
            if new_remaining == 0 {
                self.completed.push(reg);
            } else {
                self.pending.insert(file_path.clone(), (reg, new_remaining));
            }
        }
    }

    pub(super) fn maybe_flush(&mut self) {
        if self.completed.len() >= self.batch_size {
            self.flush_batch();
        }
    }

    pub(super) fn flush(&mut self) -> Result<(), IndexingError> {
        use super::types::indexing_error::*;

        if let Some(e) = self.error.take() {
            return Err(e);
        }
        for reg in self.completed.drain(..) {
            self.repository
                .track_indexed_file(&reg.file_path, &reg.file_hash, reg.mtime, &reg.context_id)
                .context(StorageFailedSnafu)?;
        }
        Ok(())
    }

    fn flush_batch(&mut self) {
        use super::types::indexing_error::*;

        for reg in self.completed.drain(..) {
            if let Err(e) = self
                .repository
                .track_indexed_file(&reg.file_path, &reg.file_hash, reg.mtime, &reg.context_id)
                .context(StorageFailedSnafu)
            {
                self.error = Some(e);
                return;
            }
        }
    }
}

//! Type definitions for indexing operations
//!
//! Defines [`IndexResult`] for reporting indexing outcomes and [`IndexingError`]
//! for representing failures during the indexing workflow.

use bon::Builder;
use snafu::Snafu;
use std::time::Duration;

use crate::knowledge::domain::BatchSize;
use crate::knowledge::storage::StorageError;

use super::super::{DispatchError, IndexDataError, ScanError};

/// Configuration for batch writing during indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchConfig {
    /// Number of chunks to accumulate before writing to storage.
    pub batch_size: BatchSize,
}

impl Default for BatchConfig {
    #[expect(clippy::expect_used)]
    fn default() -> Self {
        Self {
            batch_size: BatchSize::try_new(100).expect("100 is valid batch size"),
        }
    }
}

/// Statistics and metadata from a completed indexing operation
#[derive(Debug, Clone, PartialEq, Builder)]
#[builder(on(Duration, into))]
#[non_exhaustive]
pub struct IndexResult {
    /// Number of files processed (files_added + files_updated)
    pub files_processed: usize,

    /// Number of files added (Build/Rebuild: all files, Incremental: new files only)
    pub files_added: usize,

    /// Number of files updated (Build/Rebuild: 0, Incremental: modified files)
    pub files_updated: usize,

    /// Number of files removed (Build/Rebuild: 0, Incremental: deleted files)
    pub files_removed: usize,

    /// Number of files skipped due to parse errors
    pub files_skipped: usize,

    /// Total chunks affected (created, updated, or removed)
    pub chunks_affected: usize,

    /// Time taken
    pub duration: Duration,
}

/// Errors that can occur during indexing
#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub(crate)))]
#[non_exhaustive]
pub enum IndexingError {
    /// Failed to scan the forest directory for files.
    #[snafu(display("Failed to scan files in forest directory"))]
    #[diagnostic(
        code(crumbly::indexing::scan_failed),
        help("Check that the directory is readable and contains valid Bottlerocket repositories")
    )]
    ScanFailed {
        /// Underlying scan error.
        source: ScanError,
    },

    /// Failed to chunk a file into searchable segments.
    #[snafu(display("Failed to chunk file into searchable segments"))]
    #[diagnostic(
        code(crumbly::indexing::chunking_failed),
        help("The file may contain invalid syntax or unsupported content")
    )]
    ChunkingFailed {
        /// Underlying chunking error.
        source: DispatchError,
    },

    /// Failed to generate embeddings for chunks.
    #[snafu(display("Failed to generate search index data for chunks"))]
    #[diagnostic(
        code(crumbly::indexing::index_data_generation_failed),
        help("This may be due to embedding model issues or invalid text content")
    )]
    IndexDataGenerationFailed {
        /// Underlying embedding error.
        source: IndexDataError,
    },

    /// Failed to persist indexed chunks to storage.
    #[snafu(display("Failed to save indexed chunks to database"))]
    #[diagnostic(
        code(crumbly::indexing::storage_failed),
        help("Check available disk space and database permissions")
    )]
    StorageFailed {
        /// Underlying storage error.
        source: StorageError,
    },

    /// Failed to build rayon thread pool for embedding worker.
    #[snafu(display("Failed to create thread pool for embedding worker"))]
    #[diagnostic(
        code(crumbly::indexing::thread_pool_build_failed),
        help("This is an internal error - please report it")
    )]
    ThreadPoolBuildFailed {
        /// Underlying rayon error.
        source: rayon::ThreadPoolBuildError,
    },

    /// Pipeline collector thread panicked unexpectedly.
    #[snafu(display("Pipeline collector thread panicked"))]
    #[diagnostic(
        code(crumbly::indexing::collector_panicked),
        help("This is an internal error - please report it")
    )]
    CollectorPanicked,

    /// A worker thread panicked during parallel processing.
    #[snafu(display("Worker thread panicked during parallel processing"))]
    #[diagnostic(
        code(crumbly::indexing::thread_panic),
        help("This is an internal error - please report it")
    )]
    ThreadPanic,
}

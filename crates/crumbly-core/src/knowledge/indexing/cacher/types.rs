//! Type definitions for chunk caching operations

use bon::Builder;
use snafu::Snafu;

use crate::knowledge::chunking::DispatchError;
use crate::knowledge::indexing::IndexDataError;
use crate::knowledge::storage::StorageError;

/// Statistics from a completed caching operation
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[non_exhaustive]
pub struct CacheResult {
    pub entries_scanned: usize,
    pub chunks_created: usize,
    pub chunks_skipped: usize,
    pub embeddings_generated: usize,
}

/// Errors that can occur during chunk caching
#[derive(Debug, Snafu)]
#[snafu(module, visibility(pub(crate)))]
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

    #[snafu(display("Failed to initialize dispatcher"))]
    DispatcherInit { source: DispatchError },
}

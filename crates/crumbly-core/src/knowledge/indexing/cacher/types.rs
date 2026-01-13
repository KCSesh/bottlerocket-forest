//! Type definitions for chunk caching operations

use bon::Builder;

/// Statistics from a completed caching operation.
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct CacheResult {
    /// Number of content entries scanned.
    entries_scanned: usize,
    /// Number of new chunks created.
    chunks_created: usize,
    /// Number of chunks skipped (already cached).
    chunks_skipped: usize,
    /// Number of embeddings generated.
    embeddings_generated: usize,
}

impl CacheResult {
    /// Returns the number of entries scanned.
    pub fn entries_scanned(&self) -> usize {
        self.entries_scanned
    }
    /// Returns the number of chunks created.
    pub fn chunks_created(&self) -> usize {
        self.chunks_created
    }
    /// Returns the number of chunks skipped.
    pub fn chunks_skipped(&self) -> usize {
        self.chunks_skipped
    }
    /// Returns the number of embeddings generated.
    pub fn embeddings_generated(&self) -> usize {
        self.embeddings_generated
    }
}

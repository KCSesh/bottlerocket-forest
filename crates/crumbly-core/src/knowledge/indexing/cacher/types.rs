//! Type definitions for chunk caching operations

use bon::Builder;

/// Statistics from a completed caching operation
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct CacheResult {
    entries_scanned: usize,
    chunks_created: usize,
    chunks_skipped: usize,
    embeddings_generated: usize,
}

impl CacheResult {
    pub fn entries_scanned(&self) -> usize {
        self.entries_scanned
    }
    pub fn chunks_created(&self) -> usize {
        self.chunks_created
    }
    pub fn chunks_skipped(&self) -> usize {
        self.chunks_skipped
    }
    pub fn embeddings_generated(&self) -> usize {
        self.embeddings_generated
    }
}

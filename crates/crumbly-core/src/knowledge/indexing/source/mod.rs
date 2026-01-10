//! Content source abstraction for indexing backends
//!
//! Provides a unified interface for discovering and fetching content from
//! various backends (filesystem, bare git repositories, etc.).
//!
//! # Architecture
//!
//! * [`ContentSource`] - Trait for content backends to implement
//! * [`ContentEntry`] - Metadata for discovered content items

use bon::Builder;

use crate::knowledge::domain::{FileType, IndexRelativePath, RepoName};

/// Provides content for indexing from various backends.
pub trait ContentSource: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    type EntryId: Clone + Send + Sync + std::fmt::Debug;

    fn scan(&self) -> Result<Vec<ContentEntry<Self::EntryId>>, Self::Error>;
    fn fetch(&self, entry: &ContentEntry<Self::EntryId>) -> Result<String, Self::Error>;
}

/// Metadata for content discovered by a source.
#[derive(Debug, Clone, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct ContentEntry<Id> {
    pub id: Id,
    pub relative_path: IndexRelativePath,
    pub repo_name: RepoName,
    pub file_type: FileType,
}

//! Content source abstraction for indexing backends
//!
//! Provides a unified interface for discovering and fetching content from
//! various backends (filesystem, bare git repositories, etc.).
//!
//! # Architecture
//!
//! * [`ContentSource`] - Trait for content backends to implement
//! * [`ContentEntry`] - Metadata for discovered content items
//! * [`FilesystemSource`] - Local filesystem backend wrapping FileScanner
//! * [`BareGitSource`] - Bare git repository backend

mod bare_git;
mod filesystem;

pub use bare_git::{BareGitSource, BareGitSourceError, GitBlobRef, GitRev};
pub use filesystem::{FilesystemSource, FilesystemSourceError};

use bon::Builder;

use crate::knowledge::domain::{FileType, IndexRelativePath, RepoName};

/// Provides content for indexing from various backends.
pub trait ContentSource: Send + Sync {
    /// Error type for this source.
    type Error: std::error::Error + Send + Sync + 'static;
    /// Identifier type for content entries.
    type EntryId: Clone + Send + Sync + std::fmt::Debug;

    /// Discover all indexable content entries.
    fn scan(&self) -> Result<Vec<ContentEntry<Self::EntryId>>, Self::Error>;
    /// Fetch the content of a specific entry.
    fn fetch(&self, entry: &ContentEntry<Self::EntryId>) -> Result<String, Self::Error>;
}

/// Metadata for content discovered by a source.
#[derive(Debug, Clone, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct ContentEntry<Id> {
    /// Source-specific identifier for fetching content.
    pub id: Id,
    /// Path relative to the index root.
    pub relative_path: IndexRelativePath,
    /// Name of the repository containing this content.
    pub repo_name: RepoName,
    /// Detected file type.
    pub file_type: FileType,
}

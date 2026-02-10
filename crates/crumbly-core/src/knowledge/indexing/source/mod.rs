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

use std::path::PathBuf;
use std::time::SystemTime;

use bon::Builder;
use snafu::Snafu;

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

    /// Fetch content for multiple entries in a batch.
    ///
    /// Default implementation calls `fetch` for each entry individually.
    fn batch_fetch(
        &self,
        entries: &[ContentEntry<Self::EntryId>],
    ) -> Vec<FetchResult<Self::EntryId>>
    where
        Self::EntryId: Clone,
    {
        entries
            .iter()
            .map(|entry| {
                self.fetch(entry)
                    .map(|content| (entry.clone(), content))
                    .map_err(|_| FetchError::Io {
                        path: PathBuf::from(entry.relative_path.to_string()),
                        source: std::io::Error::other("fetch failed"),
                    })
            })
            .collect()
    }
}

/// Metadata for content discovered by a source.
#[derive(Debug, Clone, Builder)]
#[non_exhaustive]
pub struct ContentEntry<Id> {
    /// Source-specific identifier for fetching content.
    pub id: Id,
    /// Path relative to the index root.
    #[builder(into)]
    pub relative_path: IndexRelativePath,
    /// Name of the repository containing this content.
    #[builder(into)]
    pub repo_name: RepoName,
    /// Detected file type.
    pub file_type: FileType,
    /// Last modification time, if available.
    pub last_modified: Option<SystemTime>,
}

/// Error fetching content for a single entry.
#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum FetchError {
    /// File not found in source.
    #[snafu(display("file not found: {}", path.display()))]
    NotFound {
        /// Path to the missing file.
        path: PathBuf,
    },

    /// Content is not valid UTF-8.
    #[snafu(display("invalid UTF-8 in {}", path.display()))]
    InvalidUtf8 {
        /// Path to the file with invalid encoding.
        path: PathBuf,
    },

    /// I/O error reading content.
    #[snafu(display("I/O error reading {}", path.display()))]
    Io {
        /// Path to the file.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

/// Result of fetching a single content entry.
pub type FetchResult<Id> = Result<(ContentEntry<Id>, String), FetchError>;

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock source for testing the default batch_fetch implementation.
    struct MockSource {
        fetch_count: AtomicUsize,
    }

    impl MockSource {
        fn new() -> Self {
            Self {
                fetch_count: AtomicUsize::new(0),
            }
        }
    }

    #[derive(Debug)]
    struct MockError;

    impl std::fmt::Display for MockError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "mock error")
        }
    }

    impl std::error::Error for MockError {}

    impl ContentSource for MockSource {
        type Error = MockError;
        type EntryId = String;

        fn scan(&self) -> Result<Vec<ContentEntry<Self::EntryId>>, Self::Error> {
            Ok(vec![])
        }

        fn fetch(&self, entry: &ContentEntry<Self::EntryId>) -> Result<String, Self::Error> {
            self.fetch_count.fetch_add(1, Ordering::SeqCst);
            Ok(format!("content for {}", entry.id))
        }
    }

    fn make_entry(id: &str) -> ContentEntry<String> {
        ContentEntry::builder()
            .id(id.to_string())
            .relative_path(IndexRelativePath::try_new(id.to_string()).unwrap())
            .repo_name(RepoName::try_new("test-repo".to_string()).unwrap())
            .file_type(FileType::Markdown)
            .build()
    }

    #[test]
    fn test_batch_fetch_default_impl() {
        // Given a source with the default batch_fetch implementation
        let source = MockSource::new();
        let entries = vec![make_entry("a.md"), make_entry("b.md"), make_entry("c.md")];

        // When batch_fetch is called
        let results = source.batch_fetch(&entries);

        // Then it should return results for all entries
        assert_eq!(results.len(), 3);

        // And each result should contain the entry and its content
        for (i, result) in results.into_iter().enumerate() {
            let (entry, content) = result.expect("should succeed");
            assert_eq!(entry.id, entries[i].id);
            assert_eq!(content, format!("content for {}", entries[i].id));
        }

        // And fetch should have been called for each entry
        assert_eq!(source.fetch_count.load(Ordering::SeqCst), 3);
    }
}

//! Filesystem-backed content source
//!
//! Wraps [`FileScanner`] to provide content from the local filesystem.

use std::fs;
use std::time::{Duration, UNIX_EPOCH};

use snafu::{ResultExt, Snafu};

use super::{ContentEntry, ContentSource};
use crate::knowledge::domain::AbsolutePath;
use crate::knowledge::indexing::scanner::{FileScanner, IndexableFile, ScanError};

/// Content source backed by the local filesystem.
#[derive(Debug)]
pub struct FilesystemSource {
    /// Scanner for discovering files.
    scanner: FileScanner,
}

impl FilesystemSource {
    /// Create a new filesystem source wrapping the given scanner.
    pub fn new(scanner: FileScanner) -> Self {
        Self { scanner }
    }
}

fn to_content_entry(file: IndexableFile) -> ContentEntry<AbsolutePath> {
    let last_modified =
        UNIX_EPOCH.checked_add(Duration::from_secs(file.last_modified.as_secs() as u64));
    ContentEntry::builder()
        .id(file.absolute_path)
        .relative_path(file.relative_path)
        .repo_name(file.repo_name)
        .file_type(file.file_type)
        .maybe_last_modified(last_modified)
        .build()
}

impl ContentSource for FilesystemSource {
    type Error = FilesystemSourceError;
    type EntryId = AbsolutePath;

    fn scan(&self) -> Result<Vec<ContentEntry<Self::EntryId>>, Self::Error> {
        use filesystem_source_error::*;
        let files = self.scanner.scan().context(ScanSnafu)?;
        Ok(files.into_iter().map(to_content_entry).collect())
    }

    fn fetch(&self, entry: &ContentEntry<Self::EntryId>) -> Result<String, Self::Error> {
        use filesystem_source_error::*;
        let path = entry.id.to_string();
        fs::read_to_string(&path).context(ReadFileSnafu { path })
    }
}

/// Errors from filesystem content source operations.
#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum FilesystemSourceError {
    /// Failed to scan the filesystem for files.
    #[snafu(display("Failed to scan filesystem"))]
    Scan {
        /// Underlying scan error.
        source: ScanError,
    },

    /// Failed to read file contents.
    #[snafu(display("Failed to read file: {path}"))]
    ReadFile {
        /// Path to the file.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::indexing::scanner::FileScanner;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_indexer_with_filesystem_source() {
        // Given a directory with markdown files
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        fs::write(&file_path, "# Hello World").unwrap();

        // And a FilesystemSource wrapping a scanner for that directory
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let source = FilesystemSource::new(scanner);

        // When we scan for content entries
        let entries = source.scan().unwrap();

        // Then we should find the markdown file
        assert_eq!(entries.len(), 1);
        assert!(entries[0].relative_path.to_string().ends_with("test.md"));

        // And when we fetch its content
        let content = source.fetch(&entries[0]).unwrap();

        // Then we should get the file contents
        assert_eq!(content, "# Hello World");
    }
}

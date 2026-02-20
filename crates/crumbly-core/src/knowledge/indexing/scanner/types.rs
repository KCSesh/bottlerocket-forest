//! Types for file scanning

use std::path::PathBuf;

use crate::knowledge::domain::{
    AbsolutePath, FilePeek, FileType, IndexRelativePath, RepoName, Timestamp,
};
use bon::Builder;
use snafu::Snafu;

/// Controls which files are scanned during indexing.
#[derive(Debug, Clone, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct ScanConfig {
    /// Whether to respect .gitignore rules.
    #[builder(default = true)]
    pub respect_gitignore: bool,

    /// Whether to use .crumblyignore rules.
    #[builder(default = true)]
    pub use_crumblyignore: bool,

    /// Empty means scan entire forest root; non-empty restricts to specified paths.
    #[builder(default)]
    pub targets: Vec<PathBuf>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            use_crumblyignore: true,
            targets: Vec::new(),
        }
    }
}

/// Metadata for a file discovered during scanning
#[derive(Debug, Clone, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct IndexableFile {
    /// Full path to the file on disk.
    pub absolute_path: AbsolutePath,
    /// Path relative to the index root.
    pub relative_path: IndexRelativePath,
    /// Name of the repository containing this file.
    pub repo_name: RepoName,
    /// Detected file type for chunking dispatch.
    pub file_type: FileType,
    /// Last modification timestamp.
    pub last_modified: Timestamp,
    /// Content-based file identification.
    pub file_peek: FilePeek,
}

/// Errors that can occur during file scanning
#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub))]
#[non_exhaustive]
pub enum ScanError {
    /// The specified index root path does not exist.
    #[snafu(display("Forest root not found: {path}"))]
    #[diagnostic(
        code(crumbly::scanner::index_root_not_found),
        help("Ensure the path exists and is accessible")
    )]
    IndexRootNotFound {
        /// Path that was not found.
        path: String,
    },

    /// The index root exists but is not a directory.
    #[snafu(display("Forest root is not a directory: {path}"))]
    #[diagnostic(
        code(crumbly::scanner::index_root_not_directory),
        help("The forest root must be a directory, not a file")
    )]
    IndexRootNotDirectory {
        /// Path that is not a directory.
        path: String,
    },

    /// Error occurred while traversing the directory tree.
    #[snafu(display("Error walking directory tree"))]
    #[diagnostic(
        code(crumbly::scanner::walk_error),
        help("Check file permissions and filesystem health")
    )]
    Walk {
        /// Underlying walk error.
        source: ignore::Error,
    },

    /// Insufficient permissions to access a path.
    #[snafu(display("Permission denied: {path}"))]
    #[diagnostic(
        code(crumbly::scanner::permission_denied),
        help("Check file and directory permissions")
    )]
    PermissionDenied {
        /// Path with denied access.
        path: String,
    },

    /// I/O error while scanning a specific path.
    #[snafu(display("I/O error scanning {path}"))]
    #[diagnostic(
        code(crumbly::scanner::io_error),
        help("Check that the path is accessible and the filesystem is healthy")
    )]
    IoError {
        /// Path where the error occurred.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Path does not follow expected directory structure.
    #[snafu(display("Invalid path structure: {path}"))]
    #[diagnostic(
        code(crumbly::scanner::invalid_path_structure),
        help("Ensure the path follows the expected forest directory structure")
    )]
    InvalidPathStructure {
        /// Invalid path.
        path: String,
    },

    /// Failed to construct a domain path type.
    #[snafu(display("Failed to create path type"))]
    #[diagnostic(
        code(crumbly::scanner::path_creation_failed),
        help("The path may contain invalid characters or exceed length limits")
    )]
    PathCreation {
        /// Underlying path creation error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

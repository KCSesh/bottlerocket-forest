//! Types for file scanning

use crate::knowledge::domain::{AbsolutePath, FileType, ForestRelativePath, RepoName, Timestamp};
use bon::Builder;
use snafu::Snafu;

/// Metadata for a file discovered during scanning
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct IndexableFile {
    pub absolute_path: AbsolutePath,
    pub relative_path: ForestRelativePath,
    pub repo_name: RepoName,
    pub file_type: FileType,
    pub last_modified: Timestamp,
}

/// Errors that can occur during file scanning
#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub))]
#[non_exhaustive]
pub enum ScanError {
    #[snafu(display("Forest root not found: {path}"))]
    #[diagnostic(
        code(sembly::scanner::forest_root_not_found),
        help("Ensure the path exists and is accessible")
    )]
    ForestRootNotFound { path: String },

    #[snafu(display("Forest root is not a directory: {path}"))]
    #[diagnostic(
        code(sembly::scanner::forest_root_not_directory),
        help("The forest root must be a directory, not a file")
    )]
    ForestRootNotDirectory { path: String },

    #[snafu(display("Error walking directory tree"))]
    #[diagnostic(
        code(sembly::scanner::walk_error),
        help("Check file permissions and filesystem health")
    )]
    Walk { source: ignore::Error },

    #[snafu(display("Permission denied: {path}"))]
    #[diagnostic(
        code(sembly::scanner::permission_denied),
        help("Check file and directory permissions")
    )]
    PermissionDenied { path: String },

    #[snafu(display("I/O error scanning {path}"))]
    #[diagnostic(
        code(sembly::scanner::io_error),
        help("Check that the path is accessible and the filesystem is healthy")
    )]
    IoError {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("Invalid path structure: {path}"))]
    #[diagnostic(
        code(sembly::scanner::invalid_path_structure),
        help("Ensure the path follows the expected forest directory structure")
    )]
    InvalidPathStructure { path: String },

    #[snafu(display("Failed to create path type"))]
    #[diagnostic(
        code(sembly::scanner::path_creation_failed),
        help("The path may contain invalid characters or exceed length limits")
    )]
    PathCreation {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

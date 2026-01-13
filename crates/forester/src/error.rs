//! Error types for forester.

use miette::Diagnostic;
use snafu::Snafu;
use std::path::PathBuf;

/// Errors that can occur during forest operations.
#[derive(Debug, Snafu, Diagnostic)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to read configuration file.
    #[snafu(display("Failed to read config from {}", path.display()))]
    ConfigRead {
        /// Path to the config file.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to parse configuration file.
    #[snafu(display("Failed to parse config from {}", path.display()))]
    ConfigParse {
        /// Path to the config file.
        path: PathBuf,
        /// Underlying parse error.
        source: toml::de::Error,
    },

    /// Failed to determine current working directory.
    #[snafu(display("Could not determine current directory"))]
    CurrentDir {
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// No forest configuration found in directory hierarchy.
    #[snafu(display("No forester.toml found in current directory or any parent"))]
    #[diagnostic(help("Create a forester.toml or run from within a forest"))]
    NoForestFound,

    /// Git command execution failed.
    #[snafu(display("Git command failed: {}", message))]
    Git {
        /// Error message from git.
        message: String,
    },

    /// Failed to create directory.
    #[snafu(display("Failed to create directory {}", path.display()))]
    CreateDir {
        /// Path that could not be created.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Grove with the specified name already exists.
    #[snafu(display("Grove '{}' already exists", name))]
    GroveExists {
        /// Name of the existing grove.
        name: String,
    },

    /// Grove with the specified name does not exist.
    #[snafu(display("Grove '{}' not found", name))]
    GroveNotFound {
        /// Name of the missing grove.
        name: String,
    },

    /// Failed to create symbolic link.
    #[snafu(display("Failed to create symlink from {} to {}", src.display(), tgt.display()))]
    Symlink {
        /// Source path for the symlink.
        src: PathBuf,
        /// Target path for the symlink.
        tgt: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

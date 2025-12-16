//! Error types for forester.

use miette::Diagnostic;
use snafu::Snafu;
use std::path::PathBuf;

#[derive(Debug, Snafu, Diagnostic)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to read config from {}", path.display()))]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Failed to parse config from {}", path.display()))]
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[snafu(display("Could not determine current directory"))]
    CurrentDir { source: std::io::Error },

    #[snafu(display("No forester.toml found in current directory or any parent"))]
    #[diagnostic(help("Create a forester.toml or run from within a forest"))]
    NoForestFound,

    #[snafu(display("Git command failed: {}", message))]
    Git { message: String },

    #[snafu(display("Failed to create directory {}", path.display()))]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Worktree '{}' already exists", name))]
    WorktreeExists { name: String },

    #[snafu(display("Worktree '{}' not found", name))]
    WorktreeNotFound { name: String },

    #[snafu(display("Failed to create symlink from {} to {}", src.display(), tgt.display()))]
    Symlink {
        src: PathBuf,
        tgt: PathBuf,
        source: std::io::Error,
    },
}

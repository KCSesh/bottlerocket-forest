//! Cache command for pre-populating the chunk store.
//!
//! Provides CLI commands for caching chunks and embeddings from various content sources
//! without context association.

mod handlers;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use handlers::handle_cache;

/// Arguments for caching content.
#[derive(Parser)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub source: SourceBackend,

    /// Path to index root (defaults to current directory)
    #[arg(long, global = true)]
    pub index_root: Option<PathBuf>,
}

/// Content source backend selection.
#[derive(Subcommand)]
pub enum SourceBackend {
    /// Cache from filesystem directory
    Filesystem(FilesystemArgs),
    /// Cache from bare git repositories
    BareGit(BareGitArgs),
}

/// Arguments for filesystem source.
#[derive(Parser)]
pub struct FilesystemArgs {
    /// Path to scan for content
    pub path: PathBuf,
}

/// Arguments for bare git source.
#[derive(Parser)]
pub struct BareGitArgs {
    /// Directory containing bare git repos (e.g., .forest/bare/)
    pub bare_repos_dir: PathBuf,

    /// Git revision to index (default: HEAD)
    #[arg(long, default_value = "HEAD")]
    pub rev: String,
}

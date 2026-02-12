//! Build-cache command for creating database and warming chunk cache.

mod handlers;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use handlers::{handle_build_cache, handle_update_cache};

#[derive(Parser)]
pub struct BuildCacheArgs {
    #[command(subcommand)]
    pub source: SourceBackend,

    #[arg(long, global = true)]
    pub index_root: Option<PathBuf>,
}

#[derive(Parser)]
pub struct UpdateCacheArgs {
    #[command(subcommand)]
    pub source: SourceBackend,

    #[arg(long, global = true)]
    pub index_root: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum SourceBackend {
    Filesystem(FilesystemArgs),
    BareGit(BareGitArgs),
}

#[derive(Parser)]
pub struct FilesystemArgs {
    pub path: PathBuf,
}

#[derive(Parser)]
pub struct BareGitArgs {
    pub bare_repos_dir: PathBuf,
    #[arg(long, default_value = "HEAD")]
    pub rev: String,
}

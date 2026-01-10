//! Knowledge index management commands.
//!
//! This module provides CLI commands for building, updating, and querying the knowledge index.
//! The index enables semantic search across Bottlerocket repositories.
//!
//! Submodules:
//! * [`progress`] - Progress reporting for indexing operations

use clap::Parser;
use std::path::PathBuf;

pub mod progress;

/// Arguments for building the knowledge index.
#[derive(Parser)]
pub struct BuildArgs {
    /// Path to index root (defaults to current directory)
    #[arg(long)]
    index_root: Option<PathBuf>,

    /// Context path to operate on (defaults to workspace root)
    #[arg(long)]
    context: Option<PathBuf>,
}

/// Arguments for rebuilding the knowledge index from scratch.
#[derive(Parser)]
pub struct RebuildArgs {
    /// Path to index root (defaults to current directory)
    #[arg(long)]
    index_root: Option<PathBuf>,

    /// Context path to operate on (defaults to workspace root)
    #[arg(long)]
    context: Option<PathBuf>,
}

/// Arguments for incrementally updating the knowledge index.
#[derive(Parser)]
pub struct UpdateArgs {
    /// Path to index root (defaults to current directory)
    #[arg(long)]
    index_root: Option<PathBuf>,

    /// Context path to operate on (defaults to workspace root)
    #[arg(long)]
    context: Option<PathBuf>,
}

/// Arguments for clearing all chunks from the index.
#[derive(Parser)]
pub struct ClearArgs {
    /// Path to index root (defaults to current directory)
    #[arg(long)]
    index_root: Option<PathBuf>,

    /// Context path to clear (defaults to current context)
    #[arg(long)]
    context: Option<PathBuf>,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    yes: bool,
}

/// Arguments for searching the knowledge index.
#[derive(Parser)]
pub struct SearchArgs {
    /// Search query
    query: String,

    /// Path to index root (defaults to current directory)
    #[arg(long)]
    index_root: Option<PathBuf>,

    /// Context path to search within (defaults to all contexts)
    #[arg(long)]
    context: Option<PathBuf>,

    /// Maximum number of results (1-100, defaults to 10)
    #[arg(short = 'n', long)]
    limit: Option<usize>,

    /// Output format: human or json (defaults to human)
    #[arg(short = 'f', long)]
    format: Option<String>,

    /// Show individual chunk matches under each file
    #[arg(long)]
    show_chunks: bool,
}

/// Arguments for showing index status and statistics.
#[derive(Parser)]
pub struct StatusArgs {
    /// Path to index root (defaults to current directory)
    #[arg(long)]
    index_root: Option<PathBuf>,
}

mod errors;
mod formatting;
mod handlers;

pub use handlers::{
    handle_build, handle_clear, handle_rebuild, handle_search, handle_status, handle_update,
};

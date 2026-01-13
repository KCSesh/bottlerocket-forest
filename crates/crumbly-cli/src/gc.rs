//! Garbage collection command for the knowledge index.
//!
//! Removes orphaned chunks not referenced by any context's indexed files.

use clap::Parser;
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

#[allow(unused_imports)] // Used in format_gc_stats implementation
use crate::theme;
use crumbly_core::knowledge::KnowledgeIndex;
use crumbly_core::knowledge::facade::GcStats;

/// Arguments for garbage collection.
#[derive(Parser)]
pub struct GcArgs {
    /// Path to index root (defaults to current directory)
    #[arg(long)]
    index_root: Option<PathBuf>,
}

/// Runs garbage collection to remove orphaned chunks.
#[expect(clippy::expect_used)]
pub fn handle_gc(args: GcArgs) -> Result<(), GcError> {
    use gc_error::*;

    let index_root = args.index_root.unwrap_or_else(|| {
        std::env::current_dir().expect("current directory should be accessible")
    });

    let index = KnowledgeIndex::open(&index_root).context(KnowledgeIndexSnafu)?;

    let stats = index.gc().context(KnowledgeIndexSnafu)?;

    format_gc_stats(&stats);

    Ok(())
}

/// Prints a formatted summary of garbage collection results.
fn format_gc_stats(stats: &GcStats) {
    println!(
        "{}",
        theme::success(format!(
            "Garbage collection complete: {} chunks deleted, {} embeddings deleted",
            stats.chunks_deleted, stats.embeddings_deleted
        ))
    );
}

#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module)]
pub enum GcError {
    #[snafu(display("Knowledge index operation failed"))]
    #[diagnostic(
        code(crumbly::cli::gc_failed),
        help("Check the error details above for specific guidance")
    )]
    KnowledgeIndex {
        source: crumbly_core::knowledge::facade::IndexError,
    },
}

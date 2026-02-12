use miette::Diagnostic;
use snafu::{ResultExt, Snafu};
use std::sync::Arc;

use crumbly_core::knowledge::ProgressReporter;
use crumbly_core::knowledge::{CacheSource, GitRev, KnowledgeIndex};

use super::{BareGitArgs, BuildCacheArgs, FilesystemArgs, SourceBackend, UpdateCacheArgs};
use crate::index::progress::CliProgressReporter;
use crate::theme;

pub fn handle_build_cache(args: BuildCacheArgs) -> Result<(), BuildCacheError> {
    use build_cache_error::*;

    let index_root = match args.index_root {
        Some(p) => p,
        None => std::env::current_dir().context(CurrentDirSnafu)?,
    };

    let index = KnowledgeIndex::open(&index_root).context(IndexOpenSnafu)?;
    index.build_cache().context(BuildCacheSnafu)?;

    let source = to_cache_source(args.source)?;
    let progress: Arc<dyn ProgressReporter> = Arc::new(CliProgressReporter::default());

    let result = index
        .cache()
        .source(source)
        .progress(progress)
        .call()
        .context(CacheSnafu)?;

    format_cache_result(&result);
    Ok(())
}

pub fn handle_update_cache(args: UpdateCacheArgs) -> Result<(), BuildCacheError> {
    use build_cache_error::*;

    let index_root = match args.index_root {
        Some(p) => p,
        None => std::env::current_dir().context(CurrentDirSnafu)?,
    };

    let index = KnowledgeIndex::open(&index_root).context(IndexOpenSnafu)?;
    index.ensure_cache().context(BuildCacheSnafu)?;

    let source = to_cache_source(args.source)?;
    let progress: Arc<dyn ProgressReporter> = Arc::new(CliProgressReporter::default());

    let result = index
        .cache()
        .source(source)
        .progress(progress)
        .call()
        .context(CacheSnafu)?;

    format_cache_result(&result);
    Ok(())
}

fn to_cache_source(backend: SourceBackend) -> Result<CacheSource, BuildCacheError> {
    match backend {
        SourceBackend::Filesystem(FilesystemArgs { path }) => Ok(CacheSource::Filesystem(path)),
        SourceBackend::BareGit(BareGitArgs {
            bare_repos_dir,
            rev,
        }) => {
            let git_rev = GitRev::try_new(&rev)
                .map_err(|_| BuildCacheError::InvalidRevision { rev: rev.clone() })?;
            Ok(CacheSource::BareGit {
                bare_repos_dir,
                rev: git_rev,
            })
        }
    }
}

fn format_cache_result(result: &crumbly_core::knowledge::CacheResult) {
    println!(
        "{} {}",
        theme::label("Entries scanned:"),
        theme::value(result.entries_scanned())
    );
    println!(
        "{} {}",
        theme::label("Chunks created:"),
        theme::value(result.chunks_created())
    );
    println!(
        "{} {}",
        theme::label("Chunks skipped:"),
        theme::value(result.chunks_skipped())
    );
    println!(
        "{} {}",
        theme::label("Embeddings generated:"),
        theme::value(result.embeddings_generated())
    );
}

#[derive(Debug, Snafu, Diagnostic)]
#[snafu(module, visibility(pub(crate)))]
pub enum BuildCacheError {
    #[snafu(display("Failed to get current directory"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::current_dir),
        help("Ensure you have permission to access the current directory")
    )]
    CurrentDir { source: std::io::Error },

    #[snafu(display("Failed to open knowledge index"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::index_open),
        forward(source),
        help("Check that the index root path is valid")
    )]
    IndexOpen {
        source: crumbly_core::knowledge::IndexError,
    },

    #[snafu(display("Failed to create database and metadata"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::build_cache),
        forward(source),
        help("Ensure database does not already exist")
    )]
    BuildCache {
        source: crumbly_core::knowledge::IndexError,
    },

    #[snafu(display("Cache operation failed"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::cache),
        forward(source),
        help("Check source path and index permissions")
    )]
    Cache {
        source: crumbly_core::knowledge::IndexError,
    },

    #[snafu(display("Invalid git revision: {rev}"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::invalid_revision),
        help("Revision must be a non-empty string (branch, tag, or commit SHA)")
    )]
    InvalidRevision { rev: String },
}

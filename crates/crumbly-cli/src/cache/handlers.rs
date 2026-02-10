use miette::Diagnostic;
use snafu::{ResultExt, Snafu};
use std::sync::Arc;

use crumbly_core::knowledge::KnowledgeIndex;
use crumbly_core::knowledge::chunking::{ChunkingDispatcher, DispatchError};
use crumbly_core::knowledge::indexing::load_crumbly_config;
use crumbly_core::knowledge::indexing::source::FilesystemSource;
use crumbly_core::knowledge::indexing::{
    BareGitSource, CacheResult, ChunkCacher, FileScanner, GitRev, ProgressReporter,
};
use crumbly_core::knowledge::search::embeddings::PooledEmbeddingProvider;
use crumbly_core::knowledge::storage::sqlite::SqliteChunkRepository;

use super::{BareGitArgs, CacheArgs, FilesystemArgs, SourceBackend};
use crate::index::progress::CliProgressReporter;
use crate::theme;

pub fn handle_cache(args: CacheArgs) -> Result<(), CacheError> {
    use cache_error::*;

    let index_root = match args.index_root {
        Some(p) => p,
        None => std::env::current_dir().context(cache_error::CurrentDirSnafu)?,
    };

    let index = KnowledgeIndex::open(&index_root).context(IndexOpenSnafu)?;

    let result = match args.source {
        SourceBackend::Filesystem(fs_args) => cache_filesystem(&index, fs_args)?,
        SourceBackend::BareGit(git_args) => cache_bare_git(&index, git_args)?,
    };

    format_cache_result(&result);
    Ok(())
}

fn cache_filesystem(
    index: &KnowledgeIndex,
    args: FilesystemArgs,
) -> Result<CacheResult, CacheError> {
    use cache_error::*;

    let scanner = FileScanner::new(&args.path).context(ScannerCreateSnafu)?;
    let source = FilesystemSource::new(scanner);
    let dispatcher =
        ChunkingDispatcher::with_defaults(index.config()).context(DispatcherCreateSnafu)?;
    let repository =
        SqliteChunkRepository::open(index.db_path(), index.config()).context(RepositorySnafu)?;
    let provider = create_provider(index)?;
    let progress: Arc<dyn ProgressReporter> = Arc::new(CliProgressReporter::default());

    let mut cacher: ChunkCacher<FilesystemSource, SqliteChunkRepository> = ChunkCacher::builder()
        .source(source)
        .dispatcher(dispatcher)
        .repository(repository)
        .provider(provider)
        .progress(progress)
        .build();

    cacher.cache().map_err(|e| CacheError::FilesystemCache {
        source: Box::new(e),
    })
}

fn cache_bare_git(index: &KnowledgeIndex, args: BareGitArgs) -> Result<CacheResult, CacheError> {
    use cache_error::*;

    let rev = GitRev::try_new(&args.rev).map_err(|_| CacheError::InvalidRevision {
        rev: args.rev.clone(),
    })?;

    let filter = load_filter(index)?;
    let source = BareGitSource::new(&args.bare_repos_dir, rev, filter);
    let dispatcher =
        ChunkingDispatcher::with_defaults(index.config()).context(DispatcherCreateSnafu)?;
    let repository =
        SqliteChunkRepository::open(index.db_path(), index.config()).context(RepositorySnafu)?;
    let provider = create_provider(index)?;
    let progress: Arc<dyn ProgressReporter> = Arc::new(CliProgressReporter::default());

    let mut cacher: ChunkCacher<BareGitSource, SqliteChunkRepository> = ChunkCacher::builder()
        .source(source)
        .dispatcher(dispatcher)
        .repository(repository)
        .provider(provider)
        .progress(progress)
        .build();

    cacher.cache().map_err(|e| CacheError::BareGitCache {
        source: Box::new(e),
    })
}

fn create_provider(
    _index: &KnowledgeIndex,
) -> Result<Box<dyn crumbly_core::knowledge::indexing::IndexDataProvider>, CacheError> {
    use cache_error::*;
    Ok(Box::new(
        PooledEmbeddingProvider::new()
            .map_err(Box::new)
            .context(PoolCreationSnafu)?,
    ))
}

fn load_filter(
    index: &KnowledgeIndex,
) -> Result<crumbly_core::knowledge::indexing::IndexingFilter, CacheError> {
    use cache_error::*;

    let config = load_crumbly_config(index.index_root()).context(ConfigLoadSnafu)?;
    let filter = config
        .map(|c| c.to_indexing_filter())
        .transpose()
        .context(ConfigLoadSnafu)?
        .unwrap_or_default();
    Ok(filter)
}

fn format_cache_result(result: &CacheResult) {
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
pub enum CacheError {
    #[snafu(display("Failed to get current directory"))]
    #[diagnostic(
        code(crumbly::cli::cache::current_dir),
        help("Ensure you have permission to access the current directory")
    )]
    CurrentDir { source: std::io::Error },

    #[snafu(display("Failed to open knowledge index"))]
    #[diagnostic(
        code(crumbly::cli::cache::index_open),
        forward(source),
        help("Run 'crumbly init' to create an index first")
    )]
    IndexOpen {
        source: crumbly_core::knowledge::IndexError,
    },

    #[snafu(display("Failed to create file scanner"))]
    #[diagnostic(
        code(crumbly::cli::cache::scanner_create),
        forward(source),
        help("Check that the source path exists and is readable")
    )]
    ScannerCreate {
        source: crumbly_core::knowledge::ScanError,
    },

    #[snafu(display("Failed to create chunking dispatcher"))]
    #[diagnostic(
        code(crumbly::cli::cache::dispatcher_create),
        help("Check chunking configuration in crumbly.toml")
    )]
    DispatcherCreate { source: DispatchError },

    #[snafu(display("Failed to open repository"))]
    #[diagnostic(
        code(crumbly::cli::cache::repository),
        forward(source),
        help("Check that the index database is not corrupted")
    )]
    Repository {
        source: crumbly_core::knowledge::StorageError,
    },

    #[snafu(display("Failed to create embedding model pool"))]
    #[diagnostic(
        code(crumbly::cli::cache::pool_creation),
        help("Check available memory and model cache directory permissions")
    )]
    PoolCreation {
        source: Box<crumbly_core::knowledge::search::embeddings::CreatePoolError>,
    },

    #[snafu(display("Failed to load configuration"))]
    #[diagnostic(
        code(crumbly::cli::cache::config_load),
        help("Check crumbly.toml syntax")
    )]
    ConfigLoad {
        source: crumbly_core::knowledge::CrumblyConfigError,
    },

    #[snafu(display("Invalid git revision: {rev}"))]
    #[diagnostic(
        code(crumbly::cli::cache::invalid_revision),
        help("Revision must be a non-empty string (branch, tag, or commit SHA)")
    )]
    InvalidRevision { rev: String },

    #[snafu(display("Filesystem caching failed"))]
    #[diagnostic(
        code(crumbly::cli::cache::filesystem),
        help("Check source files are readable and index has write permissions")
    )]
    FilesystemCache {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[snafu(display("Bare git caching failed"))]
    #[diagnostic(
        code(crumbly::cli::cache::bare_git),
        help("Check that bare repository path is valid and revision exists")
    )]
    BareGitCache {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

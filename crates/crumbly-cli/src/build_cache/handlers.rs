use miette::Diagnostic;
use snafu::{ResultExt, Snafu};
use std::sync::Arc;

use crumbly_core::knowledge::KnowledgeIndex;
use crumbly_core::knowledge::chunking::{ChunkingDispatcher, DispatchError};
use crumbly_core::knowledge::indexing::load_crumbly_config;
use crumbly_core::knowledge::indexing::provider::EmbeddingDataProvider;
use crumbly_core::knowledge::indexing::source::FilesystemSource;
use crumbly_core::knowledge::indexing::{
    BareGitSource, CacheResult, ChunkCacher, FileScanner, GitRev, ProgressReporter,
};
use crumbly_core::knowledge::search::EmbeddingModel;
use crumbly_core::knowledge::storage::sqlite::SqliteChunkRepository;

use super::{BareGitArgs, BuildCacheArgs, FilesystemArgs, SourceBackend};
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
) -> Result<CacheResult, BuildCacheError> {
    use build_cache_error::*;

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

    cacher
        .cache()
        .map_err(|e| BuildCacheError::FilesystemCache {
            source: Box::new(e),
        })
}

fn cache_bare_git(
    index: &KnowledgeIndex,
    args: BareGitArgs,
) -> Result<CacheResult, BuildCacheError> {
    use build_cache_error::*;

    let rev = GitRev::try_new(&args.rev).map_err(|_| BuildCacheError::InvalidRevision {
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

    cacher.cache().map_err(|e| BuildCacheError::BareGitCache {
        source: Box::new(e),
    })
}

fn create_provider(
    index: &KnowledgeIndex,
) -> Result<Box<dyn crumbly_core::knowledge::indexing::IndexDataProvider>, BuildCacheError> {
    use build_cache_error::*;

    let model = EmbeddingModel::builder()
        .model_name(index.config().model_name.clone())
        .dimension(index.config().embedding_dim)
        .cache_dir(index.index_root().join(".crumbly").join("models"))
        .build()
        .load()
        .context(EmbeddingModelSnafu)?;

    Ok(Box::new(EmbeddingDataProvider::new(Box::new(model))))
}

fn load_filter(
    index: &KnowledgeIndex,
) -> Result<crumbly_core::knowledge::indexing::IndexingFilter, BuildCacheError> {
    use build_cache_error::*;

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

    #[snafu(display("Failed to create file scanner"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::scanner_create),
        forward(source),
        help("Check that the source path exists and is readable")
    )]
    ScannerCreate {
        source: crumbly_core::knowledge::ScanError,
    },

    #[snafu(display("Failed to create chunking dispatcher"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::dispatcher_create),
        help("Check chunking configuration in crumbly.toml")
    )]
    DispatcherCreate { source: DispatchError },

    #[snafu(display("Failed to open repository"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::repository),
        forward(source),
        help("Check that the index database is not corrupted")
    )]
    Repository {
        source: crumbly_core::knowledge::StorageError,
    },

    #[snafu(display("Failed to load embedding model"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::embedding_model),
        forward(source),
        help("Check network connectivity for model download")
    )]
    EmbeddingModel {
        source: crumbly_core::knowledge::search::EmbeddingError,
    },

    #[snafu(display("Failed to load configuration"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::config_load),
        help("Check crumbly.toml syntax")
    )]
    ConfigLoad {
        source: crumbly_core::knowledge::CrumblyConfigError,
    },

    #[snafu(display("Invalid git revision: {rev}"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::invalid_revision),
        help("Revision must be a non-empty string (branch, tag, or commit SHA)")
    )]
    InvalidRevision { rev: String },

    #[snafu(display("Filesystem caching failed"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::filesystem),
        help("Check source files are readable and index has write permissions")
    )]
    FilesystemCache {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[snafu(display("Bare git caching failed"))]
    #[diagnostic(
        code(crumbly::cli::build_cache::bare_git),
        help("Check that bare repository path is valid and revision exists")
    )]
    BareGitCache {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

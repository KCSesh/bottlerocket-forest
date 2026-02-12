//! Cache operations for the knowledge index.

use std::sync::Arc;

use snafu::ResultExt;

use crate::knowledge::chunking::ChunkingDispatcher;
use crate::knowledge::facade::KnowledgeIndex;
use crate::knowledge::facade::types::{CacheSource, IndexError, index_error::*};
use crate::knowledge::indexing::source::{ContentSource, FilesystemSource};
use crate::knowledge::indexing::{
    BareGitSource, CacheError, CacheResult, ChunkCacher, FileScanner, IndexDataProvider,
    ProgressReporter,
};
use crate::knowledge::storage::sqlite::SqliteChunkRepository;

pub(in crate::knowledge::facade) fn cache(
    index: &KnowledgeIndex,
    source: CacheSource,
    progress: Option<Arc<dyn ProgressReporter>>,
) -> Result<CacheResult, IndexError> {
    let provider = super::create_provider(index)?;

    match source {
        CacheSource::Filesystem(path) => {
            let scanner = FileScanner::new(&path).context(ScannerFailedSnafu)?;
            let fs_source = FilesystemSource::new(scanner);
            run_cacher(index, fs_source, provider, progress).context(FilesystemCacheFailedSnafu)
        }
        CacheSource::BareGit {
            bare_repos_dir,
            rev,
        } => {
            let filter = super::load_indexing_filter(index)?;
            let git_source = BareGitSource::new(&bare_repos_dir, rev, filter);
            run_cacher(index, git_source, provider, progress).context(BareGitCacheFailedSnafu)
        }
    }
}

fn run_cacher<S>(
    index: &KnowledgeIndex,
    source: S,
    provider: Box<dyn IndexDataProvider>,
    progress: Option<Arc<dyn ProgressReporter>>,
) -> Result<CacheResult, CacheError<S::Error>>
where
    S: ContentSource,
{
    let dispatcher = ChunkingDispatcher::with_defaults(&index.config)
        .map_err(|e| CacheError::DispatcherInit { source: e })?;
    let repository = SqliteChunkRepository::open(&index.db_path, &index.config)
        .map_err(|e| CacheError::StorageFailed { source: e })?;

    ChunkCacher::<S, SqliteChunkRepository>::builder()
        .source(source)
        .dispatcher(dispatcher)
        .repository(repository)
        .provider(provider)
        .maybe_progress(progress)
        .build()
        .cache()
}

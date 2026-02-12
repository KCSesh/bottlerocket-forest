//! Cache operations for the knowledge index.

use std::sync::Arc;

use snafu::ResultExt;

use crate::knowledge::chunking::ChunkingDispatcher;
use crate::knowledge::facade::KnowledgeIndex;
use crate::knowledge::facade::types::{CacheSource, IndexError};
use crate::knowledge::indexing::source::FilesystemSource;
use crate::knowledge::indexing::{
    BareGitSource, CacheResult, ChunkCacher, FileScanner, GitRev, ProgressReporter,
};
use crate::knowledge::storage::sqlite::SqliteChunkRepository;

pub(in crate::knowledge::facade) fn cache(
    index: &KnowledgeIndex,
    source: CacheSource,
    progress: Option<Arc<dyn ProgressReporter>>,
) -> Result<CacheResult, IndexError> {
    use crate::knowledge::facade::types::index_error::*;

    let dispatcher =
        ChunkingDispatcher::with_defaults(&index.config).map_err(|e| IndexError::CacheFailed {
            message: e.to_string(),
        })?;
    let repository = SqliteChunkRepository::open(&index.db_path, &index.config)
        .context(DatabaseAccessFailedSnafu)?;
    let provider = super::create_provider(index)?;

    match source {
        CacheSource::Filesystem(path) => {
            let scanner = FileScanner::new(&path).map_err(|e| IndexError::CacheFailed {
                message: e.to_string(),
            })?;
            let fs_source = FilesystemSource::new(scanner);

            let mut cacher: ChunkCacher<FilesystemSource, SqliteChunkRepository> =
                ChunkCacher::builder()
                    .source(fs_source)
                    .dispatcher(dispatcher)
                    .repository(repository)
                    .provider(provider)
                    .maybe_progress(progress)
                    .build();

            cacher.cache().map_err(|e| IndexError::CacheFailed {
                message: e.to_string(),
            })
        }
        CacheSource::BareGit {
            bare_repos_dir,
            rev,
        } => {
            let git_rev = GitRev::try_new(&rev)
                .map_err(|_| IndexError::InvalidRevision { rev: rev.clone() })?;

            let filter = super::load_indexing_filter(index)?;
            let git_source = BareGitSource::new(&bare_repos_dir, git_rev, filter);

            let mut cacher: ChunkCacher<BareGitSource, SqliteChunkRepository> =
                ChunkCacher::builder()
                    .source(git_source)
                    .dispatcher(dispatcher)
                    .repository(repository)
                    .provider(provider)
                    .maybe_progress(progress)
                    .build();

            cacher.cache().map_err(|e| IndexError::CacheFailed {
                message: e.to_string(),
            })
        }
    }
}

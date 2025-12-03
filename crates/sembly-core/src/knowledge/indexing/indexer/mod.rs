//! Unified indexer for building and maintaining the knowledge index
//!
//! The [`Indexer`] orchestrates the complete indexing workflow: scanning files,
//! chunking content, generating embeddings, and storing indexed chunks. It supports
//! three strategies via [`IndexStrategy`]:
//!
//! * [`IndexStrategy::Build`] - Build index from scratch without clearing existing data
//! * [`IndexStrategy::Rebuild`] - Clear existing index then build from scratch
//! * [`IndexStrategy::Incremental`] - Update only changed files (additions, modifications, deletions)
//!
//! The indexer coordinates between the [`FileScanner`], [`ChunkingDispatcher`],
//! [`IndexDataProvider`], and [`ChunkRepository`] to transform raw documentation
//! files into searchable indexed chunks.

mod operations;
mod types;

pub use types::{BatchConfig, IndexResult, IndexingError};

use snafu::ResultExt;
use std::path::Path;
use std::sync::Arc;

use crate::knowledge::chunking::ChunkingDispatcher;
use crate::knowledge::domain::{ContextId, EmbeddingModelConfig, ScanConfig};
use crate::knowledge::storage::ChunkRepository;

use super::{FileScanner, IndexDataProvider, IndexingFilter, ProgressReporter};

/// Strategy for executing index operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStrategy {
    /// Build index from scratch (don't clear existing)
    Build,

    /// Clear existing index then build from scratch
    Rebuild,

    /// Update only changed files
    Incremental,
}

/// Orchestrates the complete indexing workflow from files to indexed chunks
pub struct Indexer<R: ChunkRepository> {
    scanner: FileScanner,
    dispatcher: ChunkingDispatcher,
    repository: R,
    provider: Box<dyn IndexDataProvider>,
    progress: Option<Arc<dyn ProgressReporter>>,
    batch_config: BatchConfig,
    context_id: ContextId,
}

type FileResult = Result<Vec<crate::knowledge::domain::Chunk>, Result<(), IndexingError>>;

#[bon::bon]
impl<R: ChunkRepository> Indexer<R> {
    /// Create an indexer for the specified forest root
    #[builder]
    pub fn new(
        forest_root: impl AsRef<Path>,
        repository: R,
        config: &EmbeddingModelConfig,
        provider: Box<dyn IndexDataProvider>,
        scan_config: ScanConfig,
        filter: IndexingFilter,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default)] batch_config: BatchConfig,
        context_id: ContextId,
    ) -> Result<Self, IndexingError> {
        use types::indexing_error::*;

        let scanner =
            FileScanner::with_progress(forest_root, scan_config, filter.clone(), progress.clone())
                .context(ScanFailedSnafu)?;
        let dispatcher = ChunkingDispatcher::with_defaults_and_filter(config, &filter)
            .context(ChunkingFailedSnafu)?;

        Ok(Self {
            scanner,
            dispatcher,
            repository,
            provider,
            progress,
            batch_config,
            context_id,
        })
    }

    /// Execute an indexing operation using the specified strategy
    pub fn index(&mut self, strategy: IndexStrategy) -> Result<IndexResult, IndexingError> {
        match strategy {
            IndexStrategy::Build => self.build(),
            IndexStrategy::Rebuild => self.rebuild(),
            IndexStrategy::Incremental => self.incremental(),
        }
    }
}

mod batch;
mod strategies;

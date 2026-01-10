//! High-level facade for the knowledge index
//!
//! The [`KnowledgeIndex`] provides a unified interface for all knowledge index operations,
//! coordinating between the indexer, repository, and search engines.
//!
//! # Quick Start
//!
//! ```no_run
//! use crumbly_core::knowledge::KnowledgeIndex;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open an index handle
//! let index = KnowledgeIndex::open("/path/to/forest")?;
//!
//! // Build the index (creates database)
//! let result = index.build().call()?;
//! println!("Indexed {} files", result.files_processed);
//!
//! // Search
//! let results = index.search("how does boot work", 10)?;
//! for result in results.results {
//!     println!("Score: {}, File: {}", result.score, result.chunk.source.file_path);
//! }
//!
//! // Check status
//! let status = index.status()?;
//! println!("Index has {} chunks from {} files", status.chunk_count, status.file_count);
//! # Ok(())
//! # }
//! ```

mod types;

pub use types::{GcStats, IndexError, IndexStatus};

use bon::Builder;
use snafu::ResultExt;
use std::path::{Path, PathBuf};

use crate::knowledge::constants::SEMBLY_DIR;
use crate::knowledge::domain::{
    ContextId, EmbeddingModelConfig, QueryText, ResultLimit, SearchQuery, SearchResults,
};
use crate::knowledge::search::SearchEngine;
use crate::knowledge::storage::ChunkRepository;
use crate::knowledge::storage::sqlite::SqliteChunkRepository;

/// High-level interface for the knowledge index
///
/// Coordinates indexing, storage, and search operations. Provides a unified
/// API for all knowledge index functionality.
#[derive(Builder)]
#[builder(on(_, into), builder_type = KnowledgeIndexConstructor)]
pub struct KnowledgeIndex {
    index_root: PathBuf,
    db_path: PathBuf,
    config: EmbeddingModelConfig,
}

impl KnowledgeIndex {
    /// Open a knowledge index with default configuration
    ///
    /// Creates the `.crumbly/` directory if it doesn't exist. Validates configuration
    /// against existing database if present. Does not create the database.
    pub fn open(index_root: impl AsRef<Path>) -> Result<Self, IndexError> {
        Self::open_with_config(index_root, EmbeddingModelConfig::default())
    }

    /// Open a knowledge index with custom configuration
    ///
    /// Validates the configuration against any existing index. Does not create the database.
    pub fn open_with_config(
        index_root: impl AsRef<Path>,
        config: EmbeddingModelConfig,
    ) -> Result<Self, IndexError> {
        use crate::knowledge::storage::StorageError;
        use types::index_error::*;

        let index_root = index_root.as_ref();

        if !index_root.exists() {
            return Err(IndexError::IndexRootNotFound {
                path: index_root.display().to_string(),
            });
        }

        let crumbly_dir = index_root.join(SEMBLY_DIR);
        if !crumbly_dir.exists() {
            std::fs::create_dir_all(&crumbly_dir).context(CrumblyDirCreationFailedSnafu)?;
        }

        let db_path = Self::default_db_path(index_root);

        // If database exists, validate config matches
        if db_path.exists() {
            let repository = SqliteChunkRepository::open(&db_path, &config)
                .context(DatabaseAccessFailedSnafu)?;

            let metadata = repository
                .get_metadata()
                .context(DatabaseAccessFailedSnafu)?;

            if metadata.model_config != config {
                return Err(IndexError::DatabaseAccessFailed {
                    source: StorageError::ConfigMismatch {
                        expected: config.clone(),
                        actual: metadata.model_config.clone(),
                    },
                });
            }
        }

        Ok(Self {
            index_root: index_root.to_path_buf(),
            db_path,
            config,
        })
    }

    /// Discover and open a knowledge index from the current working directory
    ///
    /// Walks up the directory tree from `cwd` to find a workspace containing
    /// `.crumbly/knowledge.db`. Returns an error if no workspace is found.
    pub fn discover(cwd: impl AsRef<Path>) -> Result<Self, IndexError> {
        Self::discover_with_config(cwd, EmbeddingModelConfig::default())
    }

    /// Discover and open a knowledge index with custom configuration
    ///
    /// Walks up the directory tree from `cwd` to find a workspace.
    pub fn discover_with_config(
        cwd: impl AsRef<Path>,
        config: EmbeddingModelConfig,
    ) -> Result<Self, IndexError> {
        use crate::knowledge::context::discover_workspace;

        let workspace = discover_workspace(cwd.as_ref())?;
        Self::open_with_config(workspace.root(), config)
    }

    /// Resolve the context for a given working directory
    ///
    /// Returns the most specific registered context that contains the given path.
    /// The path must be within the workspace.
    /// Search the index
    ///
    /// Executes a semantic search query using embeddings. The limit parameter
    /// controls the maximum number of results returned (1-100).
    /// Searches within the default context.
    pub fn search(
        &self,
        query: impl AsRef<str>,
        limit: usize,
    ) -> Result<SearchResults, IndexError> {
        use types::index_error::*;

        // Use the default context for search
        // SAFETY: "." is always a valid path that normalizes to "."
        let context_id = ContextId::from_path(".").expect("'.' is valid context id");

        let search_query = SearchQuery::builder()
            .text(QueryText::try_new(query.as_ref()).context(InvalidQuerySnafu)?)
            .limit(ResultLimit::try_new(limit).context(InvalidResultLimitSnafu)?)
            .context_id(context_id)
            .build();

        let engine = self.create_search_engine()?;
        engine.search(&search_query).context(SearchFailedSnafu)
    }

    /// Search the index within a specific context
    ///
    /// Executes a semantic search query scoped to the specified context.
    pub fn search_in_context(
        &self,
        query: impl AsRef<str>,
        limit: usize,
        context_id: ContextId,
    ) -> Result<SearchResults, IndexError> {
        use types::index_error::*;

        snafu::ensure!(
            self.db_path.exists(),
            IndexNotFoundSnafu {
                path: self.db_path.display().to_string()
            }
        );

        let text = QueryText::try_new(query.as_ref()).context(InvalidQuerySnafu)?;
        let limit = ResultLimit::try_new(limit).context(InvalidResultLimitSnafu)?;

        let search_query = SearchQuery::builder()
            .text(text)
            .limit(limit)
            .context_id(context_id)
            .build();

        let engine = self.create_search_engine()?;
        engine.search(&search_query).context(SearchFailedSnafu)
    }

    /// Get index status and statistics
    ///
    /// Returns metadata including chunk count, file count, last build time, and disk size.
    pub fn status(&self) -> Result<IndexStatus, IndexError> {
        use types::index_error::*;

        let repository = self.repository()?;
        let metadata = repository
            .get_metadata()
            .context(DatabaseAccessFailedSnafu)?;

        let size_bytes = std::fs::metadata(&self.db_path).map(|m| m.len()).ok();

        Ok(IndexStatus::builder()
            .exists(true)
            .chunk_count(metadata.chunk_count)
            .file_count(metadata.file_count)
            .last_build(metadata.last_build)
            .model_config(self.config.clone())
            .maybe_size_bytes(size_bytes)
            .build())
    }

    /// List all registered contexts in the workspace
    pub fn index_root(&self) -> &Path {
        &self.index_root
    }

    /// Get the database path
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Get the embedding model configuration
    pub fn config(&self) -> &EmbeddingModelConfig {
        &self.config
    }
}

mod build;
mod context;
mod gc;
mod helpers;
mod mod_tests;
mod test_helpers;
mod update;

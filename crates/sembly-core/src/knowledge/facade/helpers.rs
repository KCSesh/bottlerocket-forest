//! Internal helper methods for the knowledge index.

use super::KnowledgeIndex;
use super::types::IndexError;
use snafu::ResultExt;
use std::path::{Path, PathBuf};

use crate::knowledge::constants::{KNOWLEDGE_DB, MODEL_CACHE_DIR, SEMBLY_DIR};
use crate::knowledge::domain::{ContextId, ScanConfig};
use crate::knowledge::indexing::provider::EmbeddingDataProvider;
use crate::knowledge::indexing::{self, IndexingFilter, load_sembly_config};
use crate::knowledge::scoring::ScoreBooster;
use crate::knowledge::search::{EmbeddingModel, LoadedEmbeddingModel, SemanticSearchEngine};
use crate::knowledge::storage::ChunkRepository;
use crate::knowledge::storage::sqlite::SqliteChunkRepository;

impl KnowledgeIndex {
    /// Compute the default database path for a forest root
    ///
    /// Returns `<forest_root>/.sembly/knowledge.db`
    pub(super) fn default_db_path(forest_root: impl AsRef<Path>) -> PathBuf {
        forest_root.as_ref().join(SEMBLY_DIR).join(KNOWLEDGE_DB)
    }

    /// Open a repository connection to the database
    pub(super) fn repository(&self) -> Result<SqliteChunkRepository, IndexError> {
        use super::types::index_error::*;
        SqliteChunkRepository::open(&self.db_path, &self.config).context(DatabaseAccessFailedSnafu)
    }

    /// Update the last_build timestamp in metadata
    pub(super) fn update_last_build_timestamp(&self) -> Result<(), IndexError> {
        use super::types::index_error::*;

        let mut repository = self.repository()?;
        let mut metadata = repository
            .get_metadata()
            .context(DatabaseAccessFailedSnafu)?;
        metadata.last_build = std::time::SystemTime::now();
        repository
            .set_metadata(&metadata)
            .context(DatabaseAccessFailedSnafu)?;
        Ok(())
    }

    /// Create a search engine for the current index mode
    ///
    /// Initializes the semantic search engine with the configured embedding model
    /// and score boosting rules from the sembly configuration.
    pub(super) fn create_search_engine(
        &self,
    ) -> Result<SemanticSearchEngine<SqliteChunkRepository>, IndexError> {
        let repo = self.repository()?;
        let embedding_model = self.create_embedding_model()?;
        let score_booster = self.load_score_booster()?;

        Ok(SemanticSearchEngine::new(
            repo,
            Box::new(embedding_model),
            score_booster,
        ))
    }

    /// Create an embedding data provider for indexing operations
    ///
    /// Initializes the embedding model used to generate vector embeddings during indexing.
    pub(super) fn create_provider(&self) -> Result<EmbeddingDataProvider, IndexError> {
        let embedding_model = self.create_embedding_model()?;
        Ok(EmbeddingDataProvider::new(Box::new(embedding_model)))
    }

    /// Create an embedding model with the configured parameters
    pub(super) fn create_embedding_model(&self) -> Result<LoadedEmbeddingModel, IndexError> {
        use super::types::index_error::*;

        EmbeddingModel::builder()
            .model_name(self.config.model_name.clone())
            .dimension(self.config.embedding_dim)
            .cache_dir(self.forest_root.join(SEMBLY_DIR).join(MODEL_CACHE_DIR))
            .build()
            .load()
            .context(EmbeddingProviderCreationFailedSnafu)
    }

    /// Load score booster from sembly configuration
    pub(super) fn load_score_booster(&self) -> Result<ScoreBooster, IndexError> {
        use super::types::index_error::*;

        let sembly_config = load_sembly_config(&self.forest_root).context(ConfigLoadFailedSnafu)?;

        let booster = sembly_config
            .filter(|c| !c.boost_rules.is_empty())
            .map(|c| ScoreBooster::new(c.boost_rules))
            .unwrap_or_default();

        Ok(booster)
    }

    /// Load scan configuration for a specific context
    ///
    /// When context_id is not ".", targets are prefixed with the context path.
    /// This implements MCI-21: target paths resolve relative to context root.
    pub(super) fn load_scan_config_for_context(
        &self,
        context_id: &ContextId,
    ) -> Result<ScanConfig, IndexError> {
        use super::types::index_error::*;

        let sembly_config =
            indexing::load_sembly_config(&self.forest_root).context(ConfigLoadFailedSnafu)?;

        let base_targets = sembly_config
            .as_ref()
            .map(|c| c.targets.clone())
            .unwrap_or_default();

        // Prefix targets with context path if context is not default (MCI-21)
        let targets = if context_id.as_str() != "." {
            let ctx_path = PathBuf::from(context_id.as_str());
            base_targets
                .into_iter()
                .map(|t| match t.as_os_str() {
                    // "." target becomes the context path itself
                    s if s == "." => ctx_path.clone(),
                    // Other targets are prefixed with context path
                    _ => ctx_path.join(t),
                })
                .collect()
        } else {
            base_targets
        };

        Ok(ScanConfig::builder().targets(targets).build())
    }

    /// Load indexing filter rules from sembly configuration
    ///
    /// Reads `.sembly.toml` to determine which files should be excluded from indexing.
    pub(super) fn load_indexing_filter(&self) -> Result<IndexingFilter, IndexError> {
        use super::types::index_error::*;

        let sembly_config =
            indexing::load_sembly_config(&self.forest_root).context(ConfigLoadFailedSnafu)?;

        let filter = sembly_config
            .map(|c| c.to_indexing_filter())
            .transpose()
            .context(ConfigLoadFailedSnafu)?
            .unwrap_or_default();

        Ok(filter)
    }
}

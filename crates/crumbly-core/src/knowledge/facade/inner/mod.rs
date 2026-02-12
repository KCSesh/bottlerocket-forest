//! Internal implementation of KnowledgeIndex operations.
//!
//! This module provides helper functions and operation implementations that power the
//! KnowledgeIndex facade.

mod builder;
mod context;
mod gc;
mod searcher;
mod updater;

pub(in crate::knowledge::facade) use builder::{build, build_cache, ensure_cache, rebuild};
pub(in crate::knowledge::facade) use context::{list_contexts, remove_context, resolve_context};
pub(in crate::knowledge::facade) use gc::gc;
pub(in crate::knowledge::facade) use searcher::{search, search_in_context};
pub(in crate::knowledge::facade) use updater::{clear, update};

use snafu::ResultExt;
use std::path::{Path, PathBuf};

use super::KnowledgeIndex;
use crate::knowledge::constants::{KNOWLEDGE_DB, MODEL_CACHE_DIR, SEMBLY_DIR};
use crate::knowledge::domain::ContextId;
use crate::knowledge::facade::types::IndexError;
use crate::knowledge::indexing::IndexDataProvider;
use crate::knowledge::indexing::ScanConfig;
use crate::knowledge::indexing::{self, IndexingFilter, load_crumbly_config};
use crate::knowledge::scoring::ScoreBooster;
use crate::knowledge::search::embeddings::PooledEmbeddingProvider;
use crate::knowledge::search::{EmbeddingModel, LoadedEmbeddingModel, SemanticSearchEngine};
use crate::knowledge::storage::ChunkRepository;
use crate::knowledge::storage::sqlite::SqliteChunkRepository;

pub(super) fn default_db_path(index_root: impl AsRef<Path>) -> PathBuf {
    index_root.as_ref().join(SEMBLY_DIR).join(KNOWLEDGE_DB)
}

pub(super) fn repository(index: &KnowledgeIndex) -> Result<SqliteChunkRepository, IndexError> {
    use super::types::index_error::*;
    SqliteChunkRepository::open(&index.db_path, &index.config).context(DatabaseAccessFailedSnafu)
}

pub(super) fn update_last_build_timestamp(index: &KnowledgeIndex) -> Result<(), IndexError> {
    use super::types::index_error::*;

    let mut repo = repository(index)?;
    let mut metadata = repo.get_metadata().context(DatabaseAccessFailedSnafu)?;
    metadata.last_build = std::time::SystemTime::now();
    repo.set_metadata(&metadata)
        .context(DatabaseAccessFailedSnafu)?;
    Ok(())
}

pub(super) fn create_search_engine(
    index: &KnowledgeIndex,
) -> Result<SemanticSearchEngine<SqliteChunkRepository>, IndexError> {
    let repo = repository(index)?;
    let embedding_model = create_embedding_model(index)?;
    let score_booster = load_score_booster(index)?;

    Ok(SemanticSearchEngine::new(
        repo,
        Box::new(embedding_model),
        score_booster,
    ))
}

pub(super) fn create_provider(
    index: &KnowledgeIndex,
) -> Result<Box<dyn IndexDataProvider>, IndexError> {
    use super::types::index_error::*;
    use crate::knowledge::search::embeddings::PoolConfig;

    let config = PoolConfig::builder()
        .cache_dir(index.index_root.join(SEMBLY_DIR).join(MODEL_CACHE_DIR))
        .build();

    Ok(Box::new(
        PooledEmbeddingProvider::with_config(config)
            .map_err(Box::new)
            .context(PoolCreationFailedSnafu)?,
    ))
}

pub(super) fn create_embedding_model(
    index: &KnowledgeIndex,
) -> Result<LoadedEmbeddingModel, IndexError> {
    use super::types::index_error::*;

    EmbeddingModel::builder()
        .model_name(index.config.model_name.clone())
        .dimension(index.config.embedding_dim)
        .cache_dir(index.index_root.join(SEMBLY_DIR).join(MODEL_CACHE_DIR))
        .build()
        .load()
        .context(EmbeddingProviderCreationFailedSnafu)
}

pub(super) fn load_score_booster(index: &KnowledgeIndex) -> Result<ScoreBooster, IndexError> {
    use super::types::index_error::*;

    let crumbly_config = load_crumbly_config(&index.index_root).context(ConfigLoadFailedSnafu)?;

    let booster = crumbly_config
        .filter(|c| !c.boost_rules.is_empty())
        .map(|c| ScoreBooster::new(c.boost_rules))
        .unwrap_or_default();

    Ok(booster)
}

pub(super) fn load_scan_config_for_context(
    index: &KnowledgeIndex,
    context_id: &ContextId,
) -> Result<ScanConfig, IndexError> {
    use super::types::index_error::*;

    let crumbly_config =
        indexing::load_crumbly_config(&index.index_root).context(ConfigLoadFailedSnafu)?;

    let base_targets = crumbly_config
        .as_ref()
        .map(|c| c.targets.clone())
        .unwrap_or_default();

    let targets = if context_id.as_str() != "." {
        let ctx_path = PathBuf::from(context_id.as_str());
        base_targets
            .into_iter()
            .map(|t| match t.as_os_str() {
                s if s == "." => ctx_path.clone(),
                _ => ctx_path.join(t),
            })
            .collect()
    } else {
        base_targets
    };

    Ok(ScanConfig::builder().targets(targets).build())
}

pub(super) fn load_indexing_filter(index: &KnowledgeIndex) -> Result<IndexingFilter, IndexError> {
    use super::types::index_error::*;

    let crumbly_config =
        indexing::load_crumbly_config(&index.index_root).context(ConfigLoadFailedSnafu)?;

    let filter = crumbly_config
        .map(|c| c.to_indexing_filter())
        .transpose()
        .context(ConfigLoadFailedSnafu)?
        .unwrap_or_default();

    Ok(filter)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::domain::ContextId;
    use crate::knowledge::facade::KnowledgeIndex;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_db_path_returns_correct_path() {
        let index_root = std::path::Path::new("/test/forest");

        let db_path = default_db_path(index_root);

        assert_eq!(db_path, index_root.join(".crumbly/knowledge.db"));
    }

    #[test]
    fn test_load_scan_config_with_existing_crumbly_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
targets = ["docs", "bottlerocket"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();
        fs::create_dir_all(temp_dir.path().join(".crumbly")).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let default_context = ContextId::from_path(".").unwrap();

        let scan_config = load_scan_config_for_context(&index, &default_context).unwrap();

        assert_eq!(scan_config.targets.len(), 2);
        assert_eq!(scan_config.targets[0], std::path::PathBuf::from("docs"));
        assert_eq!(
            scan_config.targets[1],
            std::path::PathBuf::from("bottlerocket")
        );
    }

    #[test]
    fn test_load_scan_config_without_crumbly_toml() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".crumbly")).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let default_context = ContextId::from_path(".").unwrap();

        let scan_config = load_scan_config_for_context(&index, &default_context).unwrap();

        assert!(scan_config.targets.is_empty());
    }

    #[test]
    fn test_load_scan_config_prefixes_targets_for_non_default_context() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
targets = [".", "docs"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();
        fs::create_dir_all(temp_dir.path().join(".crumbly")).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let context_id = ContextId::from_path("worktree/feature-a").unwrap();

        let scan_config = load_scan_config_for_context(&index, &context_id).unwrap();

        assert_eq!(scan_config.targets.len(), 2);
        assert_eq!(
            scan_config.targets[0],
            std::path::PathBuf::from("worktree/feature-a")
        );
        assert_eq!(
            scan_config.targets[1],
            std::path::PathBuf::from("worktree/feature-a/docs")
        );
    }
}

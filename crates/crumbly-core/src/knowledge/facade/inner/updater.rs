//! Update and clear operations for the knowledge index.

use std::sync::Arc;

use snafu::ResultExt;

use super::KnowledgeIndex;
use crate::knowledge::domain::{BatchSize, Context, ContextId};
use crate::knowledge::facade::types::IndexError;
use crate::knowledge::facade::types::index_error::*;
use crate::knowledge::indexing::{
    BatchConfig, IndexDataProvider, IndexResult, IndexStrategy, Indexer, ProgressReporter,
};
use crate::knowledge::storage::ContextRepository;

pub(in crate::knowledge::facade) fn update(
    index: &KnowledgeIndex,
    progress: Option<Arc<dyn ProgressReporter>>,
    batch_size: BatchSize,
    context_id: ContextId,
) -> Result<IndexResult, IndexError> {
    snafu::ensure!(
        index.db_path.exists(),
        IndexNotFoundSnafu {
            path: index.db_path.display().to_string()
        }
    );

    let provider = Box::new(super::create_provider(index)?) as Box<dyn IndexDataProvider>;
    let scan_config = super::load_scan_config_for_context(index, &context_id)?;
    let filter = super::load_indexing_filter(index)?;
    let repository = super::repository(index)?;

    let batch_config = BatchConfig { batch_size };

    if context_id.as_str() != "." {
        let context_repo = repository.context_repository();
        let exists = context_repo
            .get_context(&context_id)
            .context(ContextRegistrationFailedSnafu)?
            .is_some();

        if !exists {
            let context = Context::builder().context_id(context_id.clone()).build();
            context_repo
                .insert_context(&context)
                .context(ContextRegistrationFailedSnafu)?;
        }
    }

    let mut indexer = Indexer::builder()
        .index_root(&index.index_root)
        .repository(repository)
        .config(&index.config)
        .provider(provider)
        .scan_config(scan_config)
        .filter(filter)
        .maybe_progress(progress)
        .batch_config(batch_config)
        .context_id(context_id)
        .build()
        .context(IndexingFailedSnafu)?;

    let result = indexer
        .index(IndexStrategy::Incremental)
        .context(IndexingFailedSnafu)?;

    super::update_last_build_timestamp(index)?;

    Ok(result)
}

pub(in crate::knowledge::facade) fn clear(
    index: &KnowledgeIndex,
    context_id: &ContextId,
) -> Result<usize, IndexError> {
    use crate::knowledge::storage::ChunkRepository;

    let mut repository = super::repository(index)?;
    let deleted = repository
        .clear_context_files(context_id)
        .context(DatabaseAccessFailedSnafu)?;
    repository
        .delete_orphaned_chunks()
        .context(DatabaseAccessFailedSnafu)?;

    Ok(deleted)
}

#[cfg(test)]
mod test {
    use crate::knowledge::KnowledgeIndex;
    use crate::knowledge::domain::ContextId;
    use crate::knowledge::facade::test_helpers::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_update_detects_new_files() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        create_test_file(temp_dir.path(), "test-repo", "new.md", "# New\n\nContent");

        let result = index.update().call().unwrap();

        assert_eq!(result.files_added, 1);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_update_detects_deleted_files() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        let file_path = repo_dir.join("test.md");
        fs::write(&file_path, "# Test\n\nContent").unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        fs::remove_file(&file_path).unwrap();

        let result = index.update().call().unwrap();

        assert_eq!(result.files_removed, 1);
    }

    #[test]
    fn test_clear_removes_file_mappings() {
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);

        let db_path = temp_dir.path().join(".crumbly/knowledge.db");
        assert!(db_path.exists());

        let default_ctx = ContextId::from_path(".").unwrap();
        let result = index.clear(&default_ctx);

        assert!(result.is_ok());
        assert!(db_path.exists());
        assert!(result.unwrap() >= 1);
    }

    #[test]
    fn test_clear_on_empty_context_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);

        let other_ctx = ContextId::from_path("other").unwrap();
        let result = index.clear(&other_ctx);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_update_uses_configured_targets() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = temp_dir.path().join("docs");
        let other_dir = temp_dir.path().join("other");
        fs::create_dir(&docs_dir).unwrap();
        fs::create_dir(&other_dir).unwrap();

        let config_content = r#"
targets = ["docs"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        fs::write(docs_dir.join("new.md"), "# New").unwrap();
        fs::write(other_dir.join("ignored.md"), "# Ignored").unwrap();

        let result = index.update().call().unwrap();

        assert_eq!(result.files_added, 1);
    }
}

//! Update and clear operations for the knowledge index.

use std::sync::Arc;

use snafu::ResultExt;

use super::KnowledgeIndex;
use crate::knowledge::domain::{BatchSize, Context, ContextId};
use crate::knowledge::facade::types::IndexError;
use crate::knowledge::indexing::{
    BatchConfig, IndexResult, IndexStrategy, Indexer, ProgressReporter,
};
use crate::knowledge::storage::ContextRepository;

pub(in crate::knowledge::facade) fn update(
    index: &KnowledgeIndex,
    progress: Option<Arc<dyn ProgressReporter>>,
    batch_size: BatchSize,
    context_id: ContextId,
) -> Result<IndexResult, IndexError> {
    use crate::knowledge::facade::types::index_error::*;
    snafu::ensure!(
        index.db_path.exists(),
        IndexNotFoundSnafu {
            path: index.db_path.display().to_string()
        }
    );

    let provider = super::create_provider(index)?;
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
        .provider(provider.into())
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
    use crate::knowledge::facade::types::index_error::*;
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
        // Given an index with no files
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When a new file is added and update is called
        create_test_file(temp_dir.path(), "test-repo", "new.md", "# New\n\nContent");
        let result = index.update().call().unwrap();

        // Then the new file should be detected
        assert_eq!(result.files_added, 1);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_update_detects_deleted_files() {
        // Given an index with one file
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        let file_path = repo_dir.join("test.md");
        fs::write(&file_path, "# Test\n\nContent").unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When the file is deleted and update is called
        fs::remove_file(&file_path).unwrap();
        let result = index.update().call().unwrap();

        // Then the deletion should be detected
        assert_eq!(result.files_removed, 1);
    }

    #[test]
    fn test_clear_removes_file_mappings() {
        // Given an index with content
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);
        let db_path = temp_dir.path().join(".crumbly/knowledge.db");
        assert!(db_path.exists());

        // When clearing the default context
        let default_ctx = ContextId::from_path(".").unwrap();
        let result = index.clear(&default_ctx);

        // Then files should be removed but database preserved
        assert!(result.is_ok());
        assert!(db_path.exists());
        assert!(result.unwrap() >= 1);
    }

    #[test]
    fn test_clear_on_empty_context_succeeds() {
        // Given an index with content in default context
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);

        // When clearing a different empty context
        let other_ctx = ContextId::from_path("other").unwrap();
        let result = index.clear(&other_ctx);

        // Then it should succeed with zero deletions
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_update_uses_configured_targets() {
        // Given a config targeting only "docs" directory
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

        // When adding files to both directories and updating
        fs::write(docs_dir.join("new.md"), "# New").unwrap();
        fs::write(other_dir.join("ignored.md"), "# Ignored").unwrap();
        let result = index.update().call().unwrap();

        // Then only the docs file should be indexed
        assert_eq!(result.files_added, 1);
    }
}

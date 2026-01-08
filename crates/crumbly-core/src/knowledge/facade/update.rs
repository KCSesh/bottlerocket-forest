//! Update and clear operations for the knowledge index.

use super::KnowledgeIndex;
use super::types::IndexError;
use snafu::ResultExt;
use std::sync::Arc;

use crate::knowledge::domain::{Context, ContextId};
use crate::knowledge::indexing::{
    BatchConfig, IndexDataProvider, IndexResult, IndexStrategy, Indexer, ProgressReporter,
};
use crate::knowledge::storage::{ChunkRepository, ContextRepository};

#[bon::bon]
impl KnowledgeIndex {
    /// Update the index incrementally
    ///
    /// Processes only files that have been added, modified, or deleted since
    /// the last index operation. Requires an existing index.
    #[builder]
    pub fn update(
        &self,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default = 100)] batch_size: usize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        use super::types::index_error::*;

        snafu::ensure!(
            self.db_path.exists(),
            IndexNotFoundSnafu {
                path: self.db_path.display().to_string()
            }
        );

        let provider = Box::new(self.create_provider()?) as Box<dyn IndexDataProvider>;
        let scan_config = self.load_scan_config_for_context(&context_id)?;
        let filter = self.load_indexing_filter()?;
        let repository = self.repository()?;

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
            .index_root(&self.index_root)
            .repository(repository)
            .config(&self.config)
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

        self.update_last_build_timestamp()?;

        Ok(result)
    }

    /// Clear file mappings for a context
    ///
    /// Removes file mappings for the specified context, leaving the context registered.
    /// Also runs garbage collection to remove orphaned chunks.
    pub fn clear(&self, context_id: &ContextId) -> Result<usize, IndexError> {
        use super::types::index_error::*;

        let mut repository = self.repository()?;
        let deleted = repository
            .clear_context_files(context_id)
            .context(DatabaseAccessFailedSnafu)?;
        repository
            .delete_orphaned_chunks()
            .context(DatabaseAccessFailedSnafu)?;

        Ok(deleted)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::facade::test_helpers::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_update_detects_new_files() {
        // Given: An index with no files
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When: Adding a new file and updating
        create_test_file(temp_dir.path(), "test-repo", "new.md", "# New\n\nContent");

        let result = index.update().call().unwrap();

        // Then: It should report one file added
        assert_eq!(result.files_added, 1);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_update_detects_deleted_files() {
        // Given: An index with a file
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        let file_path = repo_dir.join("test.md");
        fs::write(&file_path, "# Test\n\nContent").unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When: Deleting the file and updating
        fs::remove_file(&file_path).unwrap();

        let result = index.update().call().unwrap();

        // Then: It should report one file removed
        assert_eq!(result.files_removed, 1);
    }

    #[test]
    fn test_clear_removes_file_mappings() {
        // Given: An index with chunks
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);

        let db_path = temp_dir.path().join(".crumbly/knowledge.db");
        assert!(db_path.exists());

        // When: Clearing the default context
        let default_ctx = ContextId::from_path(".").unwrap();
        let result = index.clear(&default_ctx);

        // Then: It should succeed and database should still exist
        assert!(result.is_ok());
        assert!(db_path.exists());
        assert!(result.unwrap() >= 1);
    }

    #[test]
    fn test_clear_on_empty_context_succeeds() {
        // Given: An index with no files in context
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);

        // When: Clearing a context that has no files
        let other_ctx = ContextId::from_path("other").unwrap();
        let result = index.clear(&other_ctx);

        // Then: It should succeed with 0 deleted
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_update_uses_configured_targets() {
        // Given: A forest with configured targets
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

        // When: Adding files to both directories and updating
        fs::write(docs_dir.join("new.md"), "# New").unwrap();
        fs::write(other_dir.join("ignored.md"), "# Ignored").unwrap();

        let result = index.update().call().unwrap();

        // Then: It should only detect changes in configured targets
        assert_eq!(result.files_added, 1);
    }
}

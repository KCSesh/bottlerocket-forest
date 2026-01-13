//! Update and clear operations for the knowledge index.

use super::KnowledgeIndex;
use super::inner;
use super::types::IndexError;
use std::sync::Arc;

use crate::knowledge::domain::{BatchSize, ContextId};
use crate::knowledge::indexing::{IndexResult, ProgressReporter};

#[expect(clippy::expect_used)]
fn default_batch_size() -> BatchSize {
    BatchSize::try_new(100).expect("100 is valid batch size")
}

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
        #[builder(default = default_batch_size())] batch_size: BatchSize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        inner::update(self, progress, batch_size, context_id)
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

//! Build and rebuild operations for the knowledge index.

use super::KnowledgeIndex;
use super::inner;
use super::types::IndexError;
use std::sync::Arc;

use crate::knowledge::domain::{BatchSize, ContextId};
use crate::knowledge::indexing::{IndexResult, ProgressReporter};

fn default_batch_size() -> BatchSize {
    BatchSize::try_new(100).expect("100 is valid batch size")
}

#[bon::bon]
impl KnowledgeIndex {
    #[builder]
    pub fn build(
        &self,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default = default_batch_size())] batch_size: BatchSize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        inner::build(self, progress, batch_size, context_id)
    }

    /// Delete the index and rebuild from scratch
    ///
    /// Deletes the existing index database and creates a new one by scanning all files.
    #[builder]
    pub fn rebuild(
        &self,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default = default_batch_size())] batch_size: BatchSize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        inner::rebuild(self, progress, batch_size, context_id)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::facade::test_helpers::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_build_indexes_files_in_forest() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "README.md",
            "# Test\n\nContent here",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.build().call();

        assert!(result.is_ok());
        let index_result = result.unwrap();
        assert!(index_result.files_processed > 0);
        assert!(index_result.chunks_affected > 0);
    }

    #[test]
    fn test_build_handles_empty_forest() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        let result = index.build().call().unwrap();

        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_affected, 0);
    }

    #[test]
    fn test_build_uses_configured_targets() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join(".git")).unwrap();
        create_test_file(
            temp_dir.path(),
            "docs",
            "guide.md",
            "# Guide\n\nDocumentation content",
        );
        create_test_file(
            temp_dir.path(),
            "bottlerocket",
            "README.md",
            "# Bottlerocket\n\nProject info",
        );
        create_test_file(
            temp_dir.path(),
            "ignored",
            "secret.md",
            "# Secret\n\nSecret content",
        );

        let config_content = r#"
targets = ["docs", "bottlerocket"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.build().call().unwrap();

        assert_eq!(result.files_processed, 2);
        let status = index.status().unwrap();
        assert_eq!(status.file_count, 2);
    }

    #[test]
    fn test_rebuild_clears_existing_chunks() {
        let temp_dir = TempDir::new().unwrap();
        let _index = test_index_with_content(&temp_dir);

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call();

        assert!(result.is_ok());
    }

    #[test]
    fn test_rebuild_reindexes_all_files() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "test-repo", "test.md", "# Test\n\nContent");

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call().unwrap();

        assert!(result.files_processed > 0);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_rebuild_uses_configured_targets() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join(".git")).unwrap();
        create_test_file(
            temp_dir.path(),
            "docs",
            "guide.md",
            "# Guide\n\nDocumentation",
        );
        create_test_file(
            temp_dir.path(),
            "other",
            "other.md",
            "# Other\n\nOther content",
        );

        let config_content = r#"
targets = ["docs"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call().unwrap();

        assert_eq!(result.files_processed, 1);
    }
}

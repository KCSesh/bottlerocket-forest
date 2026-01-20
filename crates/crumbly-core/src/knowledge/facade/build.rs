//! Build and rebuild operations for the knowledge index.

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
    /// Build the knowledge index from configured sources.
    #[builder]
    pub fn build(
        &self,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default = default_batch_size())] batch_size: BatchSize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        inner::build(self, progress, batch_size, context_id)
    }

    /// Create database and metadata without indexing files.
    ///
    /// Creates the database and saves metadata but skips context creation
    /// and file indexing. Use this to prepare a database for cache warming.
    pub fn build_cache(&self) -> Result<(), IndexError> {
        inner::build_cache(self)
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
        // Given a forest with a markdown file
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "README.md",
            "# Test\n\nContent here",
        );

        // When building the index
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.build().call();

        // Then files and chunks are processed
        assert!(result.is_ok());
        let index_result = result.unwrap();
        assert!(index_result.files_processed > 0);
        assert!(index_result.chunks_affected > 0);
    }

    #[test]
    fn test_build_handles_empty_forest() {
        // Given an empty forest
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When building the index
        let result = index.build().call().unwrap();

        // Then no files or chunks are processed
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_affected, 0);
    }

    #[test]
    fn test_build_uses_configured_targets() {
        // Given a forest with configured targets excluding some directories
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

        // When building the index
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.build().call().unwrap();

        // Then only files in configured targets are indexed
        assert_eq!(result.files_processed, 2);
        let status = index.status().unwrap();
        assert_eq!(status.file_count, 2);
    }

    #[test]
    fn test_rebuild_clears_existing_chunks() {
        // Given a forest with existing indexed content
        let temp_dir = TempDir::new().unwrap();
        let _index = test_index_with_content(&temp_dir);

        // When rebuilding the index
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call();

        // Then rebuild succeeds
        assert!(result.is_ok());
    }

    #[test]
    fn test_rebuild_reindexes_all_files() {
        // Given a forest with a markdown file
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "test-repo", "test.md", "# Test\n\nContent");

        // When rebuilding the index
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call().unwrap();

        // Then files and chunks are processed
        assert!(result.files_processed > 0);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_rebuild_uses_configured_targets() {
        // Given a forest with configured targets
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

        // When rebuilding the index
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call().unwrap();

        // Then only files in configured targets are indexed
        assert_eq!(result.files_processed, 1);
    }

    #[test]
    fn test_build_cache_creates_database() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        let result = index.build_cache();

        assert!(result.is_ok());
        assert!(
            temp_dir
                .path()
                .join(".crumbly")
                .join("knowledge.db")
                .exists()
        );
    }

    #[test]
    fn test_build_cache_creates_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        index.build_cache().unwrap();

        let status = index.status().unwrap();
        assert_eq!(status.file_count, 0);
        assert_eq!(status.chunk_count, 0);
    }

    #[test]
    fn test_build_cache_skips_context() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        index.build_cache().unwrap();

        let contexts = index.list_contexts().unwrap();
        assert!(contexts.is_empty());
    }

    #[test]
    fn test_build_cache_fails_if_exists() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        index.build_cache().unwrap();

        let result = index.build_cache();
        assert!(result.is_err());
    }
}

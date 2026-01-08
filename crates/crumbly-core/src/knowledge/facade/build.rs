//! Build, rebuild, and update operations for the knowledge index.

use super::KnowledgeIndex;
use super::types::IndexError;
use snafu::ResultExt;
use std::sync::Arc;

use crate::knowledge::domain::{Context, ContextId, IndexMetadata};
use crate::knowledge::indexing::{
    BatchConfig, IndexDataProvider, IndexResult, IndexStrategy, Indexer, ProgressReporter,
};
use crate::knowledge::storage::sqlite::SqliteChunkRepository;
use crate::knowledge::storage::{ChunkRepository, ContextRepository};

#[bon::bon]
impl KnowledgeIndex {
    #[builder]
    pub fn build(
        &self,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default = 100)] batch_size: usize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        use super::types::index_error::*;

        snafu::ensure!(
            !self.db_path.exists(),
            IndexAlreadyExistsSnafu {
                path: self.db_path.display().to_string()
            }
        );

        let mut repository = SqliteChunkRepository::open(&self.db_path, &self.config)
            .context(DatabaseAccessFailedSnafu)?;

        let metadata = IndexMetadata::builder()
            .last_build(std::time::SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(self.config.clone())
            .build();
        repository
            .set_metadata(&metadata)
            .context(DatabaseAccessFailedSnafu)?;

        let context = Context::builder().context_id(context_id.clone()).build();
        repository
            .context_repository()
            .insert_context(&context)
            .context(ContextRegistrationFailedSnafu)?;

        let provider = Box::new(self.create_provider()?) as Box<dyn IndexDataProvider>;
        let scan_config = self.load_scan_config_for_context(&context_id)?;
        let filter = self.load_indexing_filter()?;
        let repository = self.repository()?;

        let batch_config = BatchConfig { batch_size };

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
            .index(IndexStrategy::Build)
            .context(IndexingFailedSnafu)?;

        self.update_last_build_timestamp()?;

        Ok(result)
    }

    /// Delete the index and rebuild from scratch
    ///
    /// Deletes the existing index database and creates a new one by scanning all files.
    #[builder]
    pub fn rebuild(
        &self,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default = 100)] batch_size: usize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        use super::types::index_error::*;

        if self.db_path.exists() {
            std::fs::remove_file(&self.db_path).context(IndexDeletionFailedSnafu)?;
        }

        let mut repository = SqliteChunkRepository::open(&self.db_path, &self.config)
            .context(DatabaseAccessFailedSnafu)?;

        let metadata = IndexMetadata::builder()
            .last_build(std::time::SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(self.config.clone())
            .build();
        repository
            .set_metadata(&metadata)
            .context(DatabaseAccessFailedSnafu)?;

        let default_context = Context::builder()
            .context_id(ContextId::from_path(".").expect("'.' is valid context id"))
            .build();
        repository
            .context_repository()
            .insert_context(&default_context)
            .context(ContextRegistrationFailedSnafu)?;

        if context_id.as_str() != "." {
            let context = Context::builder().context_id(context_id.clone()).build();
            repository
                .context_repository()
                .insert_context(&context)
                .context(ContextRegistrationFailedSnafu)?;
        }

        let provider = Box::new(self.create_provider()?) as Box<dyn IndexDataProvider>;
        let scan_config = self.load_scan_config_for_context(&context_id)?;
        let filter = self.load_indexing_filter()?;
        let repository = self.repository()?;

        let batch_config = BatchConfig { batch_size };

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
            .index(IndexStrategy::Build)
            .context(IndexingFailedSnafu)?;

        self.update_last_build_timestamp()?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::EmbeddingModelConfig;
    use crate::knowledge::facade::test_helpers::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_open_creates_crumbly_directory() {
        // Given A forest root without .crumbly directory
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();

        // When Opening an index
        let result = KnowledgeIndex::open(index_root);

        // Then It should create .crumbly directory
        assert!(result.is_ok());
        assert!(index_root.join(".crumbly").exists());
    }

    #[test]
    fn test_open_does_not_create_database_file() {
        // Given A forest root without existing database
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();

        // When Opening an index
        let result = KnowledgeIndex::open(index_root);

        // Then It should succeed but not create knowledge.db
        assert!(result.is_ok());
        assert!(!index_root.join(".crumbly/knowledge.db").exists());
    }

    #[test]
    fn test_open_with_nonexistent_index_root_fails() {
        // Given A nonexistent forest root
        let nonexistent = std::path::Path::new("/nonexistent/forest");

        // When Opening an index
        let result = KnowledgeIndex::open(nonexistent);

        // Then It should fail with IndexRootNotFound
        assert!(matches!(result, Err(IndexError::IndexRootNotFound { .. })));
    }

    #[test]
    fn test_open_returns_index_with_correct_paths() {
        // Given A forest root
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();

        // When Opening an index
        let index = KnowledgeIndex::open(index_root).unwrap();

        // Then Paths should be correct
        assert_eq!(index.index_root(), index_root);
        assert_eq!(index.db_path(), index_root.join(".crumbly/knowledge.db"));
    }

    #[test]
    fn test_open_with_existing_index_same_mode_succeeds() {
        // Given An existing index
        let temp_dir = TempDir::new().unwrap();
        let _index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Opening with same mode
        let result = KnowledgeIndex::open(temp_dir.path());

        // Then It should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_with_config_uses_custom_config() {
        // Given A custom embedding config
        let temp_dir = TempDir::new().unwrap();
        let custom_config = EmbeddingModelConfig::builder()
            .model_name("custom-model")
            .embedding_dim(512)
            .max_tokens(512)
            .overlap_tokens(50)
            .build();

        // When Opening with custom config
        let index =
            KnowledgeIndex::open_with_config(temp_dir.path(), custom_config.clone()).unwrap();

        // Then Index should use custom config
        assert_eq!(index.config(), &custom_config);
    }

    #[test]
    fn test_open_with_config_validates_existing_config() {
        // Given An existing index with default config
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "test-repo", "test.md", "# Test");

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When Opening with different config
        let different_config = EmbeddingModelConfig::builder()
            .model_name("different-model")
            .embedding_dim(512)
            .max_tokens(512)
            .overlap_tokens(50)
            .build();
        let result = KnowledgeIndex::open_with_config(temp_dir.path(), different_config);

        // Then It should fail with DatabaseAccessFailed (config mismatch)
        assert!(matches!(
            result,
            Err(IndexError::DatabaseAccessFailed { .. })
        ));
    }

    #[test]
    fn test_build_indexes_files_in_forest() {
        // Given A forest with markdown files
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "README.md",
            "# Test\n\nContent here",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Building the index
        let result = index.build().call();

        // Then It should succeed and report indexed files
        assert!(result.is_ok());
        let index_result = result.unwrap();
        assert!(index_result.files_processed > 0);
        assert!(index_result.chunks_affected > 0);
    }

    #[test]
    fn test_build_handles_empty_forest() {
        // Given An empty forest
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Building
        let result = index.build().call().unwrap();

        // Then It should succeed with zero files
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_affected, 0);
    }

    #[test]
    fn test_rebuild_clears_existing_chunks() {
        // Given An index with existing chunks
        let temp_dir = TempDir::new().unwrap();
        let _index = test_index_with_content(&temp_dir);

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Rebuilding
        let result = index.rebuild().call();

        // Then It should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_rebuild_reindexes_all_files() {
        // Given An index
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "test-repo", "test.md", "# Test\n\nContent");

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Rebuilding
        let result = index.rebuild().call().unwrap();

        // Then It should index files
        assert!(result.files_processed > 0);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_search_executes_query() {
        // Given An index with content
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "test.md",
            "# Boot Process\n\nHow boot works",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When Searching
        let result = index.search("boot", 10);

        // Then It should return results
        assert!(result.is_ok());
        let search_results = result.unwrap();
        assert!(!search_results.results.is_empty());
    }

    #[test]
    fn test_search_respects_limit() {
        // Given An index with multiple chunks
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "test.md",
            "# Test\n\ntest test test\n\n## Section\n\ntest test",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When Searching with limit 2
        let result = index.search("test", 2).unwrap();

        // Then Results should respect limit
        assert!(result.results.len() <= 2);
    }

    #[test]
    fn test_search_validates_limit_minimum() {
        // Given An index
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Searching with limit 0
        let result = index.search("test", 0);

        // Then It should fail with InvalidResultLimit
        assert!(matches!(result, Err(IndexError::InvalidResultLimit { .. })));
    }

    #[test]
    fn test_search_validates_limit_maximum() {
        // Given An index
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Searching with limit 101
        let result = index.search("test", 101);

        // Then It should fail with InvalidResultLimit
        assert!(matches!(result, Err(IndexError::InvalidResultLimit { .. })));
    }

    #[test]
    fn test_search_validates_empty_query() {
        // Given An index
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Searching with empty query
        let result = index.search("", 10);

        // Then It should fail with InvalidQuery
        assert!(matches!(result, Err(IndexError::InvalidQuery { .. })));
    }

    #[test]
    fn test_status_returns_index_metadata() {
        // Given An index with content
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);

        // When Getting status
        let result = index.status();

        // Then It should return status with metadata
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.exists);
        assert!(status.chunk_count > 0);
        assert!(status.file_count > 0);
        assert!(status.last_build.is_some());
    }

    #[test]
    fn test_status_on_empty_index() {
        // Given An empty index
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        // When Getting status
        let status = index.status().unwrap();

        // Then It should show zero counts
        assert!(status.exists);
        assert_eq!(status.chunk_count, 0);
        assert_eq!(status.file_count, 0);
    }

    #[test]
    fn test_status_includes_disk_size() {
        // Given An index with content
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);

        // When Getting status
        let status = index.status().unwrap();

        // Then It should include size
        assert!(status.size_bytes.is_some());
        assert!(status.size_bytes.unwrap() > 0);
    }

    #[test]
    fn test_default_db_path_returns_correct_path() {
        // Given A forest root
        let index_root = std::path::Path::new("/test/forest");

        // When Computing default db path
        let db_path = KnowledgeIndex::default_db_path(index_root);

        // Then It should be .crumbly/knowledge.db
        assert_eq!(db_path, index_root.join(".crumbly/knowledge.db"));
    }

    #[test]
    fn test_index_root_returns_correct_path() {
        // Given An index
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();
        let index = KnowledgeIndex::open(index_root).unwrap();

        // When Getting forest root
        let root = index.index_root();

        // Then It should match original path
        assert_eq!(root, index_root);
    }

    #[test]
    fn test_db_path_returns_correct_path() {
        // Given An index
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();
        let index = KnowledgeIndex::open(index_root).unwrap();

        // When Getting db path
        let db_path = index.db_path();

        // Then It should be .crumbly/knowledge.db
        assert_eq!(db_path, index_root.join(".crumbly/knowledge.db"));
    }

    #[test]
    fn test_config_returns_embedding_config() {
        // Given An index with custom config
        let temp_dir = TempDir::new().unwrap();
        let custom_config = EmbeddingModelConfig::builder()
            .model_name("test-model")
            .embedding_dim(384)
            .max_tokens(256)
            .overlap_tokens(38)
            .build();

        let index =
            KnowledgeIndex::open_with_config(temp_dir.path(), custom_config.clone()).unwrap();

        // When Getting config
        let config = index.config();

        // Then It should return the custom config
        assert_eq!(config, &custom_config);
    }

    #[test]
    fn test_load_scan_config_with_existing_crumbly_toml() {
        // Given A forest root with .crumbly.toml containing targets
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
targets = ["docs", "bottlerocket"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let default_context = ContextId::from_path(".").unwrap();

        // When Loading scan config for default context
        let scan_config = index
            .load_scan_config_for_context(&default_context)
            .unwrap();

        // Then It should return ScanConfig with those targets (not prefixed)
        assert_eq!(scan_config.targets.len(), 2);
        assert_eq!(scan_config.targets[0], std::path::PathBuf::from("docs"));
        assert_eq!(
            scan_config.targets[1],
            std::path::PathBuf::from("bottlerocket")
        );
    }

    #[test]
    fn test_load_scan_config_without_crumbly_toml() {
        // Given A forest root without .crumbly.toml
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let default_context = ContextId::from_path(".").unwrap();

        // When Loading scan config for default context
        let scan_config = index
            .load_scan_config_for_context(&default_context)
            .unwrap();

        // Then It should return ScanConfig with empty targets
        assert!(scan_config.targets.is_empty());
    }

    #[test]
    fn test_load_scan_config_prefixes_targets_for_non_default_context() {
        // Given A forest root with .crumbly.toml containing targets
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
targets = [".", "docs"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let context_id = ContextId::from_path("worktree/feature-a").unwrap();

        // When Loading scan config for a non-default context
        let scan_config = index.load_scan_config_for_context(&context_id).unwrap();

        // Then targets should be prefixed with context path (MCI-21)
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

    #[test]
    fn test_build_uses_configured_targets() {
        // Given A forest with .crumbly.toml specifying specific targets
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

        // When Building the index
        let result = index.build().call().unwrap();

        // Then It should only index files from configured targets
        assert_eq!(result.files_processed, 2);
        let status = index.status().unwrap();
        assert_eq!(status.file_count, 2);
    }

    #[test]
    fn test_rebuild_uses_configured_targets() {
        // Given A forest with configured targets
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

        // When Rebuilding
        let result = index.rebuild().call().unwrap();

        // Then It should only index configured targets
        assert_eq!(result.files_processed, 1);
    }
}

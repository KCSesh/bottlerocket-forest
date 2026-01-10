//! Tests for KnowledgeIndex methods in mod.rs

#[cfg(test)]
mod test {
    use crate::knowledge::EmbeddingModelConfig;
    use crate::knowledge::KnowledgeIndex;
    use crate::knowledge::facade::test_helpers::test_helpers::*;
    use crate::knowledge::facade::types::IndexError;

    use tempfile::TempDir;

    #[test]
    fn test_open_creates_crumbly_directory() {
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();
        let result = KnowledgeIndex::open(index_root);
        assert!(result.is_ok());
        assert!(index_root.join(".crumbly").exists());
    }

    #[test]
    fn test_open_does_not_create_database_file() {
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();
        let result = KnowledgeIndex::open(index_root);
        assert!(result.is_ok());
        assert!(!index_root.join(".crumbly/knowledge.db").exists());
    }

    #[test]
    fn test_open_with_nonexistent_index_root_fails() {
        let nonexistent = std::path::Path::new("/nonexistent/forest");
        let result = KnowledgeIndex::open(nonexistent);
        assert!(matches!(result, Err(IndexError::IndexRootNotFound { .. })));
    }

    #[test]
    fn test_open_returns_index_with_correct_paths() {
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();
        let index = KnowledgeIndex::open(index_root).unwrap();
        assert_eq!(index.index_root(), index_root);
        assert_eq!(index.db_path(), index_root.join(".crumbly/knowledge.db"));
    }

    #[test]
    fn test_open_with_existing_index_same_mode_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        let _index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = KnowledgeIndex::open(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_with_config_uses_custom_config() {
        let temp_dir = TempDir::new().unwrap();
        let custom_config = EmbeddingModelConfig::builder()
            .model_name("custom-model")
            .embedding_dim(512)
            .max_tokens(512)
            .overlap_tokens(50)
            .build();
        let index =
            KnowledgeIndex::open_with_config(temp_dir.path(), custom_config.clone()).unwrap();
        assert_eq!(index.config(), &custom_config);
    }

    #[test]
    fn test_open_with_config_validates_existing_config() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "test-repo", "test.md", "# Test");
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();
        let different_config = EmbeddingModelConfig::builder()
            .model_name("different-model")
            .embedding_dim(512)
            .max_tokens(512)
            .overlap_tokens(50)
            .build();
        let result = KnowledgeIndex::open_with_config(temp_dir.path(), different_config);
        assert!(matches!(
            result,
            Err(IndexError::DatabaseAccessFailed { .. })
        ));
    }

    #[test]
    fn test_default_db_path_returns_correct_path() {
        let index_root = std::path::Path::new("/test/forest");
        let db_path = KnowledgeIndex::default_db_path(index_root);
        assert_eq!(db_path, index_root.join(".crumbly/knowledge.db"));
    }

    #[test]
    fn test_index_root_returns_correct_path() {
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();
        let index = KnowledgeIndex::open(index_root).unwrap();
        let root = index.index_root();
        assert_eq!(root, index_root);
    }

    #[test]
    fn test_db_path_returns_correct_path() {
        let temp_dir = TempDir::new().unwrap();
        let index_root = temp_dir.path();
        let index = KnowledgeIndex::open(index_root).unwrap();
        let db_path = index.db_path();
        assert_eq!(db_path, index_root.join(".crumbly/knowledge.db"));
    }

    #[test]
    fn test_config_returns_embedding_config() {
        let temp_dir = TempDir::new().unwrap();
        let custom_config = EmbeddingModelConfig::builder()
            .model_name("test-model")
            .embedding_dim(384)
            .max_tokens(256)
            .overlap_tokens(38)
            .build();
        let index =
            KnowledgeIndex::open_with_config(temp_dir.path(), custom_config.clone()).unwrap();
        let config = index.config();
        assert_eq!(config, &custom_config);
    }

    #[test]
    fn test_search_executes_query() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "test.md",
            "# Boot Process\n\nHow boot works",
        );
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();
        let result = index.search("boot", 10);
        assert!(result.is_ok());
        let search_results = result.unwrap();
        assert!(!search_results.results.is_empty());
    }

    #[test]
    fn test_search_respects_limit() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "test.md",
            "# Test\n\ntest test test\n\n## Section\n\ntest test",
        );
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();
        let result = index.search("test", 2).unwrap();
        assert!(result.results.len() <= 2);
    }

    #[test]
    fn test_search_validates_limit_minimum() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.search("test", 0);
        assert!(matches!(result, Err(IndexError::InvalidResultLimit { .. })));
    }

    #[test]
    fn test_search_validates_limit_maximum() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.search("test", 101);
        assert!(matches!(result, Err(IndexError::InvalidResultLimit { .. })));
    }

    #[test]
    fn test_search_validates_empty_query() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.search("", 10);
        assert!(matches!(result, Err(IndexError::InvalidQuery { .. })));
    }

    #[test]
    fn test_status_returns_index_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);
        let result = index.status();
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.exists);
        assert!(status.chunk_count > 0);
        assert!(status.file_count > 0);
        assert!(status.last_build.is_some());
    }

    #[test]
    fn test_status_on_empty_index() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();
        let status = index.status().unwrap();
        assert!(status.exists);
        assert_eq!(status.chunk_count, 0);
        assert_eq!(status.file_count, 0);
    }

    #[test]
    fn test_status_includes_disk_size() {
        let temp_dir = TempDir::new().unwrap();
        let index = test_index_with_content(&temp_dir);
        let status = index.status().unwrap();
        assert!(status.size_bytes.is_some());
        assert!(status.size_bytes.unwrap() > 0);
    }
}

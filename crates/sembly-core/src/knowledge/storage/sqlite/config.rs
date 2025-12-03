//! Configuration validation tests for SQLite storage.

use super::*;
use std::time::SystemTime;
use tempfile::NamedTempFile;

#[test]
fn test_metadata_model_config_roundtrip() {
    // Given A repository with custom model config
    let temp_file = NamedTempFile::new().unwrap();
    let mut repo =
        SqliteChunkRepository::open(temp_file.path(), &super::test::test_config()).unwrap();

    let custom_config = EmbeddingModelConfig::builder()
        .model_name("custom-model")
        .embedding_dim(512)
        .max_tokens(128)
        .overlap_tokens(20)
        .build();

    let metadata = IndexMetadata::builder()
        .last_build(SystemTime::now())
        .chunk_count(42)
        .file_count(7)
        .model_config(custom_config.clone())
        .build();

    // When Setting and retrieving metadata
    repo.set_metadata(&metadata).unwrap();
    let retrieved = repo.get_metadata().unwrap();

    // Then All model config fields are preserved
    assert_eq!(retrieved.model_config.model_name, custom_config.model_name);
    assert_eq!(
        retrieved.model_config.embedding_dim,
        custom_config.embedding_dim
    );
    assert_eq!(retrieved.model_config.max_tokens, custom_config.max_tokens);
    assert_eq!(
        retrieved.model_config.overlap_tokens,
        custom_config.overlap_tokens
    );
}

#[test]
fn test_metadata_default_model_config() {
    // Given A repository with default model config
    let temp_file = NamedTempFile::new().unwrap();
    let mut repo =
        SqliteChunkRepository::open(temp_file.path(), &super::test::test_config()).unwrap();

    let metadata = IndexMetadata::builder()
        .last_build(SystemTime::now())
        .chunk_count(0)
        .file_count(0)
        .model_config(EmbeddingModelConfig::default())
        .build();

    // When Setting and retrieving metadata
    repo.set_metadata(&metadata).unwrap();
    let retrieved = repo.get_metadata().unwrap();

    // Then Default config values are preserved
    assert_eq!(
        retrieved.model_config.model_name,
        "sentence-transformers/all-MiniLM-L6-v2"
    );
    assert_eq!(retrieved.model_config.embedding_dim, 384);
    assert_eq!(retrieved.model_config.max_tokens, 256);
    assert_eq!(retrieved.model_config.overlap_tokens, 38);
}

#[test]
fn test_metadata_persists_across_reopens() {
    // Given A repository with metadata
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let custom_config = EmbeddingModelConfig::builder()
        .model_name("test-model")
        .embedding_dim(256)
        .max_tokens(512)
        .overlap_tokens(64)
        .build();

    {
        let mut repo =
            SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(100)
            .file_count(10)
            .model_config(custom_config.clone())
            .build();

        repo.set_metadata(&metadata).unwrap();
    }

    // When Reopening the database
    let repo = SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
    let retrieved = repo.get_metadata().unwrap();

    // Then Model config is still present
    assert_eq!(retrieved.model_config, custom_config);
}

#[test]
fn test_open_with_matching_config_succeeds() {
    // Given An existing index with specific model config
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let config = EmbeddingModelConfig::builder()
        .model_name("test-model")
        .embedding_dim(256)
        .max_tokens(512)
        .overlap_tokens(64)
        .build();

    {
        let mut repo =
            SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(config.clone())
            .build();
        repo.set_metadata(&metadata).unwrap();
    }

    // When Opening with matching config
    let result = SqliteChunkRepository::open_with_config(&temp_path, &config);

    // Then It should succeed
    assert!(result.is_ok());
}

#[test]
fn test_open_with_mismatched_model_name_fails() {
    // Given An existing index with one model
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let stored_config = EmbeddingModelConfig::builder()
        .model_name("model-a")
        .embedding_dim(384)
        .max_tokens(256)
        .overlap_tokens(38)
        .build();

    {
        let mut repo =
            SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(stored_config)
            .build();
        repo.set_metadata(&metadata).unwrap();
    }

    // When Opening with different model name
    let expected_config = EmbeddingModelConfig::builder()
        .model_name("model-b")
        .embedding_dim(384)
        .max_tokens(256)
        .overlap_tokens(38)
        .build();

    let result = SqliteChunkRepository::open_with_config(&temp_path, &expected_config);

    // Then It should fail with ConfigMismatch error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, StorageError::ConfigMismatch { .. }));
}

#[test]
fn test_open_with_mismatched_embedding_dim_fails() {
    // Given An existing index with one embedding dimension
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let stored_config = EmbeddingModelConfig::builder()
        .model_name("test-model")
        .embedding_dim(384)
        .max_tokens(256)
        .overlap_tokens(38)
        .build();

    {
        let mut repo =
            SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(stored_config)
            .build();
        repo.set_metadata(&metadata).unwrap();
    }

    // When Opening with different embedding dimension
    let expected_config = EmbeddingModelConfig::builder()
        .model_name("test-model")
        .embedding_dim(512)
        .max_tokens(256)
        .overlap_tokens(38)
        .build();

    let result = SqliteChunkRepository::open_with_config(&temp_path, &expected_config);

    // Then It should fail with ConfigMismatch error
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        StorageError::ConfigMismatch { .. }
    ));
}

#[test]
fn test_open_with_mismatched_max_tokens_fails() {
    // Given An existing index with one max_tokens value
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let stored_config = EmbeddingModelConfig::builder()
        .model_name("test-model")
        .embedding_dim(384)
        .max_tokens(256)
        .overlap_tokens(38)
        .build();

    {
        let mut repo =
            SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(stored_config)
            .build();
        repo.set_metadata(&metadata).unwrap();
    }

    // When Opening with different max_tokens
    let expected_config = EmbeddingModelConfig::builder()
        .model_name("test-model")
        .embedding_dim(384)
        .max_tokens(512)
        .overlap_tokens(38)
        .build();

    let result = SqliteChunkRepository::open_with_config(&temp_path, &expected_config);

    // Then It should fail with ConfigMismatch error
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        StorageError::ConfigMismatch { .. }
    ));
}

#[test]
fn test_open_with_mismatched_overlap_tokens_fails() {
    // Given An existing index with one overlap_tokens value
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let stored_config = EmbeddingModelConfig::builder()
        .model_name("test-model")
        .embedding_dim(384)
        .max_tokens(256)
        .overlap_tokens(38)
        .build();

    {
        let mut repo =
            SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(stored_config)
            .build();
        repo.set_metadata(&metadata).unwrap();
    }

    // When Opening with different overlap_tokens
    let expected_config = EmbeddingModelConfig::builder()
        .model_name("test-model")
        .embedding_dim(384)
        .max_tokens(256)
        .overlap_tokens(64)
        .build();

    let result = SqliteChunkRepository::open_with_config(&temp_path, &expected_config);

    // Then It should fail with ConfigMismatch error
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        StorageError::ConfigMismatch { .. }
    ));
}

#[test]
fn test_config_mismatch_error_message_includes_details() {
    // Given An existing index with specific config
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let stored_config = EmbeddingModelConfig::builder()
        .model_name("old-model")
        .embedding_dim(256)
        .max_tokens(128)
        .overlap_tokens(20)
        .build();

    {
        let mut repo =
            SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(stored_config)
            .build();
        repo.set_metadata(&metadata).unwrap();
    }

    // When Opening with completely different config
    let expected_config = EmbeddingModelConfig::builder()
        .model_name("new-model")
        .embedding_dim(512)
        .max_tokens(256)
        .overlap_tokens(40)
        .build();

    let result = SqliteChunkRepository::open_with_config(&temp_path, &expected_config);

    // Then Error message should contain expected and actual values
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("old-model") || err_msg.contains("new-model"));
}

#[test]
fn test_open_without_config_validation_still_works() {
    // Given An existing index with any config
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    {
        let mut repo =
            SqliteChunkRepository::open(&temp_path, &super::test::test_config()).unwrap();
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(EmbeddingModelConfig::default())
            .build();
        repo.set_metadata(&metadata).unwrap();
    }

    // When Opening without config validation
    let result = SqliteChunkRepository::open(&temp_path, &super::test::test_config());

    // Then It should succeed regardless of stored config
    assert!(result.is_ok());
}

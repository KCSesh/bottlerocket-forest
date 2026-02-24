//! Tests for indexing strategies
//!
//! Extracted from strategies.rs to comply with LOC limits.

use super::*;
use crate::knowledge::domain::{ContextId, Embedding, IndexRelativePath, Timestamp};
use crate::knowledge::indexing::provider::MockIndexDataProvider;
use crate::knowledge::storage::StorageError;
use crate::knowledge::storage::repository::MockChunkRepository;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use tempfile::TempDir;

fn test_embedding() -> Embedding {
    Embedding::try_new(vec![0.1; 384]).unwrap()
}

fn mock_provider_success() -> MockIndexDataProvider {
    let mut p = MockIndexDataProvider::new();
    p.expect_generate_batch_with_progress()
        .returning(|_, _| Ok(vec![test_embedding()]));
    p
}

fn setup_mock_repo_for_build() -> MockChunkRepository {
    let mut r = MockChunkRepository::new();
    r.expect_spawn().times(0..).returning(|| {
        let mut spawned = MockChunkRepository::new();
        spawned.expect_save_batch().returning(|_| Ok(()));
        spawned
            .expect_track_indexed_file()
            .returning(|_, _, _, _| Ok(()));
        spawned
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        Ok(spawned)
    });
    r.expect_has_embedding_batch()
        .returning(|_| Ok(HashSet::new()));
    r.expect_track_indexed_file().returning(|_, _, _, _| Ok(()));
    r.expect_save_batch().returning(|_| Ok(()));
    r
}

fn setup_mock_repo_for_incremental(
    indexed: HashMap<IndexRelativePath, Timestamp>,
) -> MockChunkRepository {
    let mut r = MockChunkRepository::new();
    r.expect_spawn().times(0..).returning(|| {
        let mut spawned = MockChunkRepository::new();
        spawned.expect_save_batch().returning(|_| Ok(()));
        spawned
            .expect_track_indexed_file()
            .returning(|_, _, _, _| Ok(()));
        spawned
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        Ok(spawned)
    });
    r.expect_get_indexed_files()
        .returning(move |_| Ok(indexed.clone()));
    r.expect_has_embedding_batch()
        .returning(|_| Ok(HashSet::new()));
    r.expect_remove_indexed_file_from_context()
        .returning(|_, _| Ok(()));
    r.expect_track_indexed_file().returning(|_, _, _, _| Ok(()));
    r.expect_save_batch().returning(|_| Ok(()));
    r
}

fn create_test_indexer(
    temp_dir: &TempDir,
    repo: MockChunkRepository,
    provider: MockIndexDataProvider,
) -> Indexer<MockChunkRepository> {
    let config = EmbeddingModelConfig::default();
    let context_id = ContextId::from_path(".").unwrap();
    Indexer::builder()
        .index_root(temp_dir.path())
        .repository(repo)
        .config(&config)
        .provider(Arc::new(provider))
        .scan_config(ScanConfig::default())
        .filter(IndexingFilter::default())
        .context_id(context_id)
        .build()
        .unwrap()
}

fn setup_test_file(base: &TempDir, rel_path: &str, content: &str) {
    let full_path = base.path().join(rel_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full_path, content).unwrap();
}

fn setup_test_files(base: &TempDir, files: &[(&str, &str)]) {
    for (path, content) in files {
        setup_test_file(base, path, content);
    }
}

#[test]
fn test_new_creates_indexer_with_valid_forest() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path().join("test-repo");
    fs::create_dir(&repo_dir).unwrap();
    let mock_repo = MockChunkRepository::new();
    let mock_provider = MockIndexDataProvider::new();
    let _indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
}

#[test]
fn test_new_fails_with_invalid_forest() {
    let nonexistent = Path::new("/nonexistent/forest");
    let mock_repo = MockChunkRepository::new();
    let config = EmbeddingModelConfig::default();
    let mock_provider = MockIndexDataProvider::new();
    let context_id = ContextId::from_path(".").unwrap();
    let result = Indexer::builder()
        .index_root(nonexistent)
        .repository(mock_repo)
        .config(&config)
        .provider(Arc::new(mock_provider))
        .scan_config(ScanConfig::default())
        .filter(IndexingFilter::default())
        .context_id(context_id)
        .build();
    assert!(matches!(result, Err(IndexingError::ScanFailed { .. })));
}

#[test_case::test_case(&[("test-repo/README.md", "# Test\nContent here")]; "markdown")]
#[test_case::test_case(&[("test-repo/src/lib.rs", "/// Documentation\npub fn test() {}")]; "rust")]
fn test_build_indexes_files(files: &[(&str, &str)]) {
    let temp_dir = TempDir::new().unwrap();
    setup_test_files(&temp_dir, files);
    let mock_repo = setup_mock_repo_for_build();
    let mock_provider = mock_provider_success();
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Build);
    assert!(result.is_ok());
    let index_result = result.unwrap();
    assert_eq!(index_result.files_added, files.len());
    assert_eq!(index_result.files_processed, files.len());
    assert!(index_result.chunks_affected > 0);
}

#[test]
fn test_build_calls_provider_generate() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_file(&temp_dir, "test-repo/test.md", "# Test\n\nContent");
    let mock_repo = setup_mock_repo_for_build();
    let mut mock_provider = MockIndexDataProvider::new();
    mock_provider
        .expect_generate_batch_with_progress()
        .times(1)
        .returning(|_, _| Ok(vec![test_embedding()]));
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Build);
    assert!(result.is_ok());
}

#[test]
fn test_build_propagates_provider_error() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_file(&temp_dir, "test-repo/test.md", "# Test\n\nContent");
    let mut mock_provider = MockIndexDataProvider::new();
    mock_provider
        .expect_generate_batch_with_progress()
        .returning(|_, _| {
            Err(
                crate::knowledge::indexing::IndexDataError::EmbeddingFailed {
                    source: Box::new(std::io::Error::other("test error")),
                },
            )
        });
    let mock_repo = setup_mock_repo_for_build();
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Build);
    assert!(matches!(
        result,
        Err(IndexingError::IndexDataGenerationFailed { .. })
    ));
}

#[test]
fn test_build_propagates_storage_error() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_file(&temp_dir, "test-repo/test.md", "# Test\n\nContent");
    let mut mock_repo = MockChunkRepository::new();
    mock_repo.expect_spawn().times(0..).returning(|| {
        let mut spawned = MockChunkRepository::new();
        spawned.expect_save_batch().returning(|_| {
            Err(StorageError::InvalidData {
                message: "test error".to_string(),
            })
        });
        spawned
            .expect_track_indexed_file()
            .returning(|_, _, _, _| Ok(()));
        spawned
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        Ok(spawned)
    });
    mock_repo
        .expect_has_embedding_batch()
        .returning(|_| Ok(HashSet::new()));
    mock_repo
        .expect_track_indexed_file()
        .returning(|_, _, _, _| Ok(()));
    mock_repo.expect_save_batch().returning(|_| Ok(()));
    let mock_provider = mock_provider_success();
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Build);
    assert!(matches!(result, Err(IndexingError::StorageFailed { .. })));
}

#[test]
fn test_build_handles_empty_forest() {
    let temp_dir = TempDir::new().unwrap();
    let mut mock_repo = MockChunkRepository::new();
    mock_repo.expect_save_batch().times(0);
    let mock_provider = MockIndexDataProvider::new();
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Build).unwrap();
    assert_eq!(result.files_added, 0);
    assert_eq!(result.files_processed, 0);
    assert_eq!(result.chunks_affected, 0);
}

#[test]
fn test_rebuild_clears_then_builds() {
    let temp_dir = TempDir::new().unwrap();
    setup_test_file(&temp_dir, "test-repo/test.md", "# Test\n\nContent");
    let mut mock_repo = MockChunkRepository::new();
    mock_repo.expect_clear().times(1).returning(|| Ok(5));
    mock_repo.expect_spawn().times(0..).returning(|| {
        let mut spawned = MockChunkRepository::new();
        spawned.expect_save_batch().returning(|_| Ok(()));
        spawned
            .expect_track_indexed_file()
            .returning(|_, _, _, _| Ok(()));
        spawned
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        Ok(spawned)
    });
    mock_repo
        .expect_has_embedding_batch()
        .returning(|_| Ok(HashSet::new()));
    mock_repo
        .expect_track_indexed_file()
        .returning(|_, _, _, _| Ok(()));
    mock_repo.expect_save_batch().returning(|_| Ok(()));
    let mock_provider = mock_provider_success();
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Rebuild);
    assert!(result.is_ok());
}

#[test_case::test_case(&[("test-repo/new.md", "# New\n\nContent")], &[], 1, 0, 0, 1; "adds_new_files")]
#[test_case::test_case(&[("test-repo/modified.md", "# Modified\n\nNew content")], &["test-repo/modified.md"], 0, 1, 0, 1; "modifies_changed_files")]
#[test_case::test_case(&[], &["test-repo/deleted.md"], 0, 0, 1, 0; "removes_deleted_files")]
#[test_case::test_case(&[("test-repo/new.md", "# New\n\nContent"), ("test-repo/modified.md", "# Modified\n\nContent")], &["test-repo/modified.md", "test-repo/deleted.md"], 1, 1, 1, 2; "handles_mixed_changes")]
fn test_incremental_operations(
    files: &[(&str, &str)],
    indexed_paths: &[&str],
    exp_added: usize,
    exp_updated: usize,
    exp_removed: usize,
    exp_processed: usize,
) {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path().join("test-repo");
    fs::create_dir(&repo_dir).unwrap();
    setup_test_files(&temp_dir, files);
    let mut indexed = HashMap::new();
    for path in indexed_paths {
        indexed.insert(
            IndexRelativePath::try_new(*path).unwrap(),
            Timestamp::from_secs(1000),
        );
    }
    let mock_repo = setup_mock_repo_for_incremental(indexed);
    let mock_provider = mock_provider_success();
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Incremental).unwrap();
    assert_eq!(result.files_added, exp_added);
    assert_eq!(result.files_updated, exp_updated);
    assert_eq!(result.files_removed, exp_removed);
    assert_eq!(result.files_processed, exp_processed);
}

#[test]
fn test_incremental_propagates_storage_errors() {
    let temp_dir = TempDir::new().unwrap();
    let repo_dir = temp_dir.path().join("test-repo");
    fs::create_dir(&repo_dir).unwrap();
    let mut mock_repo = MockChunkRepository::new();
    mock_repo.expect_get_indexed_files().returning(|_| {
        Err(StorageError::InvalidData {
            message: "test error".to_string(),
        })
    });
    let mock_provider = MockIndexDataProvider::new();
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Incremental);
    assert!(matches!(result, Err(IndexingError::StorageFailed { .. })));
}

#[test]
fn test_incremental_handles_empty_forest() {
    let temp_dir = TempDir::new().unwrap();
    let mut mock_repo = MockChunkRepository::new();
    mock_repo
        .expect_get_indexed_files()
        .returning(|_| Ok(HashMap::new()));
    let mock_provider = MockIndexDataProvider::new();
    let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);
    let result = indexer.index(IndexStrategy::Incremental).unwrap();
    assert_eq!(result.files_added, 0);
    assert_eq!(result.files_updated, 0);
    assert_eq!(result.files_removed, 0);
    assert_eq!(result.files_processed, 0);
    assert_eq!(result.chunks_affected, 0);
}

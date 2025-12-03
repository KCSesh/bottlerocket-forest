//! Embedding batch operation tests for SQLite storage.

use super::*;
use crate::knowledge::Embedding;
use tempfile::NamedTempFile;

#[test]
fn test_save_batch_with_embeddings() {
    // Given A repository and chunks with embeddings
    let temp_file = NamedTempFile::new().unwrap();
    let mut repo =
        SqliteChunkRepository::open(temp_file.path(), &super::test::test_config()).unwrap();

    let indexed_chunk1 = super::test::create_test_chunk_at("test1.md", "test-repo");
    let indexed_chunk2 = super::test::create_test_chunk_at("test2.md", "test-repo");

    // When Saving in batch
    repo.save_batch(&[indexed_chunk1, indexed_chunk2]).unwrap();

    // Then Both should be in vec_chunks
    let count: i64 = repo
        .conn
        .query_row("SELECT COUNT(*) FROM vec_chunks", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_zero_embedding() {
    // Given An attempt to create an empty embedding
    let empty_embedding = Embedding::try_new(vec![]);

    // When Creating the embedding
    // Then It should fail validation at the type level
    assert!(empty_embedding.is_err());
}

#[test]
fn test_clear_removes_all_embeddings() {
    // Given A repository with multiple Best mode chunks
    let temp_file = NamedTempFile::new().unwrap();
    let mut repo =
        SqliteChunkRepository::open(temp_file.path(), &super::test::test_config()).unwrap();

    let chunk1 = super::test::create_test_chunk_at("file1.md", "repo1");
    let chunk2 = super::test::create_test_chunk_at("file2.md", "repo2");
    repo.save(&chunk1).unwrap();
    repo.save(&chunk2).unwrap();

    let vec_count_before: i64 = repo
        .conn
        .query_row("SELECT COUNT(*) FROM vec_chunks", [], |row| row.get(0))
        .unwrap();
    assert_eq!(vec_count_before, 2);

    // When Clearing all chunks
    repo.clear().unwrap();

    // Then All embeddings should be deleted from vec_chunks
    let vec_count_after: i64 = repo
        .conn
        .query_row("SELECT COUNT(*) FROM vec_chunks", [], |row| row.get(0))
        .unwrap();
    assert_eq!(vec_count_after, 0);
}

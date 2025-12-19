//! Storage queries for content-addressed chunks.
//!
//! Provides CRUD operations for Chunk records using chunk_hash as the
//! primary key for content-addressed storage. This enables deduplication
//! across contexts - identical content shares embeddings regardless of
//! which context it appears in.

use rusqlite::Connection;
use snafu::ResultExt;

use crate::knowledge::domain::{ChunkHash, FileHash, IndexedChunk};
use crate::knowledge::storage::repository::{StorageError, storage_error::*};
use super::serialization::{indexed_chunk_from_row, serialize_context};

pub fn save_chunk(conn: &Connection, chunk: &IndexedChunk) -> Result<(), StorageError> {
  let (context_type, context_data) = serialize_context(&chunk.chunk.context)?;
  conn.execute(
    "INSERT OR REPLACE INTO chunks 
    (chunk_hash, file_hash, repo_name, context_type, context_data, content, token_count, last_modified)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    rusqlite::params![
      chunk.chunk.chunk_hash.as_bytes(),
      chunk.chunk.file_hash.as_bytes(),
      chunk.chunk.source.repo_name.to_string(),
      context_type,
      context_data,
      chunk.chunk.content.text,
      chunk.chunk.content.token_count.into_inner() as i64,
      chunk.indexed_at.as_secs(),
    ],
  ).context(DatabaseSnafu)?;
  Ok(())
}

pub fn get_chunks_by_file_hash(
  conn: &Connection,
  file_hash: &FileHash,
) -> Result<Vec<IndexedChunk>, StorageError> {
  let mut stmt = conn.prepare(
    "SELECT chunk_hash, chunk_hash, file_hash, '', repo_name, context_type, context_data, 
            content, token_count, last_modified
     FROM chunks
     WHERE file_hash = ?1",
  ).context(DatabaseSnafu)?;
  stmt.query_map([file_hash.as_bytes()], |row| {
    indexed_chunk_from_row(row)
      .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
  })
  .context(DatabaseSnafu)?
  .collect::<Result<Vec<_>, _>>()
  .context(DatabaseSnafu)
}

pub fn has_chunk(conn: &Connection, chunk_hash: &ChunkHash) -> Result<bool, StorageError> {
  let count: i64 = conn.query_row(
    "SELECT COUNT(*) FROM chunks WHERE chunk_hash = ?1",
    [chunk_hash.as_bytes()],
    |row| row.get(0),
  ).context(DatabaseSnafu)?;
  Ok(count > 0)
}

pub fn delete_orphaned_chunks(conn: &Connection) -> Result<u64, StorageError> {
  let deleted_chunks = conn.execute(
    "DELETE FROM chunks WHERE file_hash NOT IN (SELECT DISTINCT file_hash FROM indexed_files)",
    [],
  ).context(DatabaseSnafu)?;
  conn.execute(
    "DELETE FROM vec_chunks WHERE chunk_hash NOT IN (SELECT chunk_hash FROM chunks)",
    [],
  ).context(DatabaseSnafu)?;
  Ok(deleted_chunks as u64)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::domain::{
        Chunk, ChunkContent, ChunkContext, ChunkId, ChunkSource, Embedding, EmbeddingModelConfig,
        HeadingText, IndexRelativePath, MarkdownContext, RepoName, Timestamp, TokenCount,
    };
    use crate::knowledge::storage::schema;
    use rusqlite::Connection;
    use std::io::Cursor;
    use uuid::Uuid;

    fn setup_connection() -> Connection {
        // SAFETY: This call satisfies the safety requirements for sqlite3_auto_extension:
        // 1. We are not calling this from within an auto-extension handler
        // 2. We will not close any database connection from within the auto-extension
        // 3. We will not manipulate the auto-extension list from within an auto-extension
        // 4. sqlite3_vec_init is a valid C function pointer provided by the sqlite-vec crate
        // 5. The transmute is valid because both types are function pointers with the same size
        #[allow(clippy::missing_transmute_annotations)]
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn, &EmbeddingModelConfig::default()).unwrap();
        conn
    }

    fn make_file_hash(content: &[u8]) -> FileHash {
        FileHash::from_reader(Cursor::new(content)).unwrap()
    }

    fn make_chunk(text: &str) -> Chunk {
        Chunk::builder()
            .id(ChunkId::new(Uuid::new_v4()))
            .chunk_hash(ChunkHash::from_text(text))
            .file_hash(make_file_hash(text.as_bytes()))
            .source(
                ChunkSource::builder()
                    .file_path(IndexRelativePath::try_new("test.md").unwrap())
                    .repo_name(RepoName::try_new("test-repo").unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(text)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(ChunkContext::Markdown(
                MarkdownContext::builder()
                    .heading_hierarchy(vec![HeadingText::try_new("Test").unwrap()])
                    .build(),
            ))
            .build()
    }

    fn make_indexed_chunk(text: &str) -> IndexedChunk {
        IndexedChunk::builder()
            .chunk(make_chunk(text))
            .embedding(Embedding::try_new(vec![0.1, 0.2, 0.3]).unwrap())
            .indexed_at(Timestamp::from_secs(1700000000))
            .build()
    }

    fn make_indexed_chunk_with_file_hash(text: &str, file_hash: FileHash) -> IndexedChunk {
        let chunk = Chunk::builder()
            .id(ChunkId::new(Uuid::new_v4()))
            .chunk_hash(ChunkHash::from_text(text))
            .file_hash(file_hash)
            .source(
                ChunkSource::builder()
                    .file_path(IndexRelativePath::try_new("test.md").unwrap())
                    .repo_name(RepoName::try_new("test-repo").unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(text)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(ChunkContext::Markdown(
                MarkdownContext::builder()
                    .heading_hierarchy(vec![HeadingText::try_new("Test").unwrap()])
                    .build(),
            ))
            .build();

        IndexedChunk::builder()
            .chunk(chunk)
            .embedding(Embedding::try_new(vec![0.1, 0.2, 0.3]).unwrap())
            .indexed_at(Timestamp::from_secs(1700000000))
            .build()
    }

    #[test]
    fn save_chunk_and_retrieve_by_file_hash() {
        // Given a database and an indexed chunk
        let conn = setup_connection();
        let indexed_chunk = make_indexed_chunk("This is test content");
        let file_hash = indexed_chunk.chunk.file_hash;

        // When saving the chunk
        save_chunk(&conn, &indexed_chunk).unwrap();

        // Then it should be retrievable by file_hash
        let chunks = get_chunks_by_file_hash(&conn, &file_hash).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk.chunk_hash, indexed_chunk.chunk.chunk_hash);
        assert_eq!(chunks[0].chunk.content.text, "This is test content");
    }

    #[test]
    fn save_duplicate_chunk_is_idempotent() {
        // Given a database with an existing chunk
        let conn = setup_connection();
        let indexed_chunk = make_indexed_chunk("Duplicate content");

        // When saving the same chunk twice
        save_chunk(&conn, &indexed_chunk).unwrap();
        let result = save_chunk(&conn, &indexed_chunk);

        // Then the second save should succeed (idempotent)
        assert!(result.is_ok());

        // And only one chunk should exist
        let chunks = get_chunks_by_file_hash(&conn, &indexed_chunk.chunk.file_hash).unwrap();
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn get_chunks_by_file_hash_returns_correct_set() {
        // Given a database with multiple chunks from the same file
        let conn = setup_connection();
        let file_hash = make_file_hash(b"shared file content");

        let chunk1 = make_indexed_chunk_with_file_hash("First chunk of the file", file_hash);
        let chunk2 = make_indexed_chunk_with_file_hash("Second chunk of the file", file_hash);
        let chunk3 = make_indexed_chunk_with_file_hash("Third chunk of the file", file_hash);

        // And a chunk from a different file
        let other_file_hash = make_file_hash(b"different file");
        let other_chunk =
            make_indexed_chunk_with_file_hash("Chunk from other file", other_file_hash);

        save_chunk(&conn, &chunk1).unwrap();
        save_chunk(&conn, &chunk2).unwrap();
        save_chunk(&conn, &chunk3).unwrap();
        save_chunk(&conn, &other_chunk).unwrap();

        // When retrieving chunks by file_hash
        let chunks = get_chunks_by_file_hash(&conn, &file_hash).unwrap();

        // Then only chunks from that file should be returned
        assert_eq!(chunks.len(), 3);
        let texts: Vec<_> = chunks
            .iter()
            .map(|c| c.chunk.content.text.as_str())
            .collect();
        assert!(texts.contains(&"First chunk of the file"));
        assert!(texts.contains(&"Second chunk of the file"));
        assert!(texts.contains(&"Third chunk of the file"));
        assert!(!texts.contains(&"Chunk from other file"));
    }

    #[test]
    fn has_chunk_returns_true_for_existing() {
        // Given a database with a saved chunk
        let conn = setup_connection();
        let indexed_chunk = make_indexed_chunk("Existing chunk content");
        save_chunk(&conn, &indexed_chunk).unwrap();

        // When checking if the chunk exists
        let exists = has_chunk(&conn, &indexed_chunk.chunk.chunk_hash).unwrap();

        // Then it should return true
        assert!(exists);
    }

    #[test]
    fn has_chunk_returns_false_for_missing() {
        // Given a database with no chunks
        let conn = setup_connection();
        let nonexistent_hash = ChunkHash::from_text("content that was never saved");

        // When checking if a non-existent chunk exists
        let exists = has_chunk(&conn, &nonexistent_hash).unwrap();

        // Then it should return false
        assert!(!exists);
    }

    #[test]
    fn delete_orphaned_chunks_returns_zero_on_empty_database() {
        // Given an empty database with no chunks
        let conn = setup_connection();

        // When running garbage collection
        let result = delete_orphaned_chunks(&conn);

        // Then it should return zero deleted chunks
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn delete_orphaned_chunks_returns_zero_when_all_chunks_referenced() {
        // Given a database with chunks that are all referenced by indexed_files
        let conn = setup_connection();
        let indexed_chunk = make_indexed_chunk("Referenced content");
        save_chunk(&conn, &indexed_chunk).unwrap();

        // And the chunk's file_hash is in indexed_files
        conn.execute(
            "INSERT INTO indexed_files (context_id, file_path, file_hash, mtime_ns) VALUES (?, ?, ?, ?)",
            rusqlite::params![
                ".",
                "test.md",
                indexed_chunk.chunk.file_hash.as_bytes().as_slice(),
                1700000000_i64 * 1_000_000_000,
            ],
        ).unwrap();

        // When running garbage collection
        let result = delete_orphaned_chunks(&conn);

        // Then it should return zero (no orphans)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // And the chunk should still exist
        assert!(has_chunk(&conn, &indexed_chunk.chunk.chunk_hash).unwrap());
    }

    #[test]
    fn delete_orphaned_chunks_deletes_unreferenced_chunks() {
        // Given a database with a chunk not referenced by any indexed_files
        let conn = setup_connection();
        let orphan_chunk = make_indexed_chunk("Orphaned content");
        save_chunk(&conn, &orphan_chunk).unwrap();

        // And no entry in indexed_files for this file_hash

        // When running garbage collection
        let result = delete_orphaned_chunks(&conn);

        // Then it should return 1 (one orphan deleted)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // And the chunk should no longer exist
        assert!(!has_chunk(&conn, &orphan_chunk.chunk.chunk_hash).unwrap());
    }

    #[test]
    fn delete_orphaned_chunks_preserves_shared_content() {
        // Given a database with a chunk referenced by one context
        let conn = setup_connection();
        let file_hash = make_file_hash(b"shared file content");
        let chunk = make_indexed_chunk_with_file_hash("Shared content", file_hash);
        save_chunk(&conn, &chunk).unwrap();

        // And two contexts reference the same file_hash
        conn.execute(
            "INSERT INTO indexed_files (context_id, file_path, file_hash, mtime_ns) VALUES (?, ?, ?, ?)",
            rusqlite::params!["context-a", "file.md", file_hash.as_bytes().as_slice(), 1700000000_i64 * 1_000_000_000],
        ).unwrap();
        conn.execute(
            "INSERT INTO indexed_files (context_id, file_path, file_hash, mtime_ns) VALUES (?, ?, ?, ?)",
            rusqlite::params!["context-b", "file.md", file_hash.as_bytes().as_slice(), 1700000000_i64 * 1_000_000_000],
        ).unwrap();

        // When removing one context's reference
        conn.execute(
            "DELETE FROM indexed_files WHERE context_id = ?",
            ["context-a"],
        )
        .unwrap();

        // And running garbage collection
        let result = delete_orphaned_chunks(&conn);

        // Then it should return zero (chunk still referenced by context-b)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // And the chunk should still exist
        assert!(has_chunk(&conn, &chunk.chunk.chunk_hash).unwrap());
    }

    #[test]
    fn delete_orphaned_chunks_deletes_after_all_references_removed() {
        // Given a database with a chunk referenced by two contexts
        let conn = setup_connection();
        let file_hash = make_file_hash(b"shared file content for deletion");
        let chunk = make_indexed_chunk_with_file_hash("Content to be orphaned", file_hash);
        save_chunk(&conn, &chunk).unwrap();

        conn.execute(
            "INSERT INTO indexed_files (context_id, file_path, file_hash, mtime_ns) VALUES (?, ?, ?, ?)",
            rusqlite::params!["context-a", "file.md", file_hash.as_bytes().as_slice(), 1700000000_i64 * 1_000_000_000],
        ).unwrap();
        conn.execute(
            "INSERT INTO indexed_files (context_id, file_path, file_hash, mtime_ns) VALUES (?, ?, ?, ?)",
            rusqlite::params!["context-b", "file.md", file_hash.as_bytes().as_slice(), 1700000000_i64 * 1_000_000_000],
        ).unwrap();

        // When removing all references
        conn.execute("DELETE FROM indexed_files", []).unwrap();

        // And running garbage collection
        let result = delete_orphaned_chunks(&conn);

        // Then it should return 1 (orphan deleted)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // And the chunk should no longer exist
        assert!(!has_chunk(&conn, &chunk.chunk.chunk_hash).unwrap());
    }

    #[test]
    fn delete_orphaned_chunks_returns_correct_count_for_multiple_orphans() {
        // Given a database with multiple orphaned chunks
        let conn = setup_connection();
        let chunk1 = make_indexed_chunk("Orphan 1");
        let chunk2 =
            make_indexed_chunk_with_file_hash("Orphan 2", make_file_hash(b"different file"));
        let chunk3 = make_indexed_chunk_with_file_hash("Orphan 3", make_file_hash(b"another file"));
        save_chunk(&conn, &chunk1).unwrap();
        save_chunk(&conn, &chunk2).unwrap();
        save_chunk(&conn, &chunk3).unwrap();

        // When running garbage collection
        let result = delete_orphaned_chunks(&conn);

        // Then it should return 3 (all orphans deleted)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }
}

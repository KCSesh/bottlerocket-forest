//! SQLite implementation of ChunkRepository
//!
//! Provides persistent storage for indexed chunks using SQLite with vector search capabilities.
//!
//! # Submodules
//!
//! * `serialization`: Converts between domain types and database formats
//! * `queries`: Implements CRUD operations for chunk storage
//! * `search`: Provides semantic search using vector embeddings
//! * `context`: Context repository for multi-context indexing
//! * `files`: Storage queries for indexed files
//! * `chunks`: Content-addressed chunk storage queries

#[cfg(test)]
mod config;
#[cfg(test)]
mod embedding;

pub mod chunks;
mod context;
pub mod files;
mod queries;
mod search;
mod serialization;

pub use context::SqliteContextRepository;

use rusqlite::Connection;
use snafu::ResultExt;
use std::path::Path;

use super::repository::{ChunkRepository, StorageError, storage_error::*};
use super::schema;
use crate::knowledge::domain::{
    ChunkHash, ChunkId, ContextId, EmbeddingModelConfig, FileHash, ForestRelativePath,
    IndexMetadata, IndexedChunk, Timestamp,
};

/// SQLite-backed implementation of chunk repository with vector search
#[derive(Debug)]
pub struct SqliteChunkRepository {
    conn: Connection,
}

impl SqliteChunkRepository {
    /// Opens or creates a database at the specified path
    ///
    /// Initializes the schema and registers the sqlite-vec extension for vector operations.
    pub fn open(
        path: impl AsRef<Path>,
        config: &EmbeddingModelConfig,
    ) -> Result<Self, StorageError> {
        // SAFETY: This call satisfies the safety requirements for sqlite3_auto_extension:
        // 1. We are not calling this from within an auto-extension handler (would cause recursion)
        // 2. We will not close any database connection from within the auto-extension
        // 3. We will not manipulate the auto-extension list from within an auto-extension
        // 4. sqlite3_vec_init is a valid C function pointer with the correct signature
        //    provided by the sqlite-vec crate's extern "C" block
        // 5. The transmute is valid because:
        //    - Both types are function pointers with the same size
        //    - sqlite3_vec_init has the correct signature expected by sqlite3_auto_extension
        //    - The function pointer is statically linked and will remain valid
        #[allow(clippy::missing_transmute_annotations)]
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }

        let conn = Connection::open(path.as_ref()).context(DatabaseSnafu)?;

        conn.create_scalar_function(
            "LN",
            1,
            rusqlite::functions::FunctionFlags::SQLITE_UTF8
                | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
            |ctx| {
                let value = ctx.get::<f64>(0)?;
                Ok(value.ln())
            },
        )
        .context(DatabaseSnafu)?;

        schema::create_tables(&conn, config).map_err(|e| {
            InvalidDataSnafu {
                message: e.to_string(),
            }
            .build()
        })?;

        schema::check_schema_version(&conn).map_err(|e| {
            InvalidDataSnafu {
                message: e.to_string(),
            }
            .build()
        })?;

        Ok(Self { conn })
    }

    /// Opens an existing database and validates model configuration compatibility
    ///
    /// Verifies that the stored model configuration matches the expected configuration.
    pub fn open_with_config(
        path: impl AsRef<Path>,
        expected_config: &EmbeddingModelConfig,
    ) -> Result<Self, StorageError> {
        let repo = Self::open(path, expected_config)?;
        let metadata = repo.get_metadata()?;

        snafu::ensure!(
            metadata.model_config == *expected_config,
            ConfigMismatchSnafu {
                expected: expected_config.clone(),
                actual: metadata.model_config,
            }
        );

        Ok(repo)
    }

    /// Returns a context repository backed by this connection
    pub fn context_repository(&self) -> SqliteContextRepository<'_> {
        SqliteContextRepository::new(&self.conn)
    }
}

impl ChunkRepository for SqliteChunkRepository {
    fn save(&mut self, chunk: &IndexedChunk) -> Result<(), StorageError> {
        queries::save(&mut self.conn, chunk)
    }

    fn save_batch(&mut self, chunks: &[IndexedChunk]) -> Result<(), StorageError> {
        queries::save_batch(&mut self.conn, chunks)
    }

    fn find_by_id(&self, id: &ChunkId) -> Result<Option<IndexedChunk>, StorageError> {
        queries::find_by_id(&self.conn, id)
    }

    fn find_by_file(&self, path: &ForestRelativePath) -> Result<Vec<IndexedChunk>, StorageError> {
        queries::find_by_file(&self.conn, path)
    }

    fn find_all(&self) -> Result<Vec<IndexedChunk>, StorageError> {
        queries::find_all(&self.conn)
    }

    fn get_indexed_files(
        &self,
        context_id: &ContextId,
    ) -> Result<
        std::collections::HashMap<ForestRelativePath, crate::knowledge::domain::Timestamp>,
        StorageError,
    > {
        queries::get_indexed_files(&self.conn, context_id)
    }

    fn clear(&mut self) -> Result<usize, StorageError> {
        queries::clear(&mut self.conn)
    }

    fn get_metadata(&self) -> Result<IndexMetadata, StorageError> {
        queries::get_metadata(&self.conn)
    }

    fn set_metadata(&mut self, metadata: &IndexMetadata) -> Result<(), StorageError> {
        queries::set_metadata(&mut self.conn, metadata)
    }

    fn search_semantic(
        &self,
        query_embedding: &[f32],
        limit: usize,
        context_id: ContextId,
    ) -> Result<Vec<(IndexedChunk, f32)>, StorageError> {
        search::search_semantic(&self.conn, query_embedding, limit, context_id)
    }

    fn has_embedding(&self, chunk_hash: &ChunkHash) -> Result<bool, StorageError> {
        let result = self.has_embedding_batch(&[*chunk_hash])?;
        Ok(result.contains(chunk_hash))
    }

    fn has_embedding_batch(
        &self,
        chunk_hashes: &[ChunkHash],
    ) -> Result<std::collections::HashSet<ChunkHash>, StorageError> {
        use super::repository::storage_error::*;

        if chunk_hashes.is_empty() {
            return Ok(std::collections::HashSet::new());
        }

        let placeholders = chunk_hashes
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT chunk_hash FROM vec_chunks WHERE chunk_hash IN ({})",
            placeholders
        );

        let mut stmt = self.conn.prepare(&query).context(DatabaseSnafu)?;
        let params: Vec<String> = chunk_hashes.iter().map(|h| h.to_string()).collect();
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        let mut rows = stmt.query(params_refs.as_slice()).context(DatabaseSnafu)?;
        let mut existing = std::collections::HashSet::new();

        while let Some(row) = rows.next().context(DatabaseSnafu)? {
            let hash_str: String = row.get(0).context(DatabaseSnafu)?;
            if let Some(hash) = chunk_hashes.iter().find(|h| h.to_string() == hash_str) {
                existing.insert(*hash);
            }
        }

        Ok(existing)
    }

    fn track_indexed_file(
        &mut self,
        file_path: &ForestRelativePath,
        file_hash: &FileHash,
        mtime: Timestamp,
        context_id: &ContextId,
    ) -> Result<(), StorageError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO indexed_files (context_id, file_path, file_hash, mtime_ns) VALUES (?, ?, ?, ?)",
                rusqlite::params![
                    context_id.as_str(),
                    file_path.to_string(),
                    file_hash.as_bytes().as_slice(),
                    mtime.as_secs() * 1_000_000_000,
                ],
            )
            .context(DatabaseSnafu)?;
        Ok(())
    }

    fn delete_orphaned_chunks(&mut self) -> Result<u64, StorageError> {
        chunks::delete_orphaned_chunks(&self.conn).map_err(|e| StorageError::DatabaseError {
            source: match e {
                chunks::ChunkStorageError::Database { source } => source,
                chunks::ChunkStorageError::InvalidData { message: _ } => {
                    rusqlite::Error::InvalidQuery
                }
            },
        })
    }

    fn clear_context_files(&mut self, context_id: &ContextId) -> Result<usize, StorageError> {
        files::delete_indexed_files_for_context(&self.conn, context_id).map_err(|e| match e {
            files::IndexedFileError::Database { source } => StorageError::DatabaseError { source },
            files::IndexedFileError::InvalidData { message } => {
                StorageError::InvalidData { message }
            }
        })
    }

    fn remove_indexed_file_from_context(
        &mut self,
        file_path: &ForestRelativePath,
        context_id: &ContextId,
    ) -> Result<(), StorageError> {
        files::remove_indexed_file_from_context(&self.conn, file_path, context_id)
            .map(|_| ())
            .map_err(|e| match e {
                files::IndexedFileError::Database { source } => {
                    StorageError::DatabaseError { source }
                }
                files::IndexedFileError::InvalidData { message } => {
                    StorageError::InvalidData { message }
                }
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::constants::EMBEDDING_DIM;
    use crate::knowledge::domain::{
        Chunk, ChunkContent, ChunkContext, ChunkHash, ChunkSource, Embedding, FileHash,
        HeadingText, ItemName, MarkdownContext, RepoName, RustDocContext, Signature, TokenCount,
        Visibility,
    };
    use crate::knowledge::domain::{EmbeddingModelConfig, Timestamp};
    use std::time::SystemTime;
    use tempfile::NamedTempFile;
    use test_case::test_case;

    pub(super) fn test_config() -> EmbeddingModelConfig {
        EmbeddingModelConfig::default()
    }

    pub(super) fn create_test_chunk_at(file_path: &str, repo_name: &str) -> IndexedChunk {
        let content = format!("best test content for {}", file_path);
        let chunk_hash = ChunkHash::from_text(&content);
        let file_hash = FileHash::new([1u8; 32]); // Different placeholder file hash

        let chunk = Chunk::builder()
            .id(ChunkId::new(uuid::Uuid::new_v4()))
            .chunk_hash(chunk_hash)
            .file_hash(file_hash)
            .source(
                ChunkSource::builder()
                    .file_path(ForestRelativePath::try_new(file_path).unwrap())
                    .repo_name(RepoName::try_new(repo_name).unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(&content)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(ChunkContext::Markdown(
                MarkdownContext::builder().heading_hierarchy(vec![]).build(),
            ))
            .build();

        IndexedChunk::builder()
            .chunk(chunk)
            .embedding(Embedding::try_new(vec![0.1; EMBEDDING_DIM]).unwrap())
            .indexed_at(Timestamp::now())
            .build()
    }

    fn create_test_chunk() -> IndexedChunk {
        create_test_chunk_at("test.md", "test-repo")
    }

    #[test]
    fn test_save_and_retrieve() {
        // Given A repository and a chunk
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();
        let indexed_chunk = create_test_chunk();

        // When Saving the chunk
        repo.save(&indexed_chunk).unwrap();

        // Then It should be retrievable via find_all
        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].chunk.content.text, indexed_chunk.chunk.content.text);
        assert_eq!(all[0].chunk.chunk_hash, indexed_chunk.chunk.chunk_hash);
    }

    #[test]
    fn test_save_batch() {
        // Given A repository and multiple chunks with different content
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();
        let chunk1 = create_test_chunk_at("file1.md", "test-repo");
        let chunk2 = create_test_chunk_at("file2.md", "test-repo");
        let chunks = vec![chunk1, chunk2];

        // When Saving in batch
        repo.save_batch(&chunks).unwrap();

        // Then All chunks should be retrievable
        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_find_by_file() {
        // Given A repository with chunks from different files
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();
        let chunk1 = create_test_chunk_at("test.md", "test-repo");
        let chunk2 = create_test_chunk_at("other.md", "test-repo");

        repo.save(&chunk1).unwrap();
        repo.save(&chunk2).unwrap();

        // When Finding by file (note: in new schema, file_path is not stored in chunks)
        let results = repo
            .find_by_file(&ForestRelativePath::try_new("test.md").unwrap())
            .unwrap();

        // Then Returns empty because file_path is no longer stored in chunks table
        // Use find_by_file_hash for file-based lookups in the new schema
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_clear() {
        // Given A repository with chunks
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();
        let chunk1 = create_test_chunk_at("file1.md", "test-repo");
        let chunk2 = create_test_chunk_at("file2.md", "test-repo");
        repo.save(&chunk1).unwrap();
        repo.save(&chunk2).unwrap();

        // When Clearing
        let count = repo.clear().unwrap();

        // Then All chunks should be removed
        assert_eq!(count, 2);
        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 0);
    }

    #[test]
    fn test_metadata() {
        // Given A repository
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();

        // When Setting metadata
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(10)
            .file_count(5)
            .model_config(crate::knowledge::domain::EmbeddingModelConfig::default())
            .build();

        repo.set_metadata(&metadata).unwrap();

        // Then It should be retrievable
        let retrieved = repo.get_metadata().unwrap();
        // chunk_count and file_count are derived from chunks table, not stored in metadata
        assert_eq!(retrieved.chunk_count, 0); // No chunks inserted yet
        assert_eq!(retrieved.file_count, 0);
        assert_eq!(retrieved.model_config, metadata.model_config);
    }

    #[test_case(
        ChunkContext::Markdown(MarkdownContext::builder().heading_hierarchy(vec![]).build())
        ; "markdown context with empty hierarchy"
    )]
    #[test_case(
        ChunkContext::Markdown(
            MarkdownContext::builder()
                .heading_hierarchy(vec![
                    HeadingText::try_new("Architecture").unwrap(),
                    HeadingText::try_new("Boot Process").unwrap(),
                ])
                .build()
        )
        ; "markdown context with hierarchy"
    )]
    #[test_case(
        ChunkContext::RustDoc(
            RustDocContext::builder()
                .item_name(ItemName::try_new("build_variant").unwrap())
                .visibility(Visibility::Public)
                .signature(Signature::try_new("pub fn build_variant()").unwrap())
                .item_type(crate::knowledge::indexing::RustItemType::Function)
                .build()
        )
        ; "rustdoc context with signature"
    )]
    #[test_case(
        ChunkContext::RustDoc(
            RustDocContext::builder()
                .item_name(ItemName::try_new("Config").unwrap())
                .visibility(Visibility::Private)
                .item_type(crate::knowledge::indexing::RustItemType::Struct)
                .build()
        )
        ; "rustdoc context without signature"
    )]
    fn test_context_roundtrip(context: ChunkContext) {
        // Given A repository and a chunk with specific context
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();

        let content = "test content for context roundtrip";
        let chunk_hash = ChunkHash::from_text(content);
        let file_hash = FileHash::new([2u8; 32]);

        let chunk = Chunk::builder()
            .id(ChunkId::new(uuid::Uuid::new_v4()))
            .chunk_hash(chunk_hash)
            .file_hash(file_hash)
            .source(
                ChunkSource::builder()
                    .file_path(ForestRelativePath::try_new("test.md").unwrap())
                    .repo_name(RepoName::try_new("test-repo").unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(content)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(context.clone())
            .build();

        let indexed_chunk = IndexedChunk::builder()
            .chunk(chunk)
            .embedding(Embedding::try_new(vec![0.1; EMBEDDING_DIM]).unwrap())
            .indexed_at(Timestamp::now())
            .build();

        // When Saving and retrieving the chunk
        repo.save(&indexed_chunk).unwrap();
        let all = repo.find_all().unwrap();
        let retrieved = &all[0];

        // Then The context should be preserved with correct type
        assert_eq!(retrieved.chunk.context, context);
    }

    #[test]
    fn test_save_with_embedding_stores_in_both_tables() {
        // Given A repository and a chunk with an embedding
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();

        let embedding = Embedding::try_new(
            (1..=EMBEDDING_DIM)
                .map(|i| i as f32 / EMBEDDING_DIM as f32)
                .collect(),
        )
        .unwrap();

        let content = "test content for embedding";
        let chunk_hash = ChunkHash::from_text(content);
        let file_hash = FileHash::new([3u8; 32]);

        let chunk = Chunk::builder()
            .id(ChunkId::new(uuid::Uuid::new_v4()))
            .chunk_hash(chunk_hash)
            .file_hash(file_hash)
            .source(
                ChunkSource::builder()
                    .file_path(ForestRelativePath::try_new("test.md").unwrap())
                    .repo_name(RepoName::try_new("test-repo").unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(content)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(ChunkContext::Markdown(
                MarkdownContext::builder().heading_hierarchy(vec![]).build(),
            ))
            .build();

        let indexed_chunk = IndexedChunk::builder()
            .chunk(chunk)
            .embedding(embedding)
            .indexed_at(Timestamp::now())
            .build();

        // When Saving the chunk
        repo.save(&indexed_chunk).unwrap();

        // Then It should be in both chunks and vec_chunks tables
        let chunk_exists: bool = repo
            .conn
            .query_row(
                "SELECT 1 FROM chunks WHERE chunk_hash = ?1",
                rusqlite::params![indexed_chunk.chunk.chunk_hash.as_bytes().as_slice()],
                |_| Ok(true),
            )
            .unwrap();
        assert!(chunk_exists);

        let vec_chunk_exists: bool = repo
            .conn
            .query_row(
                "SELECT 1 FROM vec_chunks WHERE chunk_hash = ?1",
                rusqlite::params![indexed_chunk.chunk.chunk_hash.to_string()],
                |_| Ok(true),
            )
            .unwrap();
        assert!(vec_chunk_exists);
    }

    #[test]
    fn test_get_indexed_files() {
        // Given A repository with multiple chunks from different files
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();

        let chunk1 = create_test_chunk_at("repo1/file1.md", "repo1");
        let chunk2 = create_test_chunk_at("repo2/file2.md", "repo2");

        repo.save(&chunk1).unwrap();
        repo.save(&chunk2).unwrap();

        // When Getting indexed files for default context
        let default_ctx = ContextId::from_path(".").unwrap();
        let indexed_files = repo.get_indexed_files(&default_ctx).unwrap();

        // Then In the new schema, get_indexed_files returns empty map
        // because file_path is no longer stored in chunks table.
        // File tracking is now done through the indexed_files table
        // which is context-scoped.
        assert_eq!(indexed_files.len(), 0);
    }

    #[test]
    fn has_embedding_returns_true_when_embedding_exists() {
        // Given a repository with a saved chunk (which has an embedding in vec_chunks)
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();
        let chunk = create_test_chunk();
        repo.save(&chunk).unwrap();

        // When checking if the embedding exists
        let result = repo.has_embedding(&chunk.chunk.chunk_hash);

        // Then it should return true
        assert!(result.unwrap());
    }

    #[test]
    fn has_embedding_returns_false_when_embedding_does_not_exist() {
        // Given a repository with no chunks
        let temp_file = NamedTempFile::new().unwrap();
        let repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();
        let nonexistent_hash = ChunkHash::from_text("nonexistent content");

        // When checking if an embedding exists for a hash that was never stored
        let result = repo.has_embedding(&nonexistent_hash);

        // Then it should return false
        assert!(!result.unwrap());
    }

    #[test]
    fn has_embedding_batch_returns_empty_set_when_no_embeddings_exist() {
        // Given a repository with no chunks
        let temp_file = NamedTempFile::new().unwrap();
        let repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();
        let hashes = vec![
            ChunkHash::from_text("content1"),
            ChunkHash::from_text("content2"),
            ChunkHash::from_text("content3"),
        ];

        // When checking which hashes have embeddings
        let result = repo.has_embedding_batch(&hashes);

        // Then it should return an empty set
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn has_embedding_batch_returns_subset_of_hashes_that_have_embeddings() {
        // Given a repository with some chunks saved
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();

        let chunk1 = create_test_chunk_at("file1.md", "repo");
        let chunk2 = create_test_chunk_at("file2.md", "repo");
        repo.save(&chunk1).unwrap();
        repo.save(&chunk2).unwrap();

        // When checking a mix of existing and non-existing hashes
        let existing_hash1 = chunk1.chunk.chunk_hash;
        let existing_hash2 = chunk2.chunk.chunk_hash;
        let nonexistent_hash = ChunkHash::from_text("nonexistent content");
        let hashes = vec![existing_hash1, nonexistent_hash, existing_hash2];

        let result = repo.has_embedding_batch(&hashes).unwrap();

        // Then it should return only the hashes that exist
        assert_eq!(result.len(), 2);
        assert!(result.contains(&existing_hash1));
        assert!(result.contains(&existing_hash2));
        assert!(!result.contains(&nonexistent_hash));
    }

    #[test]
    fn has_embedding_batch_returns_all_hashes_when_all_have_embeddings() {
        // Given a repository with chunks saved
        let temp_file = NamedTempFile::new().unwrap();
        let mut repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();

        let chunk1 = create_test_chunk_at("file1.md", "repo");
        let chunk2 = create_test_chunk_at("file2.md", "repo");
        let chunk3 = create_test_chunk_at("file3.md", "repo");
        repo.save(&chunk1).unwrap();
        repo.save(&chunk2).unwrap();
        repo.save(&chunk3).unwrap();

        // When checking hashes that all exist
        let hashes = vec![
            chunk1.chunk.chunk_hash,
            chunk2.chunk.chunk_hash,
            chunk3.chunk.chunk_hash,
        ];

        let result = repo.has_embedding_batch(&hashes).unwrap();

        // Then it should return all hashes
        assert_eq!(result.len(), 3);
        for hash in &hashes {
            assert!(result.contains(hash));
        }
    }

    #[test]
    fn has_embedding_batch_handles_empty_input() {
        // Given a repository
        let temp_file = NamedTempFile::new().unwrap();
        let repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();

        // When checking an empty list of hashes
        let result = repo.has_embedding_batch(&[]);

        // Then it should return an empty set
        assert!(result.unwrap().is_empty());
    }
}

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
    ChunkHash, ChunkId, ContextId, EmbeddingModelConfig, FileHash, IndexMetadata,
    IndexRelativePath, IndexedChunk, Timestamp,
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

    fn find_by_file(&self, path: &IndexRelativePath) -> Result<Vec<IndexedChunk>, StorageError> {
        queries::find_by_file(&self.conn, path)
    }

    fn find_all(&self) -> Result<Vec<IndexedChunk>, StorageError> {
        queries::find_all(&self.conn)
    }

    fn get_indexed_files(
        &self,
        context_id: &ContextId,
    ) -> Result<
        std::collections::HashMap<IndexRelativePath, crate::knowledge::domain::Timestamp>,
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
        file_path: &IndexRelativePath,
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
        chunks::delete_orphaned_chunks(&self.conn)
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
        file_path: &IndexRelativePath,
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

    fn setup_repo() -> (NamedTempFile, SqliteChunkRepository) {
        let temp_file = NamedTempFile::new().unwrap();
        let repo = SqliteChunkRepository::open(temp_file.path(), &test_config()).unwrap();
        (temp_file, repo)
    }

    fn build_chunk(
        content: &str,
        file_path: &str,
        context: ChunkContext,
        embedding: Embedding,
    ) -> IndexedChunk {
        let chunk = Chunk::builder()
            .id(ChunkId::new(uuid::Uuid::new_v4()))
            .chunk_hash(ChunkHash::from_text(content))
            .file_hash(FileHash::new([1u8; 32]))
            .source(
                ChunkSource::builder()
                    .file_path(IndexRelativePath::try_new(file_path).unwrap())
                    .repo_name(RepoName::try_new("test-repo").unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(content)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(context)
            .build();
        IndexedChunk::builder()
            .chunk(chunk)
            .embedding(embedding)
            .indexed_at(Timestamp::now())
            .build()
    }

    fn default_context() -> ChunkContext {
        ChunkContext::Markdown(MarkdownContext::builder().heading_hierarchy(vec![]).build())
    }

    fn default_embedding() -> Embedding {
        Embedding::try_new(vec![0.1; EMBEDDING_DIM]).unwrap()
    }

    pub(super) fn create_test_chunk_at(file_path: &str, repo_name: &str) -> IndexedChunk {
        let content = format!("best test content for {}", file_path);
        let chunk = Chunk::builder()
            .id(ChunkId::new(uuid::Uuid::new_v4()))
            .chunk_hash(ChunkHash::from_text(&content))
            .file_hash(FileHash::new([1u8; 32]))
            .source(
                ChunkSource::builder()
                    .file_path(IndexRelativePath::try_new(file_path).unwrap())
                    .repo_name(RepoName::try_new(repo_name).unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(&content)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(default_context())
            .build();
        IndexedChunk::builder()
            .chunk(chunk)
            .embedding(default_embedding())
            .indexed_at(Timestamp::now())
            .build()
    }

    fn create_test_chunk() -> IndexedChunk {
        create_test_chunk_at("test.md", "test-repo")
    }

    fn save_chunks(repo: &mut SqliteChunkRepository, paths: &[&str]) -> Vec<IndexedChunk> {
        let chunks: Vec<_> = paths
            .iter()
            .map(|p| create_test_chunk_at(p, "test-repo"))
            .collect();
        for chunk in &chunks {
            repo.save(chunk).unwrap();
        }
        chunks
    }

    #[test]
    fn test_save_and_retrieve() {
        // Given A repository and a chunk
        let (_temp, mut repo) = setup_repo();
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
        let (_temp, mut repo) = setup_repo();
        let chunks = vec![
            create_test_chunk_at("file1.md", "test-repo"),
            create_test_chunk_at("file2.md", "test-repo"),
        ];

        // When Saving in batch
        repo.save_batch(&chunks).unwrap();

        // Then All chunks should be retrievable
        assert_eq!(repo.find_all().unwrap().len(), 2);
    }

    #[test]
    fn test_find_by_file() {
        // Given A repository with chunks from different files
        let (_temp, mut repo) = setup_repo();
        save_chunks(&mut repo, &["test.md", "other.md"]);

        // When Finding by file (note: in new schema, file_path is not stored in chunks)
        let results = repo
            .find_by_file(&IndexRelativePath::try_new("test.md").unwrap())
            .unwrap();

        // Then Returns empty because file_path is no longer stored in chunks table
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_clear() {
        // Given A repository with chunks
        let (_temp, mut repo) = setup_repo();
        save_chunks(&mut repo, &["file1.md", "file2.md"]);

        // When Clearing
        let count = repo.clear().unwrap();

        // Then All chunks should be removed
        assert_eq!(count, 2);
        assert_eq!(repo.find_all().unwrap().len(), 0);
    }

    #[test]
    fn test_metadata() {
        // Given A repository
        let (_temp, mut repo) = setup_repo();

        // When Setting metadata
        let metadata = IndexMetadata::builder()
            .last_build(SystemTime::now())
            .chunk_count(10)
            .file_count(5)
            .model_config(EmbeddingModelConfig::default())
            .build();
        repo.set_metadata(&metadata).unwrap();

        // Then It should be retrievable
        let retrieved = repo.get_metadata().unwrap();
        assert_eq!(retrieved.chunk_count, 0);
        assert_eq!(retrieved.file_count, 0);
        assert_eq!(retrieved.model_config, metadata.model_config);
    }

    #[test_case(
        ChunkContext::Markdown(MarkdownContext::builder().heading_hierarchy(vec![]).build())
        ; "markdown context with empty hierarchy"
    )]
    #[test_case(
        ChunkContext::Markdown(MarkdownContext::builder()
            .heading_hierarchy(vec![
                HeadingText::try_new("Architecture").unwrap(),
                HeadingText::try_new("Boot Process").unwrap(),
            ]).build())
        ; "markdown context with hierarchy"
    )]
    #[test_case(
        ChunkContext::RustDoc(RustDocContext::builder()
            .item_name(ItemName::try_new("build_variant").unwrap())
            .visibility(Visibility::Public)
            .signature(Signature::try_new("pub fn build_variant()").unwrap())
            .item_type(crate::knowledge::indexing::RustItemType::Function)
            .build())
        ; "rustdoc context with signature"
    )]
    #[test_case(
        ChunkContext::RustDoc(RustDocContext::builder()
            .item_name(ItemName::try_new("Config").unwrap())
            .visibility(Visibility::Private)
            .item_type(crate::knowledge::indexing::RustItemType::Struct)
            .build())
        ; "rustdoc context without signature"
    )]
    fn test_context_roundtrip(context: ChunkContext) {
        // Given A repository and a chunk with specific context
        let (_temp, mut repo) = setup_repo();
        let indexed_chunk = build_chunk(
            "test content for context roundtrip",
            "test.md",
            context.clone(),
            default_embedding(),
        );

        // When Saving and retrieving the chunk
        repo.save(&indexed_chunk).unwrap();

        // Then The context should be preserved with correct type
        assert_eq!(repo.find_all().unwrap()[0].chunk.context, context);
    }

    #[test]
    fn test_save_with_embedding_stores_in_both_tables() {
        // Given A repository and a chunk with an embedding
        let (_temp, mut repo) = setup_repo();
        let embedding = Embedding::try_new(
            (1..=EMBEDDING_DIM)
                .map(|i| i as f32 / EMBEDDING_DIM as f32)
                .collect(),
        )
        .unwrap();
        let indexed_chunk = build_chunk(
            "test content for embedding",
            "test.md",
            default_context(),
            embedding,
        );

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
        let (_temp, mut repo) = setup_repo();
        save_chunks(&mut repo, &["repo1/file1.md", "repo2/file2.md"]);

        // When Getting indexed files for default context
        let default_ctx = ContextId::from_path(".").unwrap();
        let indexed_files = repo.get_indexed_files(&default_ctx).unwrap();

        // Then In the new schema, get_indexed_files returns empty map
        assert_eq!(indexed_files.len(), 0);
    }

    #[test_case(&[], &[] ; "no embeddings exist")]
    #[test_case(&["file1.md", "file2.md"], &[0, 1] ; "all embeddings exist")]
    #[test_case(&["file1.md", "file2.md"], &[0] ; "subset of embeddings exist")]
    fn test_has_embedding_batch(saved_files: &[&str], existing_indices: &[usize]) {
        // Given a repository with some chunks saved
        let (_temp, mut repo) = setup_repo();
        let saved = save_chunks(&mut repo, saved_files);

        // When checking which hashes have embeddings
        let mut hashes = vec![];
        for idx in existing_indices {
            hashes.push(saved[*idx].chunk.chunk_hash);
        }
        if !saved.is_empty() && existing_indices.len() < saved.len() {
            hashes.push(ChunkHash::from_text("nonexistent"));
        }

        let result = repo.has_embedding_batch(&hashes).unwrap();

        // Then it should return only the hashes that exist
        assert_eq!(result.len(), existing_indices.len());
        for idx in existing_indices {
            assert!(result.contains(&saved[*idx].chunk.chunk_hash));
        }
    }

    #[test]
    fn has_embedding_batch_handles_empty_input() {
        // Given A repository
        let (_temp, repo) = setup_repo();

        // When Checking an empty list of hashes
        let result = repo.has_embedding_batch(&[]);

        // Then It should return an empty set
        assert!(result.unwrap().is_empty());
    }

    #[test_case(true ; "embedding exists")]
    #[test_case(false ; "embedding does not exist")]
    fn test_has_embedding(exists: bool) {
        // Given a repository with or without a saved chunk
        let (_temp, mut repo) = setup_repo();
        let chunk = create_test_chunk();
        let hash = if exists {
            repo.save(&chunk).unwrap();
            chunk.chunk.chunk_hash
        } else {
            ChunkHash::from_text("nonexistent content")
        };

        // When checking if the embedding exists
        let result = repo.has_embedding(&hash);

        // Then it should return the expected result
        assert_eq!(result.unwrap(), exists);
    }
}

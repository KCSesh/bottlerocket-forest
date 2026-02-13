//! Abstract repository interface for chunk persistence
//!
//! Defines the contract for storing, retrieving, and searching indexed documentation chunks.

use snafu::Snafu;
use std::collections::HashSet;

use crate::knowledge::domain::{
    ChunkHash, ChunkId, Context, ContextId, EmbeddingModelConfig, FileHash, FileSearchResult,
    IndexMetadata, IndexRelativePath, IndexedChunk, RelevanceScore, ResultLimit, Timestamp,
};

/// Abstract interface for chunk storage operations
#[cfg_attr(test, mockall::automock)]
pub trait ChunkRepository {
    /// Persists a single indexed chunk to storage
    fn save(&mut self, chunk: &IndexedChunk) -> Result<(), StorageError>;

    /// Persists multiple indexed chunks in a single transaction
    fn save_batch(&mut self, chunks: &[IndexedChunk]) -> Result<(), StorageError>;

    /// Retrieves an indexed chunk by its unique identifier
    fn find_by_id(&self, id: &ChunkId) -> Result<Option<IndexedChunk>, StorageError>;

    /// Retrieves all indexed chunks from a specific file
    fn find_by_file(&self, path: &IndexRelativePath) -> Result<Vec<IndexedChunk>, StorageError>;

    /// Retrieves all indexed chunks from storage
    fn find_all(&self) -> Result<Vec<IndexedChunk>, StorageError>;

    /// Retrieves file paths and their most recent indexing timestamps
    ///
    /// Avoids loading chunk content, returning only paths and timestamps for incremental update comparisons.
    fn get_indexed_files(
        &self,
        context_id: &ContextId,
    ) -> Result<
        std::collections::HashMap<IndexRelativePath, crate::knowledge::domain::Timestamp>,
        StorageError,
    >;

    /// Removes all chunks from storage
    fn clear(&mut self) -> Result<usize, StorageError>;

    /// Retrieves index metadata including build time and model configuration
    fn get_metadata(&self) -> Result<IndexMetadata, StorageError>;

    /// Updates index metadata
    fn set_metadata(&mut self, metadata: &IndexMetadata) -> Result<(), StorageError>;

    /// Searches for chunks semantically similar to the query embedding
    ///
    /// Returns chunks ranked by similarity score in descending order.
    fn search_semantic(
        &self,
        query_embedding: &[f32],
        limit: ResultLimit,
        context_id: ContextId,
    ) -> Result<Vec<(IndexedChunk, RelevanceScore)>, StorageError>;

    /// Searches for files containing semantically similar chunks.
    ///
    /// Over-fetches chunks using `k = file_limit * chunk_multiplier`, then groups
    /// results by file path. Returns files ranked by their best chunk match.
    fn search_files(
        &self,
        query_embedding: &[f32],
        file_limit: ResultLimit,
        chunk_multiplier: usize,
        context_id: ContextId,
    ) -> Result<Vec<FileSearchResult>, StorageError>;

    /// Checks if an embedding exists for the given chunk hash
    fn has_embedding(&self, chunk_hash: &ChunkHash) -> Result<bool, StorageError>;

    /// Checks which chunk hashes already have embeddings
    ///
    /// Returns the subset of input hashes that have existing embeddings.
    fn has_embedding_batch(
        &self,
        chunk_hashes: &[ChunkHash],
    ) -> Result<HashSet<ChunkHash>, StorageError>;

    /// Records a file as indexed in the specified context
    fn track_indexed_file(
        &mut self,
        file_path: &IndexRelativePath,
        file_hash: &FileHash,
        mtime: Timestamp,
        context_id: &ContextId,
    ) -> Result<(), StorageError>;

    /// Removes a file from the specified context
    fn remove_indexed_file_from_context(
        &mut self,
        file_path: &IndexRelativePath,
        context_id: &ContextId,
    ) -> Result<(), StorageError>;

    /// Deletes chunks not referenced by any context's indexed files
    ///
    /// Removes orphaned chunks whose file_hash is not present in the indexed_files table.
    /// Also removes associated embeddings. Returns the count of deleted chunks.
    fn delete_orphaned_chunks(&mut self) -> Result<u64, StorageError>;

    /// Removes all file mappings for a context
    ///
    /// Deletes indexed_files records for the given context, leaving the context registered.
    /// Returns the count of deleted file mappings.
    fn clear_context_files(&mut self, context_id: &ContextId) -> Result<usize, StorageError>;
}

/// Abstract interface for context storage operations
///
/// Stores and retrieves contexts and their file mappings for multi-context indexing.
/// Contexts represent registered working directories that share a common embedding database.
#[cfg_attr(test, mockall::automock)]
pub trait ContextRepository {
    /// Retrieves all registered contexts
    fn list_contexts(&self) -> Result<Vec<Context>, ContextRepositoryError>;

    /// Retrieves a specific context by its identifier
    fn get_context(
        &self,
        context_id: &ContextId,
    ) -> Result<Option<Context>, ContextRepositoryError>;

    /// Registers a new context in the workspace
    fn insert_context(&self, context: &Context) -> Result<(), ContextRepositoryError>;

    /// Removes a context and its file mappings
    fn remove_context(&self, context_id: &ContextId) -> Result<(), ContextRepositoryError>;
}

/// Errors that can occur during context storage operations.
#[derive(Debug, Snafu, miette::Diagnostic)]
#[doc(hidden)]
#[snafu(module(context_repository_error), visibility(pub(crate)))]
#[non_exhaustive]
pub enum ContextRepositoryError {
    /// Database query or transaction failed.
    #[snafu(display("Database operation failed"))]
    #[diagnostic(
        code(crumbly::context::database_error),
        help("The database may be locked, corrupted, or out of disk space")
    )]
    DatabaseError {
        /// Underlying database error.
        source: rusqlite::Error,
    },

    /// Requested context does not exist.
    #[snafu(display("Context not found: {context_id}"))]
    #[diagnostic(
        code(crumbly::context::not_found),
        help("Use `crumbly context list` to see available contexts")
    )]
    NotFound {
        /// ID of the missing context.
        context_id: String,
    },

    /// Context with this ID already registered.
    #[snafu(display("Context already exists: {context_id}"))]
    #[diagnostic(
        code(crumbly::context::already_exists),
        help("Use a different path or remove the existing context first")
    )]
    AlreadyExists {
        /// ID of the existing context.
        context_id: String,
    },
}

/// Errors that can occur during chunk storage operations.
#[derive(Debug, Snafu, miette::Diagnostic)]
#[doc(hidden)]
#[snafu(module(storage_error), visibility(pub(crate)))]
#[non_exhaustive]
pub enum StorageError {
    /// Database query or transaction failed.
    #[snafu(display("Database operation failed"))]
    #[diagnostic(
        code(crumbly::storage::database_error),
        help("The database may be locked, corrupted, or out of disk space")
    )]
    DatabaseError {
        /// Underlying database error.
        source: rusqlite::Error,
    },

    /// JSON serialization of chunk data failed.
    #[snafu(display("Failed to serialize chunk data to JSON"))]
    #[diagnostic(
        code(crumbly::storage::serialization_error),
        help("The chunk may contain invalid UTF-8 or unsupported characters")
    )]
    SerializationError {
        /// Underlying serialization error.
        source: serde_json::Error,
    },

    /// Database contains malformed or unexpected data.
    #[snafu(display("Invalid data in database: {message}"))]
    #[diagnostic(
        code(crumbly::storage::invalid_data),
        help("The database may be corrupted. Try running `crumbly rebuild`")
    )]
    InvalidData {
        /// Description of the invalid data.
        message: String,
    },

    /// Database field contains an invalid value.
    #[snafu(display("Invalid value in database field '{field}'"))]
    #[diagnostic(
        code(crumbly::storage::invalid_field),
        help("The database schema may be incompatible with this version")
    )]
    InvalidField {
        /// Name of the invalid field.
        field: String,
        /// Underlying parse or conversion error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Requested chunk does not exist in the index.
    #[snafu(display("Chunk not found in index: {id:?}"))]
    #[diagnostic(
        code(crumbly::storage::not_found),
        help("The chunk may have been deleted or the index may be out of sync")
    )]
    NotFound {
        /// ID of the missing chunk.
        id: ChunkId,
    },

    /// Operation requires a different index mode.
    #[snafu(display("Operation not supported in current index mode: {operation}"))]
    #[diagnostic(
        code(crumbly::storage::unsupported_operation),
        help("This operation requires a different index mode")
    )]
    UnsupportedOperation {
        /// Name of the unsupported operation.
        operation: String,
    },

    /// Index was built with different embedding configuration.
    #[snafu(display("Index configuration mismatch\nExpected: {expected:?}\nFound: {actual:?}"))]
    #[diagnostic(
        code(crumbly::storage::config_mismatch),
        help("Run `crumbly rebuild` to recreate the index with the current configuration")
    )]
    ConfigMismatch {
        /// Configuration expected by the application.
        expected: EmbeddingModelConfig,
        /// Configuration found in the index.
        actual: EmbeddingModelConfig,
    },
}

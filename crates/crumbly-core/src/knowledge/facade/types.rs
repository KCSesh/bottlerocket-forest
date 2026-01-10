//! Types for the knowledge index facade
//!
//! This module defines the public API types for interacting with the knowledge index:
//! * [`IndexStatus`] provides metadata about the current state of the index
//! * [`GcStats`] provides statistics from garbage collection operations
//! * [`IndexError`] represents all errors that can occur during facade operations
//!
//! These types form the boundary between the high-level facade API and the underlying
//! domain, storage, and search implementations.

use bon::Builder;
use snafu::Snafu;
use std::time::SystemTime;

use crate::knowledge::domain::{ContextId, EmbeddingModelConfig, QueryTextError, ResultLimitError};

/// Statistics from a garbage collection operation
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[non_exhaustive]
pub struct GcStats {
    /// Number of orphaned chunks deleted
    pub chunks_deleted: usize,

    /// Number of orphaned embeddings deleted
    pub embeddings_deleted: usize,
}

/// Status information about the knowledge index
#[derive(Debug, Clone, PartialEq, Builder)]
#[non_exhaustive]
pub struct IndexStatus {
    /// Whether the index exists and is accessible
    pub exists: bool,

    /// Total number of chunks in the index
    pub chunk_count: usize,

    /// Number of unique files indexed
    pub file_count: usize,

    /// Last time the index was built or updated
    pub last_build: Option<SystemTime>,

    /// Embedding model configuration
    pub model_config: EmbeddingModelConfig,

    /// Approximate size of the index on disk in bytes
    pub size_bytes: Option<u64>,
}

/// Errors that can occur in facade operations
#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub(crate)))]
#[non_exhaustive]
pub enum IndexError {
    #[snafu(display("Forest root directory not found: {path}"))]
    #[diagnostic(
        code(crumbly::index::index_root_not_found),
        help("Ensure you're running the command from within a Bottlerocket forest directory")
    )]
    IndexRootNotFound { path: String },

    #[snafu(display("Failed to create .crumbly directory for index storage"))]
    #[diagnostic(
        code(crumbly::index::crumbly_dir_creation_failed),
        help("Check directory permissions and available disk space")
    )]
    CrumblyDirCreationFailed { source: std::io::Error },

    #[snafu(display("Failed to access knowledge index database"))]
    #[diagnostic(
        code(crumbly::index::database_access_failed),
        help("The database may be corrupted. Try running `crumbly rebuild` to recreate it")
    )]
    DatabaseAccessFailed {
        source: crate::knowledge::storage::StorageError,
    },

    #[snafu(display("Failed to index files"))]
    #[diagnostic(
        code(crumbly::index::indexing_failed),
        help("Check that the files are readable and contain valid content")
    )]
    IndexingFailed {
        source: crate::knowledge::indexing::IndexingError,
    },

    #[snafu(display("Search operation failed"))]
    #[diagnostic(
        code(crumbly::index::search_failed),
        help("The index may be corrupted or incompatible with the current version")
    )]
    SearchFailed {
        source: crate::knowledge::search::SearchError,
    },

    #[snafu(display("Invalid search query"))]
    #[diagnostic(
        code(crumbly::index::invalid_query),
        help("Provide a non-empty query string with valid characters")
    )]
    InvalidQuery { source: QueryTextError },

    #[snafu(display("Invalid result limit"))]
    #[diagnostic(
        code(crumbly::index::invalid_result_limit),
        help("Adjust the --limit parameter to be within the valid range")
    )]
    InvalidResultLimit { source: ResultLimitError },

    #[snafu(display("Failed to read database file metadata"))]
    #[diagnostic(
        code(crumbly::index::file_metadata_failed),
        help("Check that the database file exists and is accessible")
    )]
    FileMetadataFailed { source: std::io::Error },

    #[snafu(display("Failed to initialize embedding model for semantic search"))]
    #[diagnostic(
        code(crumbly::index::embedding_provider_creation_failed),
        help("Ensure the model files can be downloaded and cached in .crumbly/cache/model/")
    )]
    EmbeddingProviderCreationFailed {
        source: crate::knowledge::search::embeddings::EmbeddingError,
    },

    #[snafu(display("Failed to initialize index data provider"))]
    #[diagnostic(
        code(crumbly::index::index_data_provider_creation_failed),
        help("This is an internal error. Please report this issue")
    )]
    IndexDataProviderCreationFailed {
        source: crate::knowledge::indexing::IndexDataError,
    },

    #[snafu(display("Failed to load crumbly configuration"))]
    #[diagnostic(
        code(crumbly::index::config_load_failed),
        help("Check that .crumbly.toml is valid TOML and contains valid target paths")
    )]
    ConfigLoadFailed {
        source: crate::knowledge::indexing::CrumblyConfigError,
    },

    #[snafu(display("Index already exists at {path}"))]
    #[diagnostic(
        code(crumbly::index::already_exists),
        help("Use 'crumbly rebuild' to recreate the index or 'crumbly update' to refresh it")
    )]
    IndexAlreadyExists { path: String },

    #[snafu(display("Index does not exist at {path}"))]
    #[diagnostic(
        code(crumbly::index::not_found),
        help("Use 'crumbly build' to create the index")
    )]
    IndexNotFound { path: String },

    #[snafu(display("Failed to delete index database"))]
    #[diagnostic(
        code(crumbly::index::deletion_failed),
        help("Check file permissions and ensure the database is not in use")
    )]
    IndexDeletionFailed { source: std::io::Error },

    #[snafu(transparent)]
    WorkspaceNotFound {
        source: crate::knowledge::context::DiscoveryError,
    },

    #[snafu(display("No matching context for current directory"))]
    #[diagnostic(
        code(crumbly::index::context_not_found),
        help("Available contexts: {}", available_contexts.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "))
    )]
    ContextNotFound { available_contexts: Vec<ContextId> },

    #[snafu(display("Failed to resolve context"))]
    #[diagnostic(
        code(crumbly::index::context_resolution_failed),
        help("Ensure you are within a registered context directory")
    )]
    ContextResolutionFailed {
        source: crate::knowledge::context::ResolutionError,
    },

    #[snafu(display("Failed to register context"))]
    #[diagnostic(
        code(crumbly::index::context_registration_failed),
        help("The context may already exist or the database may be inaccessible")
    )]
    ContextRegistrationFailed {
        source: crate::knowledge::storage::ContextRepositoryError,
    },

    #[snafu(display("Cannot remove the default context"))]
    #[diagnostic(
        code(crumbly::index::cannot_remove_default_context),
        help(
            "The default context '.' cannot be removed as it is required for workspace operation"
        )
    )]
    CannotRemoveDefaultContext,

    #[snafu(display("Context does not exist: {context_id}"))]
    #[diagnostic(
        code(crumbly::index::context_does_not_exist),
        help("Use 'crumbly context list' to see available contexts")
    )]
    ContextDoesNotExist { context_id: String },

    #[snafu(display(
        "Schema version mismatch: stored version {stored}, expected version {expected}"
    ))]
    #[diagnostic(
        code(crumbly::index::schema_mismatch),
        help("Run `crumbly rebuild` to recreate the index with the current schema")
    )]
    SchemaMismatch { stored: u32, expected: u32 },
}

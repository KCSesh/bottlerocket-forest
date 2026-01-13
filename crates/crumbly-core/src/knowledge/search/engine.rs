//! Search engine abstraction for knowledge index queries
//!
//! Defines the core search interface and error types used by all search implementations.

use snafu::Snafu;

use crate::knowledge::domain::{SearchQuery, SearchResults};

/// Executes search queries against the knowledge index
#[cfg_attr(test, mockall::automock)]
pub trait SearchEngine {
    /// Execute a search query against the knowledge index
    fn search(&self, query: &SearchQuery) -> Result<SearchResults, SearchError>;
}

/// Errors that occur during search operations.
#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub(crate)))]
#[non_exhaustive]
pub enum SearchError {
    /// Database query failed during search.
    #[snafu(display("Failed to query database during search"))]
    #[diagnostic(
        code(crumbly::search::storage_error),
        help("The database may be locked or corrupted")
    )]
    Storage {
        /// Underlying storage error.
        source: crate::knowledge::storage::StorageError,
    },

    /// Failed to generate embedding for the search query text.
    #[snafu(display("Failed to generate embedding vector for search query"))]
    #[diagnostic(
        code(crumbly::search::embedding_failed),
        help("The embedding model may not be loaded or the query text may be invalid")
    )]
    EmbeddingFailed {
        /// Underlying embedding error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Database returned a relevance score outside valid range.
    #[snafu(display("Database returned invalid relevance score: {score}"))]
    #[diagnostic(
        code(crumbly::search::invalid_score),
        help("The index may be corrupted. Try running `crumbly rebuild`")
    )]
    InvalidScore {
        /// The invalid score value.
        score: f32,
        /// Underlying validation error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
}

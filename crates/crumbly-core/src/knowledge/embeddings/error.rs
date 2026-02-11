//! Error types for embedding operations
//!
//! Defines errors that can occur during embedding model loading and
//! embedding generation for semantic search.

use snafu::Snafu;

/// Errors that can occur during embedding operations.
#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub(crate)))]
#[non_exhaustive]
pub enum EmbeddingError {
    /// Failed to load the embedding model from disk or memory.
    #[snafu(display("Failed to load embedding model: {model_name}"))]
    #[diagnostic(
        code(crumbly::embeddings::model_load_failed),
        help("Check that the model name is correct and the model files are accessible")
    )]
    ModelLoadFailed {
        /// Name of the model that failed to load.
        model_name: String,
        /// Underlying error from the model loading operation.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Failed to download the model from a remote URL.
    #[snafu(display("Failed to download model from: {url}"))]
    #[diagnostic(
        code(crumbly::embeddings::model_download_failed),
        help("Check your internet connection and that the URL is accessible")
    )]
    ModelDownloadFailed {
        /// URL that failed to download.
        url: String,
        /// Underlying download error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Failed to generate an embedding vector from input text.
    #[snafu(display("Failed to generate embedding for text"))]
    #[diagnostic(
        code(crumbly::embeddings::embedding_generation_failed),
        help("The text may be too long or contain unsupported characters")
    )]
    EmbeddingGenerationFailed {
        /// Underlying embedding generation error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Embedding vector dimension does not match expected size.
    #[snafu(display("Invalid embedding dimension: expected {expected}, got {actual}"))]
    #[diagnostic(
        code(crumbly::embeddings::dimension_mismatch),
        help("The model configuration may be incorrect or the model may have changed")
    )]
    DimensionMismatch {
        /// Expected embedding dimension.
        expected: usize,
        /// Actual embedding dimension received.
        actual: usize,
    },

    /// Cannot access the model cache directory.
    #[snafu(display("Model cache directory not accessible: {path}"))]
    #[diagnostic(
        code(crumbly::embeddings::cache_access_failed),
        help("Check directory permissions and available disk space")
    )]
    CacheAccessFailed {
        /// Path to the inaccessible cache directory.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

//! Defines the chunking strategy trait and common types for file content processing.

use snafu::Snafu;

use crate::knowledge::domain::{Chunk, ChunkSource, ChunkableContent, FileHash, FilePeek};

/// Strategy for chunking file content into searchable units.
#[cfg_attr(test, mockall::automock)]
pub trait ChunkingStrategy: Send + Sync {
    /// Returns true if this strategy can process the given file.
    fn supports(&self, peek: &FilePeek) -> bool;

    /// Splits the file content into searchable chunks.
    fn chunk(&self, input: &ChunkingInput) -> Result<Vec<Chunk>, ChunkingError>;
}

/// Input for chunking operations.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkingInput {
    /// Raw content to be chunked.
    pub content: ChunkableContent,
    /// Source location metadata.
    pub source: ChunkSource,
    /// Hash of the source file for content-addressed storage.
    pub file_hash: FileHash,
}

/// Errors that can occur during chunking.
#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub(crate)))]
#[non_exhaustive]
pub enum ChunkingError {
    /// Tokenizer failed to initialize.
    #[snafu(display("Failed to initialize tokenizer"))]
    #[diagnostic(
        code(crumbly::chunking::tokenizer_init_error),
        help("Check that the embedding model configuration is valid")
    )]
    TokenizerInitError {
        /// Underlying initialization error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Content parsing failed.
    #[snafu(display("Failed to parse content from {file_path}"))]
    #[diagnostic(
        code(crumbly::chunking::parse_error),
        help("The file may contain unsupported syntax (e.g., negative trait impls)")
    )]
    ParseError {
        /// Path to the file that failed to parse.
        file_path: String,
        /// Underlying parse error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Content exceeds maximum token limit.
    #[snafu(display("Token limit exceeded: {actual} > {max}"))]
    #[diagnostic(
        code(crumbly::chunking::token_limit_exceeded),
        help("The content section is too large and cannot be chunked further")
    )]
    TokenLimitExceeded {
        /// Actual token count.
        actual: usize,
        /// Maximum allowed tokens.
        max: usize,
    },

    /// File contains invalid UTF-8 encoding.
    #[snafu(display("Invalid UTF-8 in content"))]
    #[diagnostic(
        code(crumbly::chunking::invalid_utf8),
        help("The file contains invalid UTF-8 encoding")
    )]
    InvalidUtf8 {
        /// Underlying UTF-8 error.
        source: std::str::Utf8Error,
    },

    /// Language configuration is invalid.
    #[snafu(display("Invalid language configuration: {message}"))]
    #[diagnostic(
        code(crumbly::chunking::config_error),
        help("Check the language configuration in crumbly.toml")
    )]
    ConfigError {
        /// Description of the configuration error.
        message: String,
    },
}

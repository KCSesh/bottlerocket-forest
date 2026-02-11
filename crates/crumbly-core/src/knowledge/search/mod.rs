//! Search implementations for the knowledge index
//!
//! This module provides semantic search capabilities using embedding-based similarity.
//!
//! ## Component Overview
//!
//! * [`engine`]: Core search engine trait and error types
//! * [`semantic`]: Semantic search implementation using embeddings
//! * [`embeddings`]: Re-exported from `crate::knowledge::embeddings`

pub mod engine;
pub mod semantic;

// Re-export embeddings from top-level for backward compatibility
pub use crate::knowledge::embeddings;
pub use crate::knowledge::embeddings::{
    EmbeddingError, EmbeddingModel, EmbeddingProvider, LoadedEmbeddingModel, VectorOps,
};
pub use engine::{SearchEngine, SearchError};
pub use semantic::SemanticSearchEngine;

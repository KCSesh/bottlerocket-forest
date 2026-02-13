//! Chunk domain types
//!
//! Core abstractions:
//! * [`Chunk`] combines source, content, and context
//! * [`ChunkSource`] identifies origin file and repository
//! * [`ChunkContent`] contains text and token count
//! * [`ChunkContext`] provides type-specific metadata

use bon::Builder;
use serde::{Deserialize, Serialize};

use super::{ChunkHash, ChunkId, FileHash, IndexRelativePath, RepoName, TokenCount};

/// Searchable documentation unit with source and context metadata.
#[derive(Debug, Clone, PartialEq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct Chunk {
    /// Unique identifier for this chunk.
    pub id: ChunkId,
    /// Content-addressed identifier for this chunk.
    pub chunk_hash: ChunkHash,
    /// Hash of the file that produced this chunk.
    pub file_hash: FileHash,
    /// Origin location of this chunk.
    pub source: ChunkSource,
    /// Text content with token count.
    pub content: ChunkContent,
    /// File-type-specific metadata.
    pub context: ChunkContext,
}

/// Origin location of a chunk.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct ChunkSource {
    /// Path to the source file relative to index root.
    pub file_path: IndexRelativePath,
    /// Name of the repository containing this chunk.
    pub repo_name: RepoName,
}

/// Text content with token count for size tracking.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct ChunkContent {
    /// Raw text content of the chunk.
    pub text: String,
    /// Number of tokens in the text.
    pub token_count: TokenCount,
}

/// File-type-specific metadata for chunks.
///
/// Opaque outside the chunking layer. The storage layer serializes/deserializes
/// via the `type_name` and `raw_json` fields. Chunkers downcast the inner data
/// to their concrete type using `deserialize_as`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct ChunkContext {
    type_name: String,
    raw_json: String,
}

impl ChunkContext {
    /// Create a new context from a serializable language-specific type.
    ///
    /// Uses standard JSON serialization for compatibility.
    pub fn new<T: serde::Serialize>(type_name: &str, data: &T) -> Result<Self, ChunkContextError> {
        let raw_json =
            serde_json::to_string(data).map_err(|e| ChunkContextError::Serialization {
                message: e.to_string(),
            })?;
        Ok(Self {
            type_name: type_name.to_string(),
            raw_json,
        })
    }

    /// Reconstruct a context from stored type name and JSON.
    pub fn from_stored(type_name: String, raw_json: String) -> Self {
        Self {
            type_name,
            raw_json,
        }
    }

    /// Returns the context type name (e.g. "rust_doc", "markdown").
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Returns the raw JSON representation.
    pub fn raw_json(&self) -> &str {
        &self.raw_json
    }

    /// Deserialize into a concrete type.
    pub fn deserialize_as<T: serde::de::DeserializeOwned>(&self) -> Result<T, ChunkContextError> {
        serde_json::from_str(&self.raw_json).map_err(|e| ChunkContextError::Deserialization {
            message: e.to_string(),
        })
    }
}

/// Errors that can occur when creating or deserializing ChunkContext.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChunkContextError {
    /// JSON serialization failed.
    Serialization {
        /// Error message.
        message: String,
    },
    /// JSON deserialization failed.
    Deserialization {
        /// Error message.
        message: String,
    },
}

impl std::fmt::Display for ChunkContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialization { message } => write!(f, "Failed to serialize context: {message}"),
            Self::Deserialization { message } => {
                write!(f, "Failed to deserialize context: {message}")
            }
        }
    }
}

impl std::error::Error for ChunkContextError {}

/// Visibility of a Rust item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Visible everywhere (`pub`).
    Public,
    /// Visible within the crate (`pub(crate)`).
    Crate,
    /// Visible only within the module.
    Private,
}

impl From<syn::Visibility> for Visibility {
    fn from(vis: syn::Visibility) -> Self {
        match vis {
            syn::Visibility::Public(_) => Visibility::Public,
            syn::Visibility::Restricted(r) => {
                if r.path.is_ident("crate") {
                    Visibility::Crate
                } else {
                    Visibility::Private
                }
            }
            syn::Visibility::Inherited => Visibility::Private,
        }
    }
}

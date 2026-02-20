//! Language support trait for extensible file type handling.
//!
//! Provides the [`LanguageSupport`] trait that each language implements to register
//! itself with the chunking system. Languages are discovered at runtime via `inventory`.

use std::any::Any;

use super::{ChunkingError, ChunkingStrategy};
use crate::knowledge::domain::{EmbeddingModelConfig, FilePeek};
use crate::knowledge::storage::StorageError;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Language-specific configuration.
///
/// Opaque to the framework. Each language's `LanguageSupport` knows
/// how to interpret its contents.
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageConfig {
    raw: serde_json::Value,
}

impl LanguageConfig {
    /// Creates a new language config from a serializable value.
    pub fn new<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        Ok(Self {
            raw: serde_json::to_value(value)?,
        })
    }

    /// Creates a language config from a TOML value.
    pub fn from_toml(value: &toml::Value) -> Result<Self, serde_json::Error> {
        // Convert TOML value to JSON value
        let json_str = serde_json::to_string(value)?;
        let raw: serde_json::Value = serde_json::from_str(&json_str)?;
        Ok(Self { raw })
    }

    /// Deserializes the config into a concrete type.
    pub fn deserialize_as<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.raw.clone())
    }

    /// Returns the raw JSON value.
    pub fn raw(&self) -> &serde_json::Value {
        &self.raw
    }
}

/// Self-contained language support definition.
///
/// Implementors provide everything needed to index a language:
/// file detection, chunker construction, context serialization,
/// and default indexing settings.
pub trait LanguageSupport: Send + Sync + 'static {
    /// Unique identifier used in storage (e.g. "rust_doc", "java_doc").
    fn context_type_name(&self) -> &'static str;

    /// File extensions this language handles (e.g. &["rs"], &["java"]).
    fn extensions(&self) -> &'static [&'static str];

    /// Returns true if this language can handle the given file.
    ///
    /// Default implementation checks file extension against `extensions()`.
    /// Override to add content-based detection (e.g., shebang lines).
    fn matches(&self, peek: &FilePeek) -> bool {
        peek.extension()
            .is_some_and(|ext| self.extensions().contains(&ext))
    }

    /// Construct a chunker for this language.
    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError>;

    /// Serialize a language-specific context to JSON.
    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError>;

    /// Deserialize JSON back into a language-specific context.
    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError>;

    /// Default configuration for this language.
    fn default_config(&self) -> Option<LanguageConfig>;

    /// User-facing config key used in crumbly.toml (e.g. "rust", "go").
    fn config_key(&self) -> &'static str;

    /// Whether this language is indexed by default when no config exists.
    fn enabled_by_default(&self) -> bool {
        false
    }
}

inventory::collect!(&'static dyn LanguageSupport);

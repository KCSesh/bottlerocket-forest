//! Language support registration for markdown files.

use std::any::Any;

use super::MarkdownChunker;
use crate::knowledge::chunking::{
    ChunkingError, ChunkingStrategy, LanguageConfig, LanguageSupport,
};
use crate::knowledge::domain::{EmbeddingModelConfig, MarkdownContext};
use crate::knowledge::storage::StorageError;

/// Language support registration for markdown files.
pub struct MarkdownSupport;

impl LanguageSupport for MarkdownSupport {
    fn context_type_name(&self) -> &'static str {
        "markdown"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["md"]
    }

    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        _language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError> {
        Ok(Box::new(MarkdownChunker::from_config(embedding_config)?))
    }

    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError> {
        let ctx =
            context
                .downcast_ref::<MarkdownContext>()
                .ok_or_else(|| StorageError::InvalidData {
                    message: "Expected MarkdownContext".to_string(),
                })?;
        serde_json::to_string(ctx).map_err(|e| StorageError::InvalidData {
            message: e.to_string(),
        })
    }

    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError> {
        let ctx: MarkdownContext =
            serde_json::from_str(json).map_err(|e| StorageError::InvalidData {
                message: e.to_string(),
            })?;
        Ok(Box::new(ctx))
    }

    fn default_config(&self) -> Option<LanguageConfig> {
        None
    }
}

inventory::submit!(&MarkdownSupport as &dyn LanguageSupport);

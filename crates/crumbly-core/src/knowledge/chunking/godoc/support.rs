//! Go language support registration for the chunking system.

use std::any::Any;

use snafu::ResultExt;

use super::GoDocChunker;
use super::GoDocContext;
use super::config::{GoConfig, GoFilter};
use crate::knowledge::chunking::{
    ChunkingError, ChunkingStrategy, LanguageConfig, LanguageSupport,
};
use crate::knowledge::domain::EmbeddingModelConfig;
use crate::knowledge::storage::StorageError;
use crate::knowledge::storage::repository::storage_error;

/// Go language support for the chunking system.
pub struct GoDocSupport;

impl LanguageSupport for GoDocSupport {
    fn context_type_name(&self) -> &'static str {
        "go_doc"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["go"]
    }

    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError> {
        use crate::knowledge::domain::DocLineCount;

        let filter = match language_config {
            Some(cfg) => {
                let go_config: GoConfig =
                    cfg.deserialize_as()
                        .map_err(|e| ChunkingError::ConfigError {
                            message: e.to_string(),
                        })?;
                Some(GoFilter::new(
                    go_config.visibility,
                    go_config.items,
                    DocLineCount::new(go_config.min_doc_lines),
                ))
            }
            None => None,
        };

        Ok(Box::new(GoDocChunker::from_config_with_filter(
            embedding_config,
            filter,
        )?))
    }

    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError> {
        let ctx =
            context
                .downcast_ref::<GoDocContext>()
                .ok_or_else(|| StorageError::InvalidData {
                    message: "Expected GoDocContext".to_string(),
                })?;
        serde_json::to_string(ctx).context(storage_error::SerializationSnafu)
    }

    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError> {
        let ctx: GoDocContext =
            serde_json::from_str(json).context(storage_error::SerializationSnafu)?;
        Ok(Box::new(ctx))
    }

    fn default_config(&self) -> Option<LanguageConfig> {
        LanguageConfig::new(&GoConfig::default()).ok()
    }

    fn config_key(&self) -> &'static str {
        "go"
    }

    fn enabled_by_default(&self) -> bool {
        false
    }
}

inventory::submit! {
    &GoDocSupport as &dyn LanguageSupport
}

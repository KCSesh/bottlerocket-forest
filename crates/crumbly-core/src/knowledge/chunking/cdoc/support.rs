//! C language support registration for the chunking system.

use std::any::Any;

use snafu::ResultExt;

use super::CDocChunker;
use super::CDocContext;
use super::config::{CConfig, CFilter};
use crate::knowledge::chunking::{
    ChunkingError, ChunkingStrategy, LanguageConfig, LanguageSupport,
};
use crate::knowledge::domain::EmbeddingModelConfig;
use crate::knowledge::storage::StorageError;
use crate::knowledge::storage::repository::storage_error;

/// C language support for the chunking system.
pub struct CDocSupport;

impl LanguageSupport for CDocSupport {
    fn context_type_name(&self) -> &'static str {
        "c_doc"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["c", "h"]
    }

    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError> {
        use crate::knowledge::domain::DocLineCount;

        let filter = match language_config {
            Some(cfg) => {
                let c_config: CConfig =
                    cfg.deserialize_as()
                        .map_err(|e| ChunkingError::ConfigError {
                            message: e.to_string(),
                        })?;
                Some(CFilter::new(
                    c_config.items,
                    DocLineCount::new(c_config.min_doc_lines),
                ))
            }
            None => None,
        };

        Ok(Box::new(CDocChunker::from_config_with_filter(
            embedding_config,
            filter,
        )?))
    }

    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError> {
        let ctx =
            context
                .downcast_ref::<CDocContext>()
                .ok_or_else(|| StorageError::InvalidData {
                    message: "Expected CDocContext".to_string(),
                })?;
        serde_json::to_string(ctx).context(storage_error::SerializationSnafu)
    }

    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError> {
        let ctx: CDocContext =
            serde_json::from_str(json).context(storage_error::SerializationSnafu)?;
        Ok(Box::new(ctx))
    }

    fn default_config(&self) -> Option<LanguageConfig> {
        LanguageConfig::new(&CConfig::default()).ok()
    }

    fn config_key(&self) -> &'static str {
        "c"
    }

    fn enabled_by_default(&self) -> bool {
        true
    }
}

inventory::submit! {
    &CDocSupport as &dyn LanguageSupport
}

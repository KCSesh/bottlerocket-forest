//! JavaScript/TypeScript language support registration for the chunking system.

use std::any::Any;

use snafu::ResultExt;

use super::JsDocChunker;
use super::config::{JsFilter, JsFilterConfig};
use super::context::JsDocContext;
use crate::knowledge::chunking::{
    ChunkingError, ChunkingStrategy, LanguageConfig, LanguageSupport,
};
use crate::knowledge::domain::{DocLineCount, EmbeddingModelConfig};
use crate::knowledge::storage::StorageError;
use crate::knowledge::storage::repository::storage_error;

/// JavaScript language support for the chunking system.
pub struct JsDocSupport;

impl LanguageSupport for JsDocSupport {
    fn context_type_name(&self) -> &'static str {
        "js_doc"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["js", "jsx"]
    }

    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError> {
        let filter = match language_config {
            Some(cfg) => {
                let js_config: JsFilterConfig =
                    cfg.deserialize_as()
                        .map_err(|e| ChunkingError::ConfigError {
                            message: e.to_string(),
                        })?;
                Some(JsFilter::new(
                    js_config.visibility,
                    js_config.items.iter().map(|i| i.to_domain_type()).collect(),
                    DocLineCount::new(js_config.min_doc_lines),
                ))
            }
            None => None,
        };

        Ok(Box::new(JsDocChunker::for_javascript_with_filter(
            embedding_config,
            filter,
        )?))
    }

    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError> {
        let ctx =
            context
                .downcast_ref::<JsDocContext>()
                .ok_or_else(|| StorageError::InvalidData {
                    message: "Expected JsDocContext".to_string(),
                })?;
        serde_json::to_string(ctx).context(storage_error::SerializationSnafu)
    }

    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError> {
        let ctx: JsDocContext =
            serde_json::from_str(json).context(storage_error::SerializationSnafu)?;
        Ok(Box::new(ctx))
    }

    fn default_config(&self) -> Option<LanguageConfig> {
        LanguageConfig::new(&JsFilterConfig::default()).ok()
    }

    fn config_key(&self) -> &'static str {
        "javascript"
    }

    fn enabled_by_default(&self) -> bool {
        false
    }
}

inventory::submit! {
    &JsDocSupport as &dyn LanguageSupport
}

/// TypeScript language support for the chunking system.
pub struct TsDocSupport;

impl LanguageSupport for TsDocSupport {
    fn context_type_name(&self) -> &'static str {
        "js_doc"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "tsx"]
    }

    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError> {
        let filter = match language_config {
            Some(cfg) => {
                let ts_config: JsFilterConfig =
                    cfg.deserialize_as()
                        .map_err(|e| ChunkingError::ConfigError {
                            message: e.to_string(),
                        })?;
                Some(JsFilter::new(
                    ts_config.visibility,
                    ts_config.items.iter().map(|i| i.to_domain_type()).collect(),
                    DocLineCount::new(ts_config.min_doc_lines),
                ))
            }
            None => None,
        };

        Ok(Box::new(JsDocChunker::for_typescript_with_filter(
            embedding_config,
            filter,
        )?))
    }

    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError> {
        let ctx =
            context
                .downcast_ref::<JsDocContext>()
                .ok_or_else(|| StorageError::InvalidData {
                    message: "Expected JsDocContext".to_string(),
                })?;
        serde_json::to_string(ctx).context(storage_error::SerializationSnafu)
    }

    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError> {
        let ctx: JsDocContext =
            serde_json::from_str(json).context(storage_error::SerializationSnafu)?;
        Ok(Box::new(ctx))
    }

    fn default_config(&self) -> Option<LanguageConfig> {
        LanguageConfig::new(&JsFilterConfig::default()).ok()
    }

    fn config_key(&self) -> &'static str {
        "typescript"
    }

    fn enabled_by_default(&self) -> bool {
        false
    }
}

inventory::submit! {
    &TsDocSupport as &dyn LanguageSupport
}

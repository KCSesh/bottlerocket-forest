//! Shell language support registration for the chunking system.

use std::any::Any;

use snafu::ResultExt;

use super::ShellDocChunker;
use super::config::{ShellConfig, ShellFilter};
use super::context::ShellDocContext;
use crate::knowledge::chunking::{
    ChunkingError, ChunkingStrategy, LanguageConfig, LanguageSupport,
};
use crate::knowledge::domain::{EmbeddingModelConfig, FilePeek};
use crate::knowledge::storage::StorageError;
use crate::knowledge::storage::repository::storage_error;

/// Shell language support for the chunking system.
pub struct ShellDocSupport;

impl LanguageSupport for ShellDocSupport {
    fn context_type_name(&self) -> &'static str {
        "shell"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["sh"]
    }

    fn matches(&self, peek: &FilePeek) -> bool {
        if peek
            .extension()
            .is_some_and(|ext| self.extensions().contains(&ext))
        {
            return true;
        }
        ShellDocChunker::is_shell_shebang(peek.shebang())
    }

    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError> {
        use crate::knowledge::domain::DocLineCount;

        let filter = match language_config {
            Some(cfg) => {
                let shell_config: ShellConfig =
                    cfg.deserialize_as()
                        .map_err(|e| ChunkingError::ConfigError {
                            message: e.to_string(),
                        })?;
                Some(ShellFilter::new(
                    shell_config.items,
                    DocLineCount::new(shell_config.min_doc_lines),
                ))
            }
            None => None,
        };

        Ok(Box::new(ShellDocChunker::from_config_with_filter(
            embedding_config,
            filter,
        )?))
    }

    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError> {
        let ctx =
            context
                .downcast_ref::<ShellDocContext>()
                .ok_or_else(|| StorageError::InvalidData {
                    message: "Expected ShellDocContext".to_string(),
                })?;
        serde_json::to_string(ctx).context(storage_error::SerializationSnafu)
    }

    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError> {
        let ctx: ShellDocContext =
            serde_json::from_str(json).context(storage_error::SerializationSnafu)?;
        Ok(Box::new(ctx))
    }

    fn default_config(&self) -> Option<LanguageConfig> {
        LanguageConfig::new(&ShellConfig::default()).ok()
    }

    fn config_key(&self) -> &'static str {
        "shell"
    }

    fn enabled_by_default(&self) -> bool {
        true
    }
}

inventory::submit! {
    &ShellDocSupport as &dyn LanguageSupport
}

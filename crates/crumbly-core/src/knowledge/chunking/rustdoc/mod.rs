//! Extracts and chunks Rust documentation comments for semantic search.
//!
//! Implements chunking for Rust source files by extracting doc comments
//! (`///` and `//!`) while ignoring inline comments (`//`). Uses `syn` for parsing
//! and `text-splitter` for token-aware chunking with overlap.
//!
//! Doc comments are extracted from:
//! - Module-level comments (`//!`)
//! - Functions, structs, enums, traits
//! - Methods in impl blocks
//!
//! Each chunk preserves metadata including item name, visibility, and function signatures.
//!
//! ## Module Organization
//!
//! - `extraction` - Doc comment extraction logic for different Rust item types

mod extraction;

use std::any::Any;
use std::path::Path;

use serde::{Deserialize, Serialize};
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;

use super::language::{LanguageConfig, LanguageSupport};
use super::{ChunkingError, ChunkingInput, ChunkingStrategy};
use crate::knowledge::domain::EmbeddingModelConfig;
use crate::knowledge::domain::{Chunk, DocLineCount, ItemName, RustDocContext, Visibility};
use crate::knowledge::storage::StorageError;

pub(crate) use extraction::DocExtractor;

/// Categories of Rust language items that can be filtered during indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustItemType {
    /// Module declarations.
    #[serde(rename = "modules")]
    Module,
    /// Function definitions.
    #[serde(rename = "functions")]
    Function,
    /// Struct definitions.
    #[serde(rename = "structs")]
    Struct,
    /// Enum definitions.
    #[serde(rename = "enums")]
    Enum,
    /// Trait definitions.
    #[serde(rename = "traits")]
    Trait,
    /// Impl blocks.
    #[serde(rename = "impls")]
    Impl,
    /// Type alias definitions.
    #[serde(rename = "type-aliases")]
    TypeAlias,
    /// Constant definitions.
    #[serde(rename = "constants")]
    Constant,
}

/// Filtering rules for Rust source code indexing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RustFilter {
    #[serde(default = "default_visibility")]
    visibility: Vec<Visibility>,
    #[serde(default = "default_items")]
    items: Vec<RustItemType>,
    #[serde(default = "default_min_doc_lines")]
    min_doc_lines: DocLineCount,
}

fn default_min_doc_lines() -> DocLineCount {
    DocLineCount::new(0)
}

fn default_visibility() -> Vec<Visibility> {
    vec![Visibility::Public]
}

fn default_items() -> Vec<RustItemType> {
    vec![
        RustItemType::Module,
        RustItemType::Function,
        RustItemType::Struct,
        RustItemType::Enum,
        RustItemType::Trait,
        RustItemType::Impl,
        RustItemType::TypeAlias,
        RustItemType::Constant,
    ]
}

impl RustFilter {
    /// Create a filter with visibility, item types, and minimum documentation length.
    pub fn new(
        visibility: Vec<Visibility>,
        items: Vec<RustItemType>,
        min_doc_lines: DocLineCount,
    ) -> Self {
        Self {
            visibility,
            items,
            min_doc_lines,
        }
    }

    /// Determine whether a Rust item should be indexed based on filter criteria.
    pub fn should_index(
        &self,
        visibility: &Visibility,
        item_type: &RustItemType,
        doc_lines: DocLineCount,
    ) -> bool {
        doc_lines >= self.min_doc_lines
            && self.visibility.contains(visibility)
            && self.items.contains(item_type)
    }
}

impl Default for RustFilter {
    fn default() -> Self {
        Self {
            visibility: default_visibility(),
            items: default_items(),
            min_doc_lines: DocLineCount::new(0),
        }
    }
}

/// Extracts and chunks Rust documentation comments.
///
/// Uses `syn` to parse Rust syntax and extract `///` and `//!` doc comments,
/// then uses `text-splitter` to chunk long comments with token-based overlap.
pub struct RustDocChunker {
    splitter: TextSplitter<Tokenizer>,
    tokenizer: Tokenizer,
    filter: Option<RustFilter>,
}

impl RustDocChunker {
    /// Initializes chunker with tokenizer and splitter configured for the embedding model.
    pub fn from_config(config: &EmbeddingModelConfig) -> Result<Self, ChunkingError> {
        Self::from_config_with_filter(config, None)
    }

    /// Initializes chunker with tokenizer, splitter, and filtering rules for Rust items.
    pub fn from_config_with_filter(
        config: &EmbeddingModelConfig,
        filter: Option<RustFilter>,
    ) -> Result<Self, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let tokenizer = Tokenizer::from_pretrained(&config.model_name, None)
            .map_err(|e| e as Box<dyn std::error::Error + Send + Sync>)
            .context(TokenizerInitSnafu)?;

        let tokenizer_for_counting = Tokenizer::from_pretrained(&config.model_name, None)
            .map_err(|e| e as Box<dyn std::error::Error + Send + Sync>)
            .context(TokenizerInitSnafu)?;

        let splitter = TextSplitter::new(
            #[expect(clippy::expect_used)]
            ChunkConfig::new(config.max_tokens)
                .with_sizer(tokenizer)
                .with_overlap(config.overlap_tokens)
                .expect("overlap configuration should be valid"),
        );

        Ok(Self {
            splitter,
            tokenizer: tokenizer_for_counting,
            filter,
        })
    }
}

impl ChunkingStrategy for RustDocChunker {
    fn supports(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "rs")
            .unwrap_or(false)
    }

    fn chunk(&self, input: &ChunkingInput) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;
        use syn::File;

        let content = input.content.as_ref();
        let file_path = input.source.file_path.to_string();

        let syntax_tree: File = syn::parse_str(content)
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu {
                file_path: file_path.clone(),
            })?;

        let extractor = DocExtractor {
            splitter: &self.splitter,
            tokenizer: &self.tokenizer,
            filter: self.filter.as_ref(),
        };

        let mut chunks = Vec::new();

        let module_doc = DocExtractor::extract_doc_text(&syntax_tree.attrs);
        if !module_doc.trim().is_empty() {
            chunks.extend(
                extractor.create_chunks(
                    &module_doc,
                    ItemName::try_new("module")
                        .map_err(crate::knowledge::error::box_err)
                        .context(ParseSnafu {
                            file_path: file_path.clone(),
                        })?,
                    Visibility::Public,
                    None,
                    RustItemType::Module,
                    input,
                )?,
            );
        }

        for item in &syntax_tree.items {
            chunks.extend(extractor.extract_item_doc_chunks(item, input)?);
        }

        Ok(chunks)
    }
}

/// Self-registering Rust language support.
pub struct RustDocSupport;

inventory::submit! {
    &RustDocSupport as &dyn LanguageSupport
}

impl LanguageSupport for RustDocSupport {
    fn context_type_name(&self) -> &'static str {
        "rust_doc"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError> {
        let filter = language_config
            .map(|cfg| cfg.deserialize_as::<RustFilter>())
            .transpose()
            .map_err(|e| ChunkingError::ConfigError {
                message: e.to_string(),
            })?;
        Ok(Box::new(RustDocChunker::from_config_with_filter(
            embedding_config,
            filter,
        )?))
    }

    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError> {
        let ctx =
            context
                .downcast_ref::<RustDocContext>()
                .ok_or_else(|| StorageError::InvalidData {
                    message: "Expected RustDocContext".to_string(),
                })?;
        serde_json::to_string(ctx).map_err(|e| StorageError::InvalidData {
            message: format!("Failed to serialize RustDocContext: {e}"),
        })
    }

    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError> {
        let ctx: RustDocContext =
            serde_json::from_str(json).map_err(|e| StorageError::InvalidData {
                message: format!("Failed to deserialize RustDocContext: {e}"),
            })?;
        Ok(Box::new(ctx))
    }

    fn default_config(&self) -> Option<LanguageConfig> {
        LanguageConfig::new(&RustFilter::default()).ok()
    }

    fn config_key(&self) -> &'static str {
        "rust"
    }

    fn enabled_by_default(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::chunking::ChunkingStrategy;
    use crate::knowledge::domain::EmbeddingModelConfig;
    use crate::knowledge::domain::{
        ChunkSource, ChunkableContent, FileHash, IndexRelativePath, ItemName, RepoName,
        RustDocContext,
    };
    use test_case::test_case;

    fn test_config() -> EmbeddingModelConfig {
        EmbeddingModelConfig::default()
    }

    fn create_test_input(content: &str) -> ChunkingInput {
        ChunkingInput {
            content: ChunkableContent::new(content.to_string()),
            source: ChunkSource::builder()
                .file_path(IndexRelativePath::try_new("test.rs").unwrap())
                .repo_name(RepoName::try_new("test-repo").unwrap())
                .build(),
            file_hash: FileHash::new([0u8; 32]),
        }
    }

    #[test_case(r#"
/// Processes data according to specified parameters
pub fn process_data(input: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    Ok(input.to_vec())
}
"#, "Processes data" ; "fn_item")]
    #[test_case(r#"
/// Configuration for the application
pub struct Config {
    pub api_key: String,
}
"#, "Configuration" ; "struct_item")]
    #[test_case(r#"
//! This module contains utilities for parsing configuration files.

pub fn parse_config() {}
"#, "utilities for parsing" ; "module_item")]
    #[test_case(r#"
/// Repository for data persistence
pub trait Repository {
    fn save(&self);
}
"#, "Repository for data" ; "trait_item")]
    fn test_extracts_doc_comments_for_item_types(content: &str, expected_text: &str) {
        let input = create_test_input(content);
        let config = test_config();
        let chunker = RustDocChunker::from_config(&config).unwrap();

        let chunks = chunker.chunk(&input).unwrap();

        assert!(!chunks.is_empty());
        assert!(chunks[0].content.text.contains(expected_text));
    }

    #[test]
    fn test_ignores_items_without_doc_comments() {
        let content = r#"
pub fn no_docs() {}

// Regular comment
pub struct NoDocs {}
"#;
        let input = create_test_input(content);
        let config = test_config();
        let chunker = RustDocChunker::from_config(&config).unwrap();

        let chunks = chunker.chunk(&input).unwrap();

        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_empty_rust_file() {
        let input = create_test_input("");
        let config = test_config();
        let chunker = RustDocChunker::from_config(&config).unwrap();

        let chunks = chunker.chunk(&input).unwrap();

        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_invalid_rust_syntax() {
        let content = "pub fn incomplete(";
        let input = create_test_input(content);
        let config = test_config();
        let chunker = RustDocChunker::from_config(&config).unwrap();

        let result = chunker.chunk(&input);

        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_items_create_separate_chunks() {
        let content = r#"
/// First function
pub fn first() {}

/// Second function
pub fn second() {}

/// Third function
pub fn third() {}
"#;
        let input = create_test_input(content);
        let config = test_config();
        let chunker = RustDocChunker::from_config(&config).unwrap();

        let chunks = chunker.chunk(&input).unwrap();

        assert_eq!(chunks.len(), 3);

        let names: Vec<_> = chunks
            .iter()
            .filter_map(|c| {
                c.context
                    .deserialize_as::<RustDocContext>()
                    .ok()
                    .map(|ctx| ctx.item_name.clone())
            })
            .collect();

        assert!(names.contains(&ItemName::try_new("first").unwrap()));
        assert!(names.contains(&ItemName::try_new("second").unwrap()));
        assert!(names.contains(&ItemName::try_new("third").unwrap()));
    }

    #[test]
    fn test_rust_filter_should_index_checks_doc_lines() {
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Function],
            DocLineCount::new(3),
        );

        let short_doc = filter.should_index(
            &Visibility::Public,
            &RustItemType::Function,
            DocLineCount::new(2),
        );
        let long_doc = filter.should_index(
            &Visibility::Public,
            &RustItemType::Function,
            DocLineCount::new(5),
        );

        assert!(!short_doc);
        assert!(long_doc);
    }

    #[test]
    fn test_rust_filter_should_index_checks_visibility() {
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Function],
            DocLineCount::new(0),
        );

        let public = filter.should_index(
            &Visibility::Public,
            &RustItemType::Function,
            DocLineCount::new(100),
        );
        let private = filter.should_index(
            &Visibility::Private,
            &RustItemType::Function,
            DocLineCount::new(100),
        );

        assert!(public);
        assert!(!private);
    }

    #[test]
    fn test_rust_filter_should_index_checks_item_type() {
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Struct],
            DocLineCount::new(0),
        );

        let struct_item = filter.should_index(
            &Visibility::Public,
            &RustItemType::Struct,
            DocLineCount::new(100),
        );
        let function_item = filter.should_index(
            &Visibility::Public,
            &RustItemType::Function,
            DocLineCount::new(100),
        );

        assert!(struct_item);
        assert!(!function_item);
    }
}

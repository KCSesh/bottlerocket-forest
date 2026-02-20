//! Extracts and chunks JavaScript/TypeScript documentation comments for semantic search.
//!
//! Implements chunking for JS/TS source files by extracting doc comments
//! (JSDoc, line comments, or block comments preceding declarations). Uses `tree-sitter`
//! for parsing and `text-splitter` for token-aware chunking with overlap.
//!
//! Doc comments are extracted from:
//! - Functions, classes, variables
//! - TypeScript interfaces, type aliases, enums
//! - Module-level documentation
//!
//! Each chunk preserves metadata including item name, visibility, and signatures.

mod config;
mod context;
mod extraction;
mod support;
#[cfg(test)]
mod tests;

pub use config::{JsFilter, JsFilterConfig, JsFilterItemType};
pub use context::{JsDocContext, JsItemType, JsVisibility};
pub use support::{JsDocSupport, TsDocSupport};

use std::path::Path;
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;
use tree_sitter::Parser;

use self::extraction::{
    extract_doc_comment, extract_identifier, extract_module_doc, get_item_type, get_visibility,
    node_kinds, to_filter_type,
};
use super::{ChunkingError, ChunkingInput, ChunkingStrategy};
use crate::knowledge::domain::EmbeddingModelConfig;

use crate::knowledge::domain::{
    Chunk, ChunkContent, ChunkContext, ChunkHash, DocLineCount, FilePeek, ItemName, PackageName,
    Signature, TokenCount,
};

const MODULE_ITEM_NAME: &str = "module";

/// File extensions supported by this chunker.
const JS_EXTENSIONS: &[&str] = &["js", "jsx"];
const TS_EXTENSIONS: &[&str] = &["ts", "tsx"];

/// Metadata for creating a chunk from a JS/TS declaration.
struct ChunkMetadata {
    item_name: ItemName,
    visibility: JsVisibility,
    signature: Option<Signature>,
    item_type: JsItemType,
}

/// Context for processing JS/TS nodes during chunking.
struct ProcessingContext<'a> {
    source_bytes: &'a [u8],
    module_name: &'a Option<PackageName>,
    input: &'a ChunkingInput,
}

/// Extracts and chunks JavaScript/TypeScript documentation comments.
pub struct JsDocChunker {
    splitter: TextSplitter<Tokenizer>,
    tokenizer: Tokenizer,
    filter: Option<JsFilter>,
    extensions: &'static [&'static str],
}

impl JsDocChunker {
    /// Initializes chunker for JavaScript files.
    pub fn for_javascript(config: &EmbeddingModelConfig) -> Result<Self, ChunkingError> {
        Self::new(config, None, JS_EXTENSIONS)
    }

    /// Initializes chunker for TypeScript files.
    pub fn for_typescript(config: &EmbeddingModelConfig) -> Result<Self, ChunkingError> {
        Self::new(config, None, TS_EXTENSIONS)
    }

    /// Initializes chunker with filter for JavaScript files.
    pub fn for_javascript_with_filter(
        config: &EmbeddingModelConfig,
        filter: Option<JsFilter>,
    ) -> Result<Self, ChunkingError> {
        Self::new(config, filter, JS_EXTENSIONS)
    }

    /// Initializes chunker with filter for TypeScript files.
    pub fn for_typescript_with_filter(
        config: &EmbeddingModelConfig,
        filter: Option<JsFilter>,
    ) -> Result<Self, ChunkingError> {
        Self::new(config, filter, TS_EXTENSIONS)
    }

    fn new(
        config: &EmbeddingModelConfig,
        filter: Option<JsFilter>,
        extensions: &'static [&'static str],
    ) -> Result<Self, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let tokenizer = Tokenizer::from_pretrained(&config.model_name, None)
            .map_err(|e| e as Box<dyn std::error::Error + Send + Sync>)
            .context(TokenizerInitSnafu)?;

        let tokenizer_for_counting = tokenizer.clone();

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
            extensions,
        })
    }

    fn should_index(
        &self,
        visibility: &crate::knowledge::domain::Visibility,
        item_type: JsItemType,
        doc: &str,
    ) -> bool {
        let Some(ref filter) = self.filter else {
            return true;
        };
        let filter_type = to_filter_type(item_type);
        filter.should_index(
            visibility,
            &filter_type,
            DocLineCount::new(doc.lines().count()),
        )
    }

    fn get_language(&self, ext: Option<&str>) -> tree_sitter::Language {
        match ext {
            Some("tsx") => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Some("ts") => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            _ => tree_sitter_javascript::LANGUAGE.into(),
        }
    }

    fn process_declaration(
        &self,
        node: tree_sitter::Node,
        item_type: JsItemType,
        ctx: &ProcessingContext,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let Some(doc) = extract_doc_comment(node, ctx.source_bytes) else {
            return Ok(Vec::new());
        };
        let Some(name) = extract_identifier(node, ctx.source_bytes) else {
            return Ok(Vec::new());
        };
        let (js_vis, vis) = get_visibility(node);
        if !self.should_index(&vis, item_type, &doc) {
            return Ok(Vec::new());
        }
        let signature = node
            .utf8_text(ctx.source_bytes)
            .ok()
            .and_then(extract_signature)
            .and_then(|s| Signature::try_new(&s).ok());
        let metadata = ChunkMetadata {
            item_name: ItemName::try_new(&name)
                .map_err(crate::knowledge::error::box_err)
                .context(ParseSnafu {
                    file_path: ctx.input.source.file_path.to_string(),
                })?,
            visibility: js_vis,
            signature,
            item_type,
        };
        self.create_chunks(&doc, metadata, ctx.module_name, ctx.input)
    }

    fn process_module_doc(
        &self,
        root: tree_sitter::Node,
        ctx: &ProcessingContext,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let Some(doc) = extract_module_doc(root, ctx.source_bytes) else {
            return Ok(Vec::new());
        };

        let metadata = ChunkMetadata {
            item_name: ItemName::try_new(MODULE_ITEM_NAME)
                .map_err(crate::knowledge::error::box_err)
                .context(ParseSnafu {
                    file_path: ctx.input.source.file_path.to_string(),
                })?,
            visibility: JsVisibility::Exported,
            signature: None,
            item_type: JsItemType::Module,
        };
        self.create_chunks(&doc, metadata, ctx.module_name, ctx.input)
    }

    fn create_chunks(
        &self,
        text: &str,
        metadata: ChunkMetadata,
        module_name: &Option<PackageName>,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let chunks: Vec<_> = self.splitter.chunks(text).collect();
        let mut result = Vec::new();

        for chunk_text in chunks {
            let encoding = self
                .tokenizer
                .encode(chunk_text, false)
                .context(ParseSnafu {
                    file_path: input.source.file_path.to_string(),
                })?;

            let token_count = encoding.len().max(1);
            let token_count = TokenCount::try_new(token_count)
                .map_err(crate::knowledge::error::box_err)
                .context(ParseSnafu {
                    file_path: input.source.file_path.to_string(),
                })?;

            let chunk_hash = ChunkHash::from_text(chunk_text);
            let chunk = Chunk::builder()
                .id(crate::knowledge::domain::ChunkId::new(uuid::Uuid::new_v4()))
                .chunk_hash(chunk_hash)
                .file_hash(input.file_hash)
                .source(input.source.clone())
                .content(
                    ChunkContent::builder()
                        .text(chunk_text.to_string())
                        .token_count(token_count)
                        .build(),
                )
                .context(
                    ChunkContext::new(
                        "js_doc",
                        &JsDocContext::builder()
                            .item_name(metadata.item_name.clone())
                            .visibility(metadata.visibility)
                            .maybe_signature(metadata.signature.clone())
                            .item_type(metadata.item_type)
                            .maybe_module_name(module_name.as_ref().cloned())
                            .build(),
                    )
                    .map_err(crate::knowledge::error::box_err)
                    .context(ParseSnafu {
                        file_path: input.source.file_path.to_string(),
                    })?,
                )
                .build();

            result.push(chunk);
        }

        Ok(result)
    }
}

fn extract_signature(text: &str) -> Option<String> {
    text.lines().next().map(|s| s.trim().to_string())
}

impl ChunkingStrategy for JsDocChunker {
    fn supports(&self, peek: &FilePeek) -> bool {
        peek.extension()
            .is_some_and(|ext| self.extensions.contains(&ext))
    }

    fn chunk(&self, input: &ChunkingInput) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let content = input.content.as_ref();
        let file_path = input.source.file_path.to_string();

        let mut parser = Parser::new();
        let ext = std::path::Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str());
        let language = self.get_language(ext);
        parser
            .set_language(&language)
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu {
                file_path: file_path.clone(),
            })?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| ChunkingError::ParseError {
                file_path: file_path.clone(),
                source: "Failed to parse JS/TS source".into(),
            })?;

        let root = tree.root_node();
        let source_bytes = content.as_bytes();
        let mut chunks = Vec::new();

        // Extract module name from file path
        let module_name = Path::new(&file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| PackageName::try_new(s).ok());

        let ctx = ProcessingContext {
            source_bytes,
            module_name: &module_name,
            input,
        };

        // Process module-level documentation
        chunks.extend(self.process_module_doc(root, &ctx)?);

        // Process all nodes
        process_node(self, root, &ctx, &mut chunks)?;

        Ok(chunks)
    }
}

fn process_node(
    chunker: &JsDocChunker,
    node: tree_sitter::Node,
    ctx: &ProcessingContext,
    chunks: &mut Vec<Chunk>,
) -> Result<(), ChunkingError> {
    // Handle export statements - process the inner declaration
    if node.kind() == node_kinds::EXPORT_STATEMENT {
        for child in node.children(&mut node.walk()) {
            if let Some(item_type) = get_item_type(child.kind()) {
                chunks.extend(chunker.process_declaration(child, item_type, ctx)?);
            }
        }
    } else if let Some(item_type) = get_item_type(node.kind()) {
        chunks.extend(chunker.process_declaration(node, item_type, ctx)?);
    }

    for child in node.children(&mut node.walk()) {
        process_node(chunker, child, ctx, chunks)?;
    }
    Ok(())
}

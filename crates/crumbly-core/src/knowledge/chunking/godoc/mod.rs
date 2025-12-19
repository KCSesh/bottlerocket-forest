//! Extracts and chunks Go documentation comments for semantic search.
//!
//! Implements chunking for Go source files by extracting doc comments
//! (comments immediately preceding declarations). Uses `tree-sitter` for parsing
//! and `text-splitter` for token-aware chunking with overlap.
//!
//! Doc comments are extracted from:
//! - Package-level comments
//! - Functions, methods, types, constants, variables
//!
//! Each chunk preserves metadata including item name, visibility, and signatures.

mod extraction;

use std::path::Path;
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;
use tree_sitter::Parser;

use self::extraction::{node_kinds, extract_doc_comment, extract_identifier, get_visibility, determine_type_kind, to_filter_type};
use super::{ChunkingError, ChunkingInput, ChunkingStrategy};
use crate::knowledge::domain::{
    Chunk, ChunkContent, ChunkContext, ChunkHash, GoDocContext, GoItemType, GoVisibility, ItemName,
    PackageName, Signature, TokenCount,
};
use crate::knowledge::domain::EmbeddingModelConfig;
use crate::knowledge::indexing::GoFilter;

const PACKAGE_ITEM_NAME: &str = "package";

/// Extracts and chunks Go documentation comments.
pub struct GoDocChunker {
    splitter: TextSplitter<Tokenizer>,
    tokenizer: Tokenizer,
    filter: Option<GoFilter>,
}

impl GoDocChunker {
    /// Initializes chunker with tokenizer and splitter configured for the embedding model.
    pub fn from_config(config: &EmbeddingModelConfig) -> Result<Self, ChunkingError> {
        Self::from_config_with_filter(config, None)
    }

    /// Initializes chunker with tokenizer, splitter, and filtering rules for Go items.
    pub fn from_config_with_filter(
        config: &EmbeddingModelConfig,
        filter: Option<GoFilter>,
    ) -> Result<Self, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let tokenizer = Tokenizer::from_pretrained(&config.model_name, None)
            .map_err(|e| e as Box<dyn std::error::Error + Send + Sync>)
            .context(TokenizerInitSnafu)?;

        let tokenizer_for_counting = tokenizer.clone();

        let splitter = TextSplitter::new(
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

    fn should_index(&self, name: &str, item_type: GoItemType, doc: &str) -> bool {
        let Some(ref filter) = self.filter else {
            return true;
        };
        let (_, visibility) = get_visibility(name);
        let filter_type = to_filter_type(item_type);
        filter.should_index(&visibility, &filter_type, doc.lines().count())
    }

    fn create_chunks(
        &self,
        text: &str,
        item_name: ItemName,
        visibility: GoVisibility,
        signature: Option<Signature>,
        item_type: GoItemType,
        package_name: &Option<PackageName>,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let chunks: Vec<_> = self.splitter.chunks(text).collect();
        let mut result = Vec::new();

        for chunk_text in chunks {
            let encoding = self.tokenizer.encode(chunk_text, false)
                .context(ParseSnafu { file_path: input.source.file_path.to_string() })?;
            
            let token_count = encoding.len().max(1);
            let token_count = TokenCount::try_new(token_count)
                .map_err(crate::knowledge::error::box_err)
                .context(ParseSnafu { file_path: input.source.file_path.to_string() })?;

            let chunk_hash = ChunkHash::from_text(chunk_text);
            let chunk = Chunk::builder()
                .id(crate::knowledge::domain::ChunkId::new(uuid::Uuid::new_v4()))
                .chunk_hash(chunk_hash)
                .file_hash(input.file_hash.clone())
                .source(input.source.clone())
                .content(ChunkContent::builder()
                    .text(chunk_text.to_string())
                    .token_count(token_count)
                    .build())
                .context(ChunkContext::GoDoc(GoDocContext::builder()
                    .item_name(item_name.clone())
                    .visibility(visibility)
                    .maybe_signature(signature.clone())
                    .item_type(item_type)
                    .maybe_package_name(package_name.as_ref().cloned())
                    .build()))
                .build();

            result.push(chunk);
        }

        Ok(result)
    }
}

impl ChunkingStrategy for GoDocChunker {
    fn supports(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "go")
            .unwrap_or(false)
    }

    fn chunk(&self, input: &ChunkingInput) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let content = input.content.as_ref();
        let file_path = input.source.file_path.to_string();

        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu { file_path: file_path.clone() })?;

        let tree = parser.parse(content, None)
            .ok_or_else(|| ChunkingError::ParseError {
                file_path: file_path.clone(),
                source: "Failed to parse Go source".into(),
            })?;

        let root = tree.root_node();
        let source_bytes = content.as_bytes();
        let mut chunks = Vec::new();
        let mut package_name = None;

        for node in root.children(&mut root.walk()) {
            match node.kind() {
                node_kinds::PACKAGE_CLAUSE => {
                    if let Some(pkg_node) = node.child_by_field_name("name").or_else(|| node.children(&mut node.walk()).find(|c| c.kind() == "package_identifier")) {
                        package_name = pkg_node.utf8_text(source_bytes).ok().and_then(|s| PackageName::try_new(s).ok());
                    }
                    if let Some(doc) = extract_doc_comment(node, source_bytes) {
                        chunks.extend(self.create_chunks(
                            &doc,
                            ItemName::try_new(PACKAGE_ITEM_NAME).map_err(crate::knowledge::error::box_err).context(ParseSnafu { file_path: file_path.clone() })?,
                            GoVisibility::Exported,
                            None,
                            GoItemType::Package,
                            &package_name,
                            input,
                        )?);
                    }
                }
                node_kinds::FUNCTION_DECLARATION | node_kinds::METHOD_DECLARATION => {
                    let Some(doc) = extract_doc_comment(node, source_bytes) else { continue };
                    let Some(name) = extract_identifier(node, source_bytes) else {
                        // Identifier not found, skipping node
                        continue;
                    };
                    let item_type = if node.kind() == node_kinds::METHOD_DECLARATION { GoItemType::Method } else { GoItemType::Function };
                    if !self.should_index(&name, item_type, &doc) { continue; }
                    let (go_vis, _) = get_visibility(&name);
                    chunks.extend(self.create_chunks(
                        &doc,
                        ItemName::try_new(&name).map_err(crate::knowledge::error::box_err).context(ParseSnafu { file_path: file_path.clone() })?,
                        go_vis,
                        node.utf8_text(source_bytes).ok().and_then(|s| Signature::try_new(s).ok()),
                        item_type,
                        &package_name,
                        input,
                    )?);
                }
                node_kinds::TYPE_DECLARATION => {
                    let Some(doc) = extract_doc_comment(node, source_bytes) else { continue };
                    let Some(name) = extract_identifier(node, source_bytes) else {
                        // Identifier not found, skipping node
                        continue;
                    };
                    let item_type = determine_type_kind(node);
                    if !self.should_index(&name, item_type, &doc) { continue; }
                    let (go_vis, _) = get_visibility(&name);
                    chunks.extend(self.create_chunks(
                        &doc,
                        ItemName::try_new(&name).map_err(crate::knowledge::error::box_err).context(ParseSnafu { file_path: file_path.clone() })?,
                        go_vis,
                        None,
                        item_type,
                        &package_name,
                        input,
                    )?);
                }
                node_kinds::CONST_DECLARATION | node_kinds::VAR_DECLARATION => {
                    let Some(doc) = extract_doc_comment(node, source_bytes) else { continue };
                    let Some(name) = extract_identifier(node, source_bytes) else {
                        // Identifier not found, skipping node
                        continue;
                    };
                    let item_type = if node.kind() == node_kinds::CONST_DECLARATION { GoItemType::Const } else { GoItemType::Var };
                    if !self.should_index(&name, item_type, &doc) { continue; }
                    let (go_vis, _) = get_visibility(&name);
                    chunks.extend(self.create_chunks(
                        &doc,
                        ItemName::try_new(&name).map_err(crate::knowledge::error::box_err).context(ParseSnafu { file_path: file_path.clone() })?,
                        go_vis,
                        None,
                        item_type,
                        &package_name,
                        input,
                    )?);
                }
                _ => {}
            }
        }

        Ok(chunks)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::chunking::ChunkingStrategy;
    use crate::knowledge::domain::{
        ChunkContext, ChunkSource, ChunkableContent, FileHash, IndexRelativePath, RepoName,
    };

    fn test_config() -> EmbeddingModelConfig {
        EmbeddingModelConfig::default()
    }

    fn make_input(content: &str) -> ChunkingInput {
        ChunkingInput {
            content: ChunkableContent::new(content.to_string()),
            source: ChunkSource::builder()
                .file_path(IndexRelativePath::try_new("test.go").unwrap())
                .repo_name(RepoName::try_new("test").unwrap())
                .build(),
            file_hash: FileHash::new([0u8; 32]),
        }
    }

    #[test]
    fn test_supports_go_files() {
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        assert!(chunker.supports(Path::new("main.go")));
        assert!(chunker.supports(Path::new("pkg/util.go")));
        assert!(!chunker.supports(Path::new("main.rs")));
        assert!(!chunker.supports(Path::new("main.go.bak")));
    }

    #[test]
    fn test_extracts_function_doc() {
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(r#"
package main

// Hello prints a greeting.
// It takes a name parameter.
func Hello(name string) {
    println("Hello, " + name)
}
"#);
        let chunks = chunker.chunk(&input).unwrap();
        assert!(!chunks.is_empty());
        let ctx = match &chunks[0].context {
            ChunkContext::GoDoc(ctx) => ctx,
            _ => panic!("Expected GoDoc context"),
        };
        assert_eq!(ctx.item_name.to_string().as_str(), "Hello");
        assert_eq!(ctx.visibility, GoVisibility::Exported);
        assert_eq!(ctx.item_type, GoItemType::Function);
    }

    #[test]
    fn test_extracts_unexported_function() {
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(r#"
package main

// helper is an internal function.
func helper() {}
"#);
        let chunks = chunker.chunk(&input).unwrap();
        assert!(!chunks.is_empty());
        let ctx = match &chunks[0].context {
            ChunkContext::GoDoc(ctx) => ctx,
            _ => panic!("Expected GoDoc context"),
        };
        assert_eq!(ctx.item_name.to_string().as_str(), "helper");
        assert_eq!(ctx.visibility, GoVisibility::Unexported);
    }

    #[test]
    fn test_extracts_struct_doc() {
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(r#"
package main

// Config holds configuration.
type Config struct {
    Name string
}
"#);
        let chunks = chunker.chunk(&input).unwrap();
        assert!(!chunks.is_empty());
        let ctx = match &chunks[0].context {
            ChunkContext::GoDoc(ctx) => ctx,
            _ => panic!("Expected GoDoc context"),
        };
        assert_eq!(ctx.item_name.to_string().as_str(), "Config");
        assert_eq!(ctx.item_type, GoItemType::Struct);
    }

    #[test]
    fn test_skips_undocumented() {
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(r#"
package main

func NoDoc() {}
"#);
        let chunks = chunker.chunk(&input).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_extracts_package_name() {
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(r#"
package mypackage

// Foo does something.
func Foo() {}
"#);
        let chunks = chunker.chunk(&input).unwrap();
        assert!(!chunks.is_empty());
        let ctx = match &chunks[0].context {
            ChunkContext::GoDoc(ctx) => ctx,
            _ => panic!("Expected GoDoc context"),
        };
        assert_eq!(ctx.package_name.as_ref().map(|p| p.to_string()).as_deref(), Some("mypackage"));
    }
}

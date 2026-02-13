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

mod config;
mod extraction;
mod support;

pub use config::{GoConfig, GoFilter};
pub use support::GoDocSupport;

use std::path::Path;
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;
use tree_sitter::Parser;

use self::extraction::{
    determine_type_kind, extract_doc_comment, extract_identifier, get_visibility, node_kinds,
    to_filter_type,
};
use super::{ChunkingError, ChunkingInput, ChunkingStrategy};
use crate::knowledge::domain::EmbeddingModelConfig;
use crate::knowledge::domain::{
    Chunk, ChunkContent, ChunkContext, ChunkHash, DocLineCount, GoDocContext, GoItemType,
    GoVisibility, ItemName, PackageName, Signature, TokenCount,
};

const PACKAGE_ITEM_NAME: &str = "package";

/// Metadata for creating a chunk from a Go declaration.
struct ChunkMetadata {
    item_name: ItemName,
    visibility: GoVisibility,
    signature: Option<Signature>,
    item_type: GoItemType,
}

/// Extracts and chunks Go documentation comments.
pub struct GoDocChunker {
    splitter: TextSplitter<Tokenizer>,
    tokenizer: Tokenizer,
    filter: Option<GoFilter>,
}

fn get_function_or_method_type(kind: &str) -> GoItemType {
    if kind == node_kinds::METHOD_DECLARATION {
        GoItemType::Method
    } else {
        GoItemType::Function
    }
}

fn get_const_or_var_type(kind: &str) -> GoItemType {
    if kind == node_kinds::CONST_DECLARATION {
        GoItemType::Const
    } else {
        GoItemType::Var
    }
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

    fn should_index(&self, name: &str, item_type: GoItemType, doc: &str) -> bool {
        let Some(ref filter) = self.filter else {
            return true;
        };
        let (_, visibility) = get_visibility(name);
        let filter_type = to_filter_type(item_type);
        filter.should_index(
            &visibility,
            &filter_type,
            DocLineCount::new(doc.lines().count()),
        )
    }

    #[expect(clippy::too_many_arguments)]
    fn process_declaration(
        &self,
        node: tree_sitter::Node,
        source_bytes: &[u8],
        item_type: GoItemType,
        signature: Option<Signature>,
        package_name: &Option<PackageName>,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let Some(doc) = extract_doc_comment(node, source_bytes) else {
            return Ok(Vec::new());
        };
        let Some(name) = extract_identifier(node, source_bytes) else {
            return Ok(Vec::new());
        };
        if !self.should_index(&name, item_type, &doc) {
            return Ok(Vec::new());
        }
        let (go_vis, _) = get_visibility(&name);
        let metadata = ChunkMetadata {
            item_name: ItemName::try_new(&name)
                .map_err(crate::knowledge::error::box_err)
                .context(ParseSnafu {
                    file_path: input.source.file_path.to_string(),
                })?,
            visibility: go_vis,
            signature,
            item_type,
        };
        self.create_chunks(&doc, metadata, package_name, input)
    }

    fn process_package_clause(
        &self,
        node: tree_sitter::Node,
        source_bytes: &[u8],
        input: &ChunkingInput,
    ) -> Result<(Option<PackageName>, Vec<Chunk>), ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let pkg_name = node
            .child_by_field_name("name")
            .or_else(|| {
                node.children(&mut node.walk())
                    .find(|c| c.kind() == "package_identifier")
            })
            .and_then(|pkg_node| pkg_node.utf8_text(source_bytes).ok())
            .and_then(|s| PackageName::try_new(s).ok());

        let chunks = if let Some(doc) = extract_doc_comment(node, source_bytes) {
            let metadata = ChunkMetadata {
                item_name: ItemName::try_new(PACKAGE_ITEM_NAME)
                    .map_err(crate::knowledge::error::box_err)
                    .context(ParseSnafu {
                        file_path: input.source.file_path.to_string(),
                    })?,
                visibility: GoVisibility::Exported,
                signature: None,
                item_type: GoItemType::Package,
            };
            self.create_chunks(&doc, metadata, &pkg_name, input)?
        } else {
            Vec::new()
        };

        Ok((pkg_name, chunks))
    }

    fn create_chunks(
        &self,
        text: &str,
        metadata: ChunkMetadata,
        package_name: &Option<PackageName>,
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
                .context(ChunkContext::go_doc(
                    &GoDocContext::builder()
                        .item_name(metadata.item_name.clone())
                        .visibility(metadata.visibility)
                        .maybe_signature(metadata.signature.clone())
                        .item_type(metadata.item_type)
                        .maybe_package_name(package_name.as_ref().cloned())
                        .build(),
                ))
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
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu {
                file_path: file_path.clone(),
            })?;

        let tree = parser
            .parse(content, None)
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
                    let (pkg_name, pkg_chunks) =
                        self.process_package_clause(node, source_bytes, input)?;
                    package_name = pkg_name;
                    chunks.extend(pkg_chunks);
                }
                node_kinds::FUNCTION_DECLARATION | node_kinds::METHOD_DECLARATION => {
                    let item_type = get_function_or_method_type(node.kind());
                    let signature = node
                        .utf8_text(source_bytes)
                        .ok()
                        .and_then(|s| Signature::try_new(s).ok());
                    chunks.extend(self.process_declaration(
                        node,
                        source_bytes,
                        item_type,
                        signature,
                        &package_name,
                        input,
                    )?);
                }
                node_kinds::TYPE_DECLARATION => {
                    let item_type = determine_type_kind(node);
                    chunks.extend(self.process_declaration(
                        node,
                        source_bytes,
                        item_type,
                        None,
                        &package_name,
                        input,
                    )?);
                }
                node_kinds::CONST_DECLARATION | node_kinds::VAR_DECLARATION => {
                    let item_type = get_const_or_var_type(node.kind());
                    chunks.extend(self.process_declaration(
                        node,
                        source_bytes,
                        item_type,
                        None,
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
mod test {
    use super::*;
    use crate::knowledge::chunking::ChunkingStrategy;
    use crate::knowledge::domain::{
        ChunkSource, ChunkableContent, FileHash, GoDocContext, IndexRelativePath, RepoName,
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
        // Given a chunker configured for Go files
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();

        // When checking file extension support
        // Then .go files are supported and others are not
        assert!(chunker.supports(Path::new("main.go")));
        assert!(chunker.supports(Path::new("pkg/util.go")));
        assert!(!chunker.supports(Path::new("main.rs")));
        assert!(!chunker.supports(Path::new("main.go.bak")));
    }

    #[test]
    fn test_extracts_function_doc() {
        // Given Go source with a documented exported function
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package main

// Hello prints a greeting.
// It takes a name parameter.
func Hello(name string) {
    println("Hello, " + name)
}
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the function doc is extracted with correct metadata
        assert!(!chunks.is_empty());
        let ctx: GoDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be GoDocContext");
        assert_eq!(ctx.item_name.to_string().as_str(), "Hello");
        assert_eq!(ctx.visibility, GoVisibility::Exported);
        assert_eq!(ctx.item_type, GoItemType::Function);
    }

    #[test]
    fn test_extracts_unexported_function() {
        // Given Go source with a documented unexported function
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package main

// helper is an internal function.
func helper() {}
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the function is marked as unexported
        assert!(!chunks.is_empty());
        let ctx: GoDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be GoDocContext");
        assert_eq!(ctx.item_name.to_string().as_str(), "helper");
        assert_eq!(ctx.visibility, GoVisibility::Unexported);
    }

    #[test]
    fn test_extracts_struct_doc() {
        // Given Go source with a documented struct type
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package main

// Config holds configuration.
type Config struct {
    Name string
}
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the struct doc is extracted with correct type
        assert!(!chunks.is_empty());
        let ctx: GoDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be GoDocContext");
        assert_eq!(ctx.item_name.to_string().as_str(), "Config");
        assert_eq!(ctx.item_type, GoItemType::Struct);
    }

    #[test]
    fn test_skips_undocumented() {
        // Given Go source with an undocumented function
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package main

func NoDoc() {}
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then no chunks are produced
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_extracts_package_name() {
        // Given Go source with a named package
        let chunker = GoDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package mypackage

// Foo does something.
func Foo() {}
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the package name is captured in chunk context
        assert!(!chunks.is_empty());
        let ctx: GoDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be GoDocContext");
        assert_eq!(
            ctx.package_name.as_ref().map(|p| p.to_string()).as_deref(),
            Some("mypackage")
        );
    }
}

//! Extracts and chunks C documentation comments for semantic search.
//!
//! Implements chunking for C source files by extracting doc comments
//! (block comments `/* */`, kernel-style `/** */`, and consecutive `//` lines).
//! Uses `tree-sitter` for parsing and `text-splitter` for token-aware chunking.
//!
//! Doc comments are extracted from:
//! - Functions, structs, enums, typedefs
//! - Macros and preprocessor definitions
//! - Standalone comment blocks
//!
//! Each chunk preserves metadata including item name and signatures.

mod config;
mod context;
mod extraction;
mod support;

pub use config::{CConfig, CFilter};
pub use support::CDocSupport;

use std::path::Path;
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;
use tree_sitter::Parser;

use self::extraction::{
    extract_doc_comment, extract_identifier, find_standalone_comments, get_item_type,
};
use super::{ChunkingError, ChunkingInput, ChunkingStrategy};
use crate::knowledge::domain::EmbeddingModelConfig;
pub use context::{CDocContext, CItemType};

use crate::knowledge::domain::{
    Chunk, ChunkContent, ChunkContext, ChunkHash, DocLineCount, ItemName, Signature, TokenCount,
};

/// Metadata for creating a chunk from a C declaration.
struct ChunkMetadata {
    item_name: Option<ItemName>,
    signature: Option<Signature>,
    item_type: CItemType,
}

/// Extracts and chunks C documentation comments.
pub struct CDocChunker {
    splitter: TextSplitter<Tokenizer>,
    tokenizer: Tokenizer,
    filter: Option<CFilter>,
}

impl CDocChunker {
    /// Initializes chunker with tokenizer and splitter configured for the embedding model.
    pub fn from_config(config: &EmbeddingModelConfig) -> Result<Self, ChunkingError> {
        Self::from_config_with_filter(config, None)
    }

    /// Initializes chunker with tokenizer, splitter, and filtering rules for C items.
    pub fn from_config_with_filter(
        config: &EmbeddingModelConfig,
        filter: Option<CFilter>,
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

    fn should_index(&self, item_type: CItemType, doc: &str) -> bool {
        let Some(ref filter) = self.filter else {
            return true;
        };
        filter.should_index(&item_type, DocLineCount::new(doc.lines().count()))
    }

    fn process_declaration(
        &self,
        node: tree_sitter::Node,
        source_bytes: &[u8],
        item_type: CItemType,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        let Some(doc) = extract_doc_comment(node, source_bytes) else {
            return Ok(Vec::new());
        };
        if !self.should_index(item_type, &doc) {
            return Ok(Vec::new());
        }
        let name = extract_identifier(node, source_bytes);
        let item_name = name.as_ref().and_then(|n| ItemName::try_new(n).ok());
        let signature = node
            .utf8_text(source_bytes)
            .ok()
            .and_then(|s| s.lines().next())
            .and_then(|s| Signature::try_new(s.trim()).ok());
        let metadata = ChunkMetadata {
            item_name,
            signature,
            item_type,
        };
        self.create_chunks(&doc, metadata, input)
    }

    fn process_standalone_comment(
        &self,
        text: &str,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        if !self.should_index(CItemType::StandaloneComment, text) {
            return Ok(Vec::new());
        }
        let metadata = ChunkMetadata {
            item_name: None,
            signature: None,
            item_type: CItemType::StandaloneComment,
        };
        self.create_chunks(text, metadata, input)
    }

    fn process_node_children(
        &self,
        node: tree_sitter::Node,
        source_bytes: &[u8],
        input: &ChunkingInput,
        chunks: &mut Vec<Chunk>,
    ) -> Result<(), ChunkingError> {
        for child in node.children(&mut node.walk()) {
            // Recursively process children of preprocessor blocks
            if child.kind().starts_with("preproc_") {
                self.process_node_children(child, source_bytes, input, chunks)?;
                continue;
            }
            if let Some(item_type) = get_item_type(child.kind()) {
                chunks.extend(self.process_declaration(child, source_bytes, item_type, input)?);
            }
        }
        Ok(())
    }

    fn create_chunks(
        &self,
        text: &str,
        metadata: ChunkMetadata,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::ParseSnafu;
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
                        "c_doc",
                        &CDocContext::builder()
                            .maybe_item_name(metadata.item_name.clone())
                            .item_type(metadata.item_type)
                            .maybe_signature(metadata.signature.clone())
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

impl ChunkingStrategy for CDocChunker {
    fn supports(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "c" || ext == "h")
            .unwrap_or(false)
    }

    fn chunk(&self, input: &ChunkingInput) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let content = input.content.as_ref();
        let file_path = input.source.file_path.to_string();

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_c::LANGUAGE.into())
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu {
                file_path: file_path.clone(),
            })?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| ChunkingError::ParseError {
                file_path: file_path.clone(),
                source: "Failed to parse C source".into(),
            })?;

        let root = tree.root_node();
        let source_bytes = content.as_bytes();
        let mut chunks = Vec::new();

        // Process declarations (including those inside preprocessor blocks)
        self.process_node_children(root, source_bytes, input, &mut chunks)?;

        // Process standalone comments
        for comment in find_standalone_comments(root, source_bytes) {
            chunks.extend(self.process_standalone_comment(&comment.text, input)?);
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod test {
    use super::CDocContext;
    use super::*;
    use crate::knowledge::chunking::ChunkingStrategy;
    use crate::knowledge::domain::{
        ChunkSource, ChunkableContent, FileHash, IndexRelativePath, RepoName,
    };

    fn test_config() -> EmbeddingModelConfig {
        EmbeddingModelConfig::default()
    }

    fn make_input(content: &str, filename: &str) -> ChunkingInput {
        ChunkingInput {
            content: ChunkableContent::new(content.to_string()),
            source: ChunkSource::builder()
                .file_path(IndexRelativePath::try_new(filename).unwrap())
                .repo_name(RepoName::try_new("test").unwrap())
                .build(),
            file_hash: FileHash::new([0u8; 32]),
        }
    }

    #[test]
    fn test_supports_c_and_h_files() {
        // Given a chunker configured for C files
        let chunker = CDocChunker::from_config(&test_config()).unwrap();

        // When checking file extension support
        // Then .c and .h files are supported and others are not
        assert!(chunker.supports(Path::new("main.c")));
        assert!(chunker.supports(Path::new("header.h")));
        assert!(chunker.supports(Path::new("src/util.c")));
        assert!(!chunker.supports(Path::new("main.rs")));
        assert!(!chunker.supports(Path::new("main.cpp")));
        assert!(!chunker.supports(Path::new("main.c.bak")));
    }

    #[test]
    fn test_extracts_kernel_style_doc_comment() {
        // Given C source with a kernel-style doc comment preceding a function
        let chunker = CDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
/**
 * hello - prints a greeting
 * @name: the name to greet
 */
void hello(const char *name) {
    printf("Hello, %s\n", name);
}
"#,
            "test.c",
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the function doc is extracted with correct metadata
        assert!(!chunks.is_empty());
        let ctx: CDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be CDocContext");
        assert_eq!(
            ctx.item_name.as_ref().map(|n| n.to_string()).as_deref(),
            Some("hello")
        );
        assert_eq!(ctx.item_type, CItemType::Function);
    }

    #[test]
    fn test_extracts_plain_block_comment() {
        // Given C source with a plain block comment
        let chunker = CDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
/* Initialize the system */
void init(void) {}
"#,
            "test.c",
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the comment is extracted
        assert!(!chunks.is_empty());
        assert!(chunks[0].content.text.contains("Initialize the system"));
    }

    #[test]
    fn test_extracts_line_comments_preceding_struct() {
        // Given C source with consecutive // comments preceding a struct
        let chunker = CDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
// Configuration structure
// Holds all settings
struct config {
    int value;
};
"#,
            "test.h",
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the struct doc is extracted
        assert!(!chunks.is_empty());
        let ctx: CDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be CDocContext");
        assert_eq!(
            ctx.item_name.as_ref().map(|n| n.to_string()).as_deref(),
            Some("config")
        );
        assert_eq!(ctx.item_type, CItemType::Struct);
    }

    #[test]
    fn test_extracts_standalone_comment() {
        // Given C source with a standalone comment not attached to a declaration
        let chunker = CDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
/* This is a file-level comment explaining the module */

int some_var;
"#,
            "test.c",
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the standalone comment is extracted
        let standalone = chunks.iter().find(|c| {
            c.context
                .deserialize_as::<CDocContext>()
                .ok()
                .map(|ctx| ctx.item_type == CItemType::StandaloneComment)
                .unwrap_or(false)
        });
        assert!(standalone.is_some());
    }

    #[test]
    fn test_skips_files_with_no_comments() {
        // Given C source with no comments
        let chunker = CDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
void foo(void) {}
"#,
            "test.c",
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then no chunks are produced
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_respects_min_doc_lines_filter() {
        // Given a chunker with min_doc_lines filter
        let filter = CFilter::new(vec![CItemType::Function], DocLineCount::new(3));
        let chunker = CDocChunker::from_config_with_filter(&test_config(), Some(filter)).unwrap();
        let input = make_input(
            r#"
// Short comment
void short_doc(void) {}

// Line one
// Line two
// Line three
void long_doc(void) {}
"#,
            "test.c",
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then only the function with 3+ doc lines is included
        assert_eq!(chunks.len(), 1);
        let ctx: CDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be CDocContext");
        assert_eq!(
            ctx.item_name.as_ref().map(|n| n.to_string()).as_deref(),
            Some("long_doc")
        );
    }

    #[test]
    fn test_respects_item_type_filter() {
        // Given a chunker filtering for structs only
        let filter = CFilter::new(vec![CItemType::Struct], DocLineCount::new(0));
        let chunker = CDocChunker::from_config_with_filter(&test_config(), Some(filter)).unwrap();
        let input = make_input(
            r#"
// A function
void foo(void) {}

// A struct
struct bar {};
"#,
            "test.c",
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then only the struct is included
        assert_eq!(chunks.len(), 1);
        let ctx: CDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be CDocContext");
        assert_eq!(ctx.item_type, CItemType::Struct);
    }

    #[test]
    fn test_extracts_doc_after_preprocessor_directives() {
        // Given C header with include guards before struct
        let chunker = CDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
#ifndef ENGINE_H
#define ENGINE_H

/**
 * Engine telemetry data structure.
 *
 * Contains rpm, temperature, and oil pressure readings.
 */
struct EngineTelemetry {
    int rpm;
};

#endif
"#,
            "test.h",
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the struct doc is extracted despite preprocessor directives
        assert!(
            !chunks.is_empty(),
            "Should extract struct doc from header with include guards"
        );
        let ctx: CDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be CDocContext");
        assert_eq!(
            ctx.item_name.as_ref().map(|n| n.to_string()).as_deref(),
            Some("EngineTelemetry")
        );
    }
}

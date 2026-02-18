//! Extracts and chunks shell script comments for semantic search.
//!
//! Implements chunking for shell scripts by extracting comments
//! (# prefixed lines). Uses `tree-sitter` for parsing
//! and `text-splitter` for token-aware chunking with overlap.
//!
//! Comments are extracted from:
//! - Standalone comment blocks
//! - Comments preceding function definitions
//!
//! Each chunk preserves metadata including item name and signatures.

mod config;
mod context;
mod extraction;
mod support;

pub use config::{ShellConfig, ShellFilter};
pub use support::ShellDocSupport;

use std::path::Path;
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;
use tree_sitter::Parser;

use self::extraction::extract_comments;
use super::{ChunkingError, ChunkingInput, ChunkingStrategy};
use crate::knowledge::domain::EmbeddingModelConfig;
pub use context::{ShellDocContext, ShellItemType};

use crate::knowledge::domain::{
    Chunk, ChunkContent, ChunkContext, ChunkHash, DocLineCount, ItemName, Signature, TokenCount,
};

use self::config::ShellFilter as Filter;

/// Extracts and chunks shell script comments.
pub struct ShellDocChunker {
    splitter: TextSplitter<Tokenizer>,
    tokenizer: Tokenizer,
    filter: Option<Filter>,
}

impl ShellDocChunker {
    /// Initializes chunker with tokenizer and splitter configured for the embedding model.
    pub fn from_config(config: &EmbeddingModelConfig) -> Result<Self, ChunkingError> {
        Self::from_config_with_filter(config, None)
    }

    /// Initializes chunker with tokenizer, splitter, and filtering rules for shell items.
    pub fn from_config_with_filter(
        config: &EmbeddingModelConfig,
        filter: Option<Filter>,
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

    fn should_index(&self, item_type: &ShellItemType, doc: &str) -> bool {
        let Some(ref filter) = self.filter else {
            return true;
        };
        filter.should_index(item_type, DocLineCount::new(doc.lines().count()))
    }

    #[expect(clippy::too_many_arguments)]
    fn create_chunks(
        &self,
        text: &str,
        item_name: Option<ItemName>,
        item_type: ShellItemType,
        signature: Option<Signature>,
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
                        "shell",
                        &ShellDocContext::builder()
                            .maybe_item_name(item_name.clone())
                            .item_type(item_type)
                            .maybe_signature(signature.clone())
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

impl ChunkingStrategy for ShellDocChunker {
    fn supports(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "sh")
            .unwrap_or(false)
    }

    fn chunk(&self, input: &ChunkingInput) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let content = input.content.as_ref();
        let file_path = input.source.file_path.to_string();

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_bash::LANGUAGE.into())
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu {
                file_path: file_path.clone(),
            })?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| ChunkingError::ParseError {
                file_path: file_path.clone(),
                source: "Failed to parse shell source".into(),
            })?;

        let root = tree.root_node();
        let source_bytes = content.as_bytes();
        let mut chunks = Vec::new();

        for extracted in extract_comments(root, source_bytes) {
            if !self.should_index(&extracted.item_type, &extracted.text) {
                continue;
            }

            let item_name = extracted
                .function_name
                .as_ref()
                .and_then(|n| ItemName::try_new(n).ok());

            let signature = extracted
                .signature
                .as_ref()
                .and_then(|s| Signature::try_new(s).ok());

            chunks.extend(self.create_chunks(
                &extracted.text,
                item_name,
                extracted.item_type,
                signature,
                input,
            )?);
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::chunking::ChunkingStrategy;
    use crate::knowledge::domain::{
        ChunkSource, ChunkableContent, FileHash, IndexRelativePath, RepoName,
    };

    fn test_config() -> EmbeddingModelConfig {
        EmbeddingModelConfig::default()
    }

    fn make_input(content: &str) -> ChunkingInput {
        ChunkingInput {
            content: ChunkableContent::new(content.to_string()),
            source: ChunkSource::builder()
                .file_path(IndexRelativePath::try_new("test.sh").unwrap())
                .repo_name(RepoName::try_new("test").unwrap())
                .build(),
            file_hash: FileHash::new([0u8; 32]),
        }
    }

    #[test]
    fn test_supports_sh_files() {
        // Given a chunker configured for shell files
        let chunker = ShellDocChunker::from_config(&test_config()).unwrap();

        // When checking file extension support
        // Then .sh files are supported and others are not
        assert!(chunker.supports(Path::new("script.sh")));
        assert!(chunker.supports(Path::new("bin/deploy.sh")));
        assert!(!chunker.supports(Path::new("main.rs")));
        assert!(!chunker.supports(Path::new("script.bash")));
    }

    #[test]
    fn test_extracts_function_comment() {
        // Given shell source with a documented function
        let chunker = ShellDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"#!/bin/bash

# Prints a greeting message.
# Takes a name parameter.
greet() {
    echo "Hello, $1"
}
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the function comment is extracted with correct metadata
        assert!(!chunks.is_empty());
        let ctx: ShellDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be ShellDocContext");
        assert_eq!(
            ctx.item_name.as_ref().map(|n| n.to_string()).as_deref(),
            Some("greet")
        );
        assert_eq!(ctx.item_type, ShellItemType::Function);
    }

    #[test]
    fn test_extracts_standalone_comment() {
        // Given shell source with a standalone comment block
        let chunker = ShellDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"#!/bin/bash

# This is a standalone comment block.
# It describes the script purpose.

echo "Hello"
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then the standalone comment is extracted
        assert!(!chunks.is_empty());
        let ctx: ShellDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be ShellDocContext");
        assert!(ctx.item_name.is_none());
        assert_eq!(ctx.item_type, ShellItemType::StandaloneComment);
    }

    #[test]
    fn test_skips_files_with_no_comments() {
        // Given shell source with no comments
        let chunker = ShellDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"#!/bin/bash
echo "Hello"
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then no chunks are produced
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_respects_min_doc_lines_filter() {
        // Given a chunker with min_doc_lines filter
        let filter = ShellFilter::new(vec![ShellItemType::Function], DocLineCount::new(3));
        let chunker =
            ShellDocChunker::from_config_with_filter(&test_config(), Some(filter)).unwrap();
        let input = make_input(
            r#"#!/bin/bash

# Short comment.
short_func() {
    echo "short"
}

# This is a longer comment.
# It has multiple lines.
# Three lines total.
long_func() {
    echo "long"
}
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then only the function with enough doc lines is included
        assert_eq!(chunks.len(), 1);
        let ctx: ShellDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be ShellDocContext");
        assert_eq!(
            ctx.item_name.as_ref().map(|n| n.to_string()).as_deref(),
            Some("long_func")
        );
    }

    #[test]
    fn test_respects_item_type_filter() {
        // Given a chunker filtering for functions only
        let filter = ShellFilter::new(vec![ShellItemType::Function], DocLineCount::new(0));
        let chunker =
            ShellDocChunker::from_config_with_filter(&test_config(), Some(filter)).unwrap();
        let input = make_input(
            r#"#!/bin/bash

# Standalone comment.

# Function comment.
my_func() {
    echo "func"
}
"#,
        );

        // When chunking the source
        let chunks = chunker.chunk(&input).unwrap();

        // Then only function comments are included
        assert_eq!(chunks.len(), 1);
        let ctx: ShellDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be ShellDocContext");
        assert_eq!(ctx.item_type, ShellItemType::Function);
    }
}

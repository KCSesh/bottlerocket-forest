//! Extracts and chunks Kotlin documentation comments for semantic search.
//!
//! Implements chunking for Kotlin source files by extracting KDoc comments
//! (/** ... */ comments immediately preceding declarations). Uses `tree-sitter` for parsing
//! and `text-splitter` for token-aware chunking with overlap.
//!
//! KDoc comments are extracted from classes, objects, interfaces, functions,
//! properties, and enum classes.
//!
//! Each chunk preserves metadata including item name, visibility, and signatures.

mod config;
mod context;
mod extraction;

use std::any::Any;
use text_splitter::{ChunkConfig, TextSplitter};
use tokenizers::Tokenizer;
use tree_sitter::Parser;

use self::extraction::{extract_doc_comment, extract_identifier, get_visibility, node_kinds};
use super::language::{LanguageConfig, LanguageSupport};
use super::{ChunkingError, ChunkingInput, ChunkingStrategy};
use crate::knowledge::domain::EmbeddingModelConfig;
use crate::knowledge::domain::FilePeek;
pub use context::{KotlinDocContext, KotlinItemType, KotlinVisibility};

use crate::knowledge::domain::{
    Chunk, ChunkContent, ChunkContext, ChunkHash, DocLineCount, ItemName, PackageName, Signature,
    TokenCount,
};

use crate::knowledge::storage::StorageError;

pub use config::{KotlinFilter, KotlinFilterConfig, KotlinFilterItemType};

/// Metadata for creating a chunk from a Kotlin declaration.
struct ChunkMetadata {
    item_name: ItemName,
    visibility: KotlinVisibility,
    signature: Option<Signature>,
    item_type: KotlinItemType,
}

/// Context for processing Kotlin nodes during chunking.
struct ProcessingContext<'a> {
    source_bytes: &'a [u8],
    package_name: &'a Option<PackageName>,
    input: &'a ChunkingInput,
}

/// Extracts and chunks Kotlin documentation comments.
pub struct KotlinDocChunker {
    splitter: TextSplitter<Tokenizer>,
    tokenizer: Tokenizer,
    filter: Option<KotlinFilter>,
}

impl KotlinDocChunker {
    /// Initializes chunker with tokenizer and splitter configured for the embedding model.
    pub fn from_config(config: &EmbeddingModelConfig) -> Result<Self, ChunkingError> {
        Self::from_config_with_filter(config, None)
    }

    /// Initializes chunker with tokenizer, splitter, and filtering rules for Kotlin items.
    pub fn from_config_with_filter(
        config: &EmbeddingModelConfig,
        filter: Option<KotlinFilter>,
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

    fn should_index(
        &self,
        visibility: &crate::knowledge::domain::Visibility,
        item_type: KotlinItemType,
        doc: &str,
    ) -> bool {
        let Some(ref filter) = self.filter else {
            return true;
        };
        filter.should_index(
            visibility,
            &item_type,
            DocLineCount::new(doc.lines().count()),
        )
    }

    fn process_declaration(
        &self,
        node: tree_sitter::Node,
        item_type: KotlinItemType,
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
        let (kotlin_vis, vis) = get_visibility(node, ctx.source_bytes);
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
            visibility: kotlin_vis,
            signature,
            item_type,
        };
        self.create_chunks(&doc, metadata, ctx.package_name, ctx.input)
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
                .context(
                    ChunkContext::new(
                        "kotlin_doc",
                        &KotlinDocContext::builder()
                            .item_name(metadata.item_name.clone())
                            .visibility(metadata.visibility)
                            .maybe_signature(metadata.signature.clone())
                            .item_type(metadata.item_type)
                            .maybe_package_name(package_name.as_ref().cloned())
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

fn get_item_type(kind: &str) -> Option<KotlinItemType> {
    match kind {
        node_kinds::CLASS_DECLARATION => Some(KotlinItemType::Class),
        node_kinds::OBJECT_DECLARATION => Some(KotlinItemType::Object),
        node_kinds::FUNCTION_DECLARATION => Some(KotlinItemType::Function),
        node_kinds::PROPERTY_DECLARATION => Some(KotlinItemType::Property),
        _ => None,
    }
}

impl ChunkingStrategy for KotlinDocChunker {
    fn supports(&self, peek: &FilePeek) -> bool {
        matches!(peek.extension(), Some("kt") | Some("kts"))
    }

    fn chunk(&self, input: &ChunkingInput) -> Result<Vec<Chunk>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let content = input.content.as_ref();
        let file_path = input.source.file_path.to_string();

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu {
                file_path: file_path.clone(),
            })?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| ChunkingError::ParseError {
                file_path: file_path.clone(),
                source: "Failed to parse Kotlin source".into(),
            })?;

        let root = tree.root_node();
        let source_bytes = content.as_bytes();
        let mut chunks = Vec::new();
        let package_name = extract_package_name(root, source_bytes);
        let ctx = ProcessingContext {
            source_bytes,
            package_name: &package_name,
            input,
        };

        process_node(self, root, &ctx, &mut chunks)?;

        Ok(chunks)
    }
}

/// Language support registration for Kotlin.
pub struct KotlinDocSupport;

inventory::submit!(&KotlinDocSupport as &dyn LanguageSupport);

impl LanguageSupport for KotlinDocSupport {
    fn context_type_name(&self) -> &'static str {
        "kotlin_doc"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["kt", "kts"]
    }

    fn create_chunker(
        &self,
        embedding_config: &EmbeddingModelConfig,
        language_config: Option<&LanguageConfig>,
    ) -> Result<Box<dyn ChunkingStrategy>, ChunkingError> {
        use super::strategy::chunking_error::*;
        use snafu::ResultExt;

        let filter = language_config
            .map(|cfg| cfg.deserialize_as::<KotlinFilterConfig>())
            .transpose()
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu {
                file_path: "kotlin config".to_string(),
            })?
            .map(|cfg| cfg.to_filter());
        Ok(Box::new(KotlinDocChunker::from_config_with_filter(
            embedding_config,
            filter,
        )?))
    }

    fn serialize_context(&self, context: &dyn Any) -> Result<String, StorageError> {
        use crate::knowledge::storage::repository::storage_error::*;
        use serde::ser::Error as _;
        use snafu::ResultExt;

        let ctx = context
            .downcast_ref::<KotlinDocContext>()
            .ok_or_else(|| serde_json::Error::custom("Expected KotlinDocContext"))
            .context(SerializationSnafu)?;
        serde_json::to_string(ctx).context(SerializationSnafu)
    }

    fn deserialize_context(&self, json: &str) -> Result<Box<dyn Any + Send + Sync>, StorageError> {
        use crate::knowledge::storage::repository::storage_error::*;
        use snafu::ResultExt;

        let ctx: KotlinDocContext = serde_json::from_str(json).context(SerializationSnafu)?;
        Ok(Box::new(ctx))
    }

    fn default_config(&self) -> Option<LanguageConfig> {
        LanguageConfig::new(&KotlinFilterConfig::default()).ok()
    }

    fn config_key(&self) -> &'static str {
        "kotlin"
    }

    fn enabled_by_default(&self) -> bool {
        false
    }
}

fn extract_package_name(root: tree_sitter::Node, source: &[u8]) -> Option<PackageName> {
    root.children(&mut root.walk())
        .find(|c| c.kind() == "package_header")
        .and_then(|pkg| {
            pkg.children(&mut pkg.walk())
                .find(|c| c.kind() == "qualified_identifier")
        })
        .and_then(|n| n.utf8_text(source).ok())
        .and_then(|s| PackageName::try_new(s).ok())
}

fn process_node(
    chunker: &KotlinDocChunker,
    node: tree_sitter::Node,
    ctx: &ProcessingContext,
    chunks: &mut Vec<Chunk>,
) -> Result<(), ChunkingError> {
    if let Some(item_type) = get_item_type(node.kind()) {
        chunks.extend(chunker.process_declaration(node, item_type, ctx)?);
    }
    for child in node.children(&mut node.walk()) {
        process_node(chunker, child, ctx, chunks)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::KotlinDocContext;
    use super::*;
    use crate::knowledge::chunking::ChunkingStrategy;
    use crate::knowledge::domain::{
        ChunkSource, ChunkableContent, FileHash, FilePeek, IndexRelativePath, RepoName,
    };

    fn test_config() -> EmbeddingModelConfig {
        EmbeddingModelConfig::default()
    }

    fn make_input(content: &str) -> ChunkingInput {
        ChunkingInput {
            content: ChunkableContent::new(content.to_string()),
            source: ChunkSource::builder()
                .file_path(IndexRelativePath::try_new("Test.kt").unwrap())
                .repo_name(RepoName::try_new("test").unwrap())
                .build(),
            file_hash: FileHash::new([0u8; 32]),
        }
    }

    #[test]
    fn test_supports_kotlin_files() {
        // Given a Kotlin chunker
        let chunker = KotlinDocChunker::from_config(&test_config()).unwrap();

        // When checking file support
        // Then .kt and .kts files should be supported
        assert!(chunker.supports(&FilePeek::from_path_string("Main.kt")));
        assert!(chunker.supports(&FilePeek::from_path_string("build.gradle.kts")));
        assert!(chunker.supports(&FilePeek::from_path_string("pkg/Util.kt")));

        // And other files should not be supported
        assert!(!chunker.supports(&FilePeek::from_path_string("main.rs")));
        assert!(!chunker.supports(&FilePeek::from_path_string("Main.java")));
    }

    #[test]
    fn test_extracts_class_doc() {
        // Given a Kotlin file with a documented class
        let chunker = KotlinDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package com.example

/**
 * A sample class.
 * With multiple lines.
 */
class Sample {
}
"#,
        );

        // When chunking the file
        let chunks = chunker.chunk(&input).unwrap();

        // Then the class doc should be extracted
        assert!(!chunks.is_empty());
        let ctx: KotlinDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be KotlinDocContext");
        assert_eq!(ctx.item_name.to_string().as_str(), "Sample");
        assert_eq!(ctx.visibility, KotlinVisibility::Public);
        assert_eq!(ctx.item_type, KotlinItemType::Class);
    }

    #[test]
    fn test_extracts_function_doc() {
        // Given a Kotlin file with a documented function
        let chunker = KotlinDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package com.example

class Sample {
    /**
     * Does something useful.
     */
    fun doSomething() {}
}
"#,
        );

        // When chunking the file
        let chunks = chunker.chunk(&input).unwrap();

        // Then the function doc should be extracted
        let func_chunk = chunks.iter().find(|c| {
            c.context
                .deserialize_as::<KotlinDocContext>()
                .ok()
                .map(|ctx| ctx.item_type == KotlinItemType::Function)
                .unwrap_or(false)
        });
        assert!(func_chunk.is_some());
        let ctx: KotlinDocContext = func_chunk
            .unwrap()
            .context
            .deserialize_as()
            .expect("should be KotlinDocContext");
        assert_eq!(ctx.item_name.to_string().as_str(), "doSomething");
    }

    #[test]
    fn test_skips_undocumented() {
        // Given a Kotlin file without documentation
        let chunker = KotlinDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package com.example

class NoDoc {
}
"#,
        );

        // When chunking the file
        let chunks = chunker.chunk(&input).unwrap();

        // Then no chunks should be produced
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_extracts_package_name() {
        // Given a Kotlin file with a package declaration
        let chunker = KotlinDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package com.example.mypackage

/**
 * A class.
 */
class Foo {}
"#,
        );

        // When chunking the file
        let chunks = chunker.chunk(&input).unwrap();

        // Then the package name should be extracted
        assert!(!chunks.is_empty());
        let ctx: KotlinDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be KotlinDocContext");
        assert_eq!(
            ctx.package_name.as_ref().map(|p| p.to_string()).as_deref(),
            Some("com.example.mypackage")
        );
    }

    #[test]
    fn test_private_visibility() {
        // Given a Kotlin file with a private class
        let chunker = KotlinDocChunker::from_config(&test_config()).unwrap();
        let input = make_input(
            r#"
package com.example

/**
 * Private class.
 */
private class Internal {
}
"#,
        );

        // When chunking the file
        let chunks = chunker.chunk(&input).unwrap();

        // Then the visibility should be private
        assert!(!chunks.is_empty());
        let ctx: KotlinDocContext = chunks[0]
            .context
            .deserialize_as()
            .expect("should be KotlinDocContext");
        assert_eq!(ctx.visibility, KotlinVisibility::Private);
    }
}

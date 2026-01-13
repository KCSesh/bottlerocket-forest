//! Extracts doc comments from Rust syntax tree items and creates searchable chunks.

use snafu::ResultExt;
use syn::{Attribute, Item, ItemImpl};
use text_splitter::TextSplitter;
use tokenizers::Tokenizer;

use crate::knowledge::chunking::{ChunkingError, ChunkingInput};
use crate::knowledge::domain::{
    Chunk, ChunkContent, ChunkContext, ChunkHash, ChunkId, DocLineCount, ItemName, RustDocContext,
    Signature, TokenCount, Visibility,
};
use crate::knowledge::indexing::RustItemType;

pub(crate) struct DocExtractor<'a> {
    pub(super) splitter: &'a TextSplitter<Tokenizer>,
    pub(super) tokenizer: &'a Tokenizer,
    pub(super) filter: Option<&'a crate::knowledge::indexing::RustFilter>,
}

impl<'a> DocExtractor<'a> {
    /// Extracts doc comment text from attributes.
    ///
    /// Collects `///` and `//!` comments, ignoring inline `//` comments.
    pub(super) fn extract_doc_text(attrs: &[Attribute]) -> String {
        attrs
            .iter()
            .filter_map(|attr| {
                if !attr.path().is_ident("doc") {
                    return None;
                }
                let syn::Meta::NameValue(meta) = &attr.meta else {
                    return None;
                };
                let syn::Expr::Lit(expr_lit) = &meta.value else {
                    return None;
                };
                let syn::Lit::Str(lit_str) = &expr_lit.lit else {
                    return None;
                };
                Some(lit_str.value())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Helper to process items with common pattern: extract doc, create name, create chunks.
    #[expect(clippy::too_many_arguments)]
    fn process_item(
        &self,
        attrs: &[Attribute],
        ident: &syn::Ident,
        visibility: syn::Visibility,
        signature: Option<Signature>,
        item_type: crate::knowledge::indexing::RustItemType,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use crate::knowledge::chunking::strategy::chunking_error::*;

        let doc_text = Self::extract_doc_text(attrs);
        if doc_text.trim().is_empty() {
            return Ok(vec![]);
        }

        let file_path = input.source.file_path.to_string();
        let item_name = ItemName::try_new(ident.to_string())
            .map_err(crate::knowledge::error::box_err)
            .context(ParseSnafu {
                file_path: file_path.clone(),
            })?;

        self.create_chunks(
            &doc_text,
            item_name,
            Visibility::from(visibility),
            signature,
            item_type,
            input,
        )
    }

    /// Extracts doc comments from a syntax tree item and creates chunks.
    ///
    /// Processes functions, structs, enums, traits, modules, and impl blocks.
    /// Recursively processes nested items in modules.
    #[expect(clippy::excessive_nesting)]
    pub(super) fn extract_item_doc_chunks(
        &self,
        item: &Item,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use crate::knowledge::chunking::strategy::chunking_error::*;

        let mut chunks = Vec::new();

        match item {
            Item::Fn(item_fn) => {
                let sig = &item_fn.sig;
                let signature = Signature::try_new(quote::quote!(#sig).to_string())
                    .map_err(crate::knowledge::error::box_err)
                    .context(ParseSnafu {
                        file_path: input.source.file_path.to_string(),
                    })?;
                chunks.extend(self.process_item(
                    &item_fn.attrs,
                    &item_fn.sig.ident,
                    item_fn.vis.clone(),
                    Some(signature),
                    RustItemType::Function,
                    input,
                )?);
            }
            Item::Struct(i) => chunks.extend(self.process_simple_item(
                &i.attrs,
                &i.ident,
                i.vis.clone(),
                RustItemType::Struct,
                input,
            )?),
            Item::Enum(i) => chunks.extend(self.process_simple_item(
                &i.attrs,
                &i.ident,
                i.vis.clone(),
                RustItemType::Enum,
                input,
            )?),
            Item::Trait(i) => chunks.extend(self.process_simple_item(
                &i.attrs,
                &i.ident,
                i.vis.clone(),
                RustItemType::Trait,
                input,
            )?),
            Item::Type(i) => chunks.extend(self.process_simple_item(
                &i.attrs,
                &i.ident,
                i.vis.clone(),
                RustItemType::TypeAlias,
                input,
            )?),
            Item::Const(i) => chunks.extend(self.process_simple_item(
                &i.attrs,
                &i.ident,
                i.vis.clone(),
                RustItemType::Constant,
                input,
            )?),
            Item::Mod(item_mod) => {
                chunks.extend(self.process_simple_item(
                    &item_mod.attrs,
                    &item_mod.ident,
                    item_mod.vis.clone(),
                    RustItemType::Module,
                    input,
                )?);
                if let Some((_, items)) = &item_mod.content {
                    for item in items {
                        chunks.extend(self.extract_item_doc_chunks(item, input)?);
                    }
                }
            }
            Item::Impl(item_impl) => {
                chunks.extend(self.extract_impl_method_chunks(item_impl, input)?);
            }
            _ => {}
        }

        Ok(chunks)
    }

    /// Helper for items without signatures (struct, enum, trait, type, const, module).
    #[expect(clippy::too_many_arguments)]
    fn process_simple_item(
        &self,
        attrs: &[Attribute],
        ident: &syn::Ident,
        visibility: syn::Visibility,
        item_type: crate::knowledge::indexing::RustItemType,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        self.process_item(attrs, ident, visibility, None, item_type, input)
    }

    /// Extracts doc comments from methods in impl blocks and creates chunks.
    fn extract_impl_method_chunks(
        &self,
        item_impl: &ItemImpl,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use crate::knowledge::chunking::strategy::chunking_error::*;

        let file_path = input.source.file_path.to_string();
        let mut chunks = Vec::new();

        for impl_item in &item_impl.items {
            let syn::ImplItem::Fn(method) = impl_item else {
                continue;
            };

            let doc_text = Self::extract_doc_text(&method.attrs);
            if doc_text.trim().is_empty() {
                continue;
            }

            let item_name = ItemName::try_new(method.sig.ident.to_string())
                .map_err(crate::knowledge::error::box_err)
                .context(ParseSnafu {
                    file_path: file_path.clone(),
                })?;

            let sig = &method.sig;
            let signature = Signature::try_new(quote::quote!(#sig).to_string())
                .map_err(crate::knowledge::error::box_err)
                .context(ParseSnafu {
                    file_path: file_path.clone(),
                })?;

            chunks.extend(self.create_chunks(
                &doc_text,
                item_name,
                Visibility::from(method.vis.clone()),
                Some(signature),
                RustItemType::Impl,
                input,
            )?);
        }

        Ok(chunks)
    }

    /// Splits doc text into chunks with token-based overlap.
    ///
    /// Each chunk preserves the same metadata (item name, visibility, signature).
    #[expect(clippy::too_many_arguments)]
    pub(super) fn create_chunks(
        &self,
        doc_text: &str,
        item_name: ItemName,
        visibility: Visibility,
        signature: Option<Signature>,
        item_type: crate::knowledge::indexing::RustItemType,
        input: &ChunkingInput,
    ) -> Result<Vec<Chunk>, ChunkingError> {
        use crate::knowledge::chunking::strategy::chunking_error::*;

        if let Some(filter) = &self.filter
            && !filter.should_index(
                &visibility,
                &item_type,
                DocLineCount::new(doc_text.lines().count()),
            )
        {
            return Ok(vec![]);
        }

        let file_path = input.source.file_path.to_string();
        let text_chunks: Vec<&str> = self.splitter.chunks(doc_text).collect();

        text_chunks
            .into_iter()
            .map(|text| {
                let encoding = self.tokenizer.encode(text, false).context(ParseSnafu {
                    file_path: file_path.clone(),
                })?;

                let token_count = encoding.len().max(1);

                let token_count = TokenCount::try_new(token_count)
                    .map_err(crate::knowledge::error::box_err)
                    .context(ParseSnafu {
                        file_path: file_path.clone(),
                    })?;

                let context = RustDocContext::builder()
                    .item_name(item_name.clone())
                    .visibility(visibility)
                    .maybe_signature(signature.clone())
                    .item_type(item_type)
                    .build();

                let chunk_hash = ChunkHash::from_text(text);
                Ok(Chunk::builder()
                    .id(ChunkId::new(uuid::Uuid::new_v4()))
                    .chunk_hash(chunk_hash)
                    .file_hash(input.file_hash)
                    .source(input.source.clone())
                    .content(
                        ChunkContent::builder()
                            .text(text)
                            .token_count(token_count)
                            .build(),
                    )
                    .context(ChunkContext::RustDoc(context))
                    .build())
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::chunking::{ChunkingStrategy, RustDocChunker};
    use crate::knowledge::domain::EmbeddingModelConfig;
    use crate::knowledge::domain::{
        ChunkContext, ChunkSource, ChunkableContent, DocLineCount, FileHash, IndexRelativePath,
        ItemName, RepoName, Visibility,
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

    fn chunk_test_content(content: &str) -> Vec<Chunk> {
        let input = create_test_input(content);
        let config = test_config();
        let chunker = RustDocChunker::from_config(&config).unwrap();
        chunker.chunk(&input).unwrap()
    }

    fn chunker_with_filter(
        vis: Vec<Visibility>,
        types: Vec<crate::knowledge::indexing::RustItemType>,
        min_lines: usize,
    ) -> RustDocChunker {
        use crate::knowledge::indexing::RustFilter;
        let config = test_config();
        let filter = RustFilter::new(vis, types, DocLineCount::new(min_lines));
        RustDocChunker::from_config_with_filter(&config, Some(filter)).unwrap()
    }

    fn assert_rustdoc_context(chunk: &Chunk, expected_name: &str, expected_vis: Visibility) {
        if let ChunkContext::RustDoc(ctx) = &chunk.context {
            assert_eq!(ctx.item_name, ItemName::try_new(expected_name).unwrap());
            assert_eq!(ctx.visibility, expected_vis);
        } else {
            panic!("Expected RustDoc context");
        }
    }

    #[test_case(r#"
/// Validates input data
pub fn validate(input: &str) -> bool {
    true
}
"#, "validate" ; "fn_item")]
    #[test_case(r#"
/// User profile data
pub struct User {
    name: String,
}
"#, "User" ; "struct_item")]
    #[test_case(r#"
/// Connection state
pub enum State {
    Connected,
    Disconnected,
}
"#, "State" ; "enum_item")]
    #[test_case(r#"
/// Repository for data persistence
pub trait Repository {
    fn save(&self);
}
"#, "Repository" ; "trait_item")]
    fn test_captures_item_metadata(content: &str, expected_name: &str) {
        // Given: Rust code with documented item
        // When: Chunking the content
        let chunks = chunk_test_content(content);
        // Then: Chunk captures item name and visibility
        assert!(!chunks.is_empty());
        assert_rustdoc_context(&chunks[0], expected_name, Visibility::Public);
    }

    #[test_case("pub", Visibility::Public ; "public_vis")]
    #[test_case("pub(crate)", Visibility::Crate ; "crate_vis")]
    #[test_case("", Visibility::Private ; "private_vis")]
    fn test_captures_visibility(vis_modifier: &str, expected_visibility: Visibility) {
        // Given: Function with specific visibility
        // When: Chunking the content
        let content = format!(
            "/// Documented function\n{} fn test_fn() {{}}",
            vis_modifier
        );
        let chunks = chunk_test_content(&content);
        // Then: Chunk captures correct visibility
        assert!(!chunks.is_empty());
        if let ChunkContext::RustDoc(ctx) = &chunks[0].context {
            assert_eq!(ctx.visibility, expected_visibility);
        } else {
            panic!("Expected RustDoc context");
        }
    }

    #[test]
    fn test_captures_function_signature() {
        // Given: Function with complex signature
        // When: Chunking the content
        let content = r#"
/// Processes data
pub fn process(input: &str, count: usize) -> Result<String, std::io::Error> {
    Ok(input.to_string())
}
"#;
        let chunks = chunk_test_content(content);
        // Then: Chunk captures signature with parameters
        assert!(!chunks.is_empty());
        if let ChunkContext::RustDoc(ctx) = &chunks[0].context {
            assert!(ctx.signature.is_some());
            let sig = ctx.signature.as_ref().unwrap();
            assert!(sig.to_string().contains("process"));
            assert!(sig.to_string().contains("input"));
            assert!(sig.to_string().contains("count"));
        } else {
            panic!("Expected RustDoc context");
        }
    }

    #[test]
    fn test_respects_token_limit() {
        // Given: Documentation exceeding token limit
        // When: Chunking the content
        let long_doc = format!("/// {}\n", "word ".repeat(300));
        let content = format!("{}pub fn test() {{}}", long_doc);
        let chunks = chunk_test_content(&content);
        // Then: All chunks respect token limit
        for chunk in &chunks {
            assert!(
                chunk.content.token_count.into_inner() <= 256,
                "Chunk exceeded token limit: {}",
                chunk.content.token_count.into_inner()
            );
        }
    }

    #[test]
    fn test_chunks_have_overlap() {
        // Given: Documentation requiring multiple chunks
        // When: Chunking the content
        let long_doc = format!("/// {}\n", "This is sentence number X. ".repeat(200));
        let content = format!("{}pub fn test() {{}}", long_doc);
        let chunks = chunk_test_content(&content);
        // Then: Adjacent chunks have overlapping content
        if chunks.len() > 1 {
            let first_chunk_end = &chunks[0].content.text[chunks[0].content.text.len() - 50..];
            let second_chunk_start =
                &chunks[1].content.text[..50.min(chunks[1].content.text.len())];
            assert!(
                first_chunk_end
                    .split_whitespace()
                    .any(|word| second_chunk_start.contains(word)),
                "Expected overlap between chunks"
            );
        }
    }

    #[test]
    fn test_preserves_metadata_across_chunks() {
        // Given: Documentation requiring multiple chunks
        // When: Chunking the content
        let long_doc = format!("/// {}\n", "word ".repeat(300));
        let content = format!("{}pub fn long_function() {{}}", long_doc);
        let chunks = chunk_test_content(&content);
        // Then: All chunks preserve same metadata
        if chunks.len() > 1 {
            let first_ctx = if let ChunkContext::RustDoc(ctx) = &chunks[0].context {
                ctx
            } else {
                panic!("Expected RustDoc context");
            };
            for chunk in &chunks[1..] {
                if let ChunkContext::RustDoc(ctx) = &chunk.context {
                    assert_eq!(ctx.item_name, first_ctx.item_name);
                    assert_eq!(ctx.visibility, first_ctx.visibility);
                } else {
                    panic!("Expected RustDoc context");
                }
            }
        }
    }

    #[test_case(
        vec![Visibility::Public], vec![crate::knowledge::indexing::RustItemType::Function], 0,
        r#"
/// Public function
pub fn public_fn() {}

/// Private function
fn private_fn() {}
"#, 1, "Public function" ; "visibility_filter")]
    #[test_case(
        vec![Visibility::Public], vec![crate::knowledge::indexing::RustItemType::Struct], 0,
        r#"
/// A struct
pub struct MyStruct {}

/// A function
pub fn my_function() {}
"#, 1, "A struct" ; "item_type_filter")]
    #[test_case(
        vec![Visibility::Public], vec![crate::knowledge::indexing::RustItemType::Function], 3,
        r#"
/// Short doc
pub fn short() {}

/// This is a longer documentation comment
/// that spans multiple lines
/// and exceeds the minimum line count
pub fn long() {}
"#, 1, "longer documentation" ; "min_lines_filter")]
    #[test_case(
        vec![Visibility::Public],
        vec![crate::knowledge::indexing::RustItemType::Struct, crate::knowledge::indexing::RustItemType::Enum], 2,
        r#"
/// A public struct with
/// sufficient documentation
pub struct PublicStruct {}

/// Short
pub struct ShortDoc {}

/// A private struct with
/// sufficient documentation
struct PrivateStruct {}

/// A public enum with
/// sufficient documentation
pub enum PublicEnum { A, B }

/// A public function with
/// sufficient documentation
pub fn public_function() {}
"#, 2, "public struct" ; "multiple_criteria_filter")]
    fn test_chunker_filter(
        vis: Vec<Visibility>,
        types: Vec<crate::knowledge::indexing::RustItemType>,
        min_lines: usize,
        content: &str,
        expected_count: usize,
        expected_text: &str,
    ) {
        // Given: Chunker with specific filter configuration
        let chunker = chunker_with_filter(vis, types, min_lines);
        let input = create_test_input(content);

        // When: Chunking content with mixed items
        let chunks = chunker.chunk(&input).unwrap();

        // Then: Only matching items are chunked
        assert_eq!(chunks.len(), expected_count);
        assert!(
            chunks
                .iter()
                .any(|c| c.content.text.contains(expected_text))
        );
    }
}

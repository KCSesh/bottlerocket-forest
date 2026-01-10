//! Tests for chunker filtering functionality.

use crate::knowledge::chunking::ChunkingInput;
use crate::knowledge::chunking::ChunkingStrategy;
use crate::knowledge::chunking::rustdoc::RustDocChunker;
use crate::knowledge::domain::{
    ChunkSource, ChunkableContent, EmbeddingModelConfig, FileHash, IndexRelativePath, RepoName,
    Visibility,
};
use crate::knowledge::indexing::{RustFilter, RustItemType};
use test_case::test_case;

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

fn chunker_with_filter(
    vis: Vec<Visibility>,
    types: Vec<RustItemType>,
    min_lines: usize,
) -> RustDocChunker {
    let config = EmbeddingModelConfig::default();
    let filter = RustFilter::new(vis, types, min_lines);
    RustDocChunker::from_config_with_filter(&config, Some(filter)).unwrap()
}

#[test_case(
    vec![Visibility::Public], vec![RustItemType::Function], 0,
    r#"
/// Public function
pub fn public_fn() {}

/// Private function
fn private_fn() {}
"#, 1, "Public function" ; "visibility_filter")]
#[test_case(
    vec![Visibility::Public], vec![RustItemType::Struct], 0,
    r#"
/// A struct
pub struct MyStruct {}

/// A function
pub fn my_function() {}
"#, 1, "A struct" ; "item_type_filter")]
#[test_case(
    vec![Visibility::Public], vec![RustItemType::Function], 3,
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
    vec![RustItemType::Struct, RustItemType::Enum], 2,
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
    types: Vec<RustItemType>,
    min_lines: usize,
    content: &str,
    expected_count: usize,
    expected_text: &str,
) {
    let chunker = chunker_with_filter(vis, types, min_lines);
    let input = create_test_input(content);
    let chunks = chunker.chunk(&input).unwrap();
    assert_eq!(chunks.len(), expected_count);
    assert!(
        chunks
            .iter()
            .any(|c| c.content.text.contains(expected_text))
    );
}

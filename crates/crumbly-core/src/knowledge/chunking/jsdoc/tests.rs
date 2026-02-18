//! Tests for JavaScript/TypeScript documentation chunking.

use std::path::Path;

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
fn test_supports_js_files() {
    let chunker = JsDocChunker::for_javascript(&test_config()).unwrap();
    assert!(chunker.supports(Path::new("main.js")));
    assert!(chunker.supports(Path::new("component.jsx")));
    assert!(!chunker.supports(Path::new("main.ts")));
    assert!(!chunker.supports(Path::new("main.rs")));
}

#[test]
fn test_supports_ts_files() {
    let chunker = JsDocChunker::for_typescript(&test_config()).unwrap();
    assert!(chunker.supports(Path::new("main.ts")));
    assert!(chunker.supports(Path::new("component.tsx")));
    assert!(!chunker.supports(Path::new("main.js")));
    assert!(!chunker.supports(Path::new("main.rs")));
}

#[test]
fn test_extracts_jsdoc_function() {
    let chunker = JsDocChunker::for_javascript(&test_config()).unwrap();
    let input = make_input(
        r#"
/**
 * Greets a user.
 * @param name The user's name
 */
export function greet(name) {
    console.log("Hello, " + name);
}
"#,
        "test.js",
    );
    let chunks = chunker.chunk(&input).unwrap();
    assert!(!chunks.is_empty());
    let ctx: JsDocContext = chunks
        .iter()
        .find(|c| {
            c.context
                .deserialize_as::<JsDocContext>()
                .ok()
                .map(|ctx| ctx.item_type == JsItemType::Function)
                .unwrap_or(false)
        })
        .unwrap()
        .context
        .deserialize_as()
        .unwrap();
    assert_eq!(ctx.item_name.to_string().as_str(), "greet");
    assert_eq!(ctx.visibility, JsVisibility::Exported);
}

#[test]
fn test_extracts_line_comment_class() {
    let chunker = JsDocChunker::for_javascript(&test_config()).unwrap();
    let input = make_input(
        r#"
// A simple counter class.
// Tracks a numeric value.
export class Counter {
    constructor() {
        this.value = 0;
    }
}
"#,
        "test.js",
    );
    let chunks = chunker.chunk(&input).unwrap();
    let class_chunk = chunks.iter().find(|c| {
        c.context
            .deserialize_as::<JsDocContext>()
            .ok()
            .map(|ctx| ctx.item_type == JsItemType::Class)
            .unwrap_or(false)
    });
    assert!(class_chunk.is_some());
    let ctx: JsDocContext = class_chunk.unwrap().context.deserialize_as().unwrap();
    assert_eq!(ctx.item_name.to_string().as_str(), "Counter");
}

#[test]
fn test_extracts_block_comment_variable() {
    let chunker = JsDocChunker::for_javascript(&test_config()).unwrap();
    let input = make_input(
        r#"
/* Default configuration options */
export const CONFIG = {
    timeout: 5000
};
"#,
        "test.js",
    );
    let chunks = chunker.chunk(&input).unwrap();
    let var_chunk = chunks.iter().find(|c| {
        c.context
            .deserialize_as::<JsDocContext>()
            .ok()
            .map(|ctx| ctx.item_type == JsItemType::Variable)
            .unwrap_or(false)
    });
    assert!(var_chunk.is_some());
    let ctx: JsDocContext = var_chunk.unwrap().context.deserialize_as().unwrap();
    assert_eq!(ctx.item_name.to_string().as_str(), "CONFIG");
}

#[test]
fn test_extracts_module_doc() {
    let chunker = JsDocChunker::for_javascript(&test_config()).unwrap();
    let input = make_input(
        r#"/**
 * This module provides utility functions.
 */

import { something } from 'somewhere';

export function util() {}
"#,
        "test.js",
    );
    let chunks = chunker.chunk(&input).unwrap();
    let module_chunk = chunks.iter().find(|c| {
        c.context
            .deserialize_as::<JsDocContext>()
            .ok()
            .map(|ctx| ctx.item_type == JsItemType::Module)
            .unwrap_or(false)
    });
    assert!(module_chunk.is_some());
}

#[test]
fn test_export_vs_local_visibility() {
    let chunker = JsDocChunker::for_javascript(&test_config()).unwrap();
    let input = make_input(
        r#"
/** Exported function */
export function exported() {}

/** Local function */
function local() {}
"#,
        "test.js",
    );
    let chunks = chunker.chunk(&input).unwrap();

    let exported_chunk = chunks.iter().find(|c| {
        c.context
            .deserialize_as::<JsDocContext>()
            .ok()
            .map(|ctx| ctx.item_name.to_string() == "exported")
            .unwrap_or(false)
    });
    let local_chunk = chunks.iter().find(|c| {
        c.context
            .deserialize_as::<JsDocContext>()
            .ok()
            .map(|ctx| ctx.item_name.to_string() == "local")
            .unwrap_or(false)
    });

    assert!(exported_chunk.is_some());
    assert!(local_chunk.is_some());

    let exported_ctx: JsDocContext = exported_chunk.unwrap().context.deserialize_as().unwrap();
    let local_ctx: JsDocContext = local_chunk.unwrap().context.deserialize_as().unwrap();

    assert_eq!(exported_ctx.visibility, JsVisibility::Exported);
    assert_eq!(local_ctx.visibility, JsVisibility::Local);
}

#[test]
fn test_ts_interface() {
    let chunker = JsDocChunker::for_typescript(&test_config()).unwrap();
    let input = make_input(
        r#"
/** User interface definition */
export interface User {
    name: string;
    age: number;
}
"#,
        "test.ts",
    );
    let chunks = chunker.chunk(&input).unwrap();
    let interface_chunk = chunks.iter().find(|c| {
        c.context
            .deserialize_as::<JsDocContext>()
            .ok()
            .map(|ctx| ctx.item_type == JsItemType::Interface)
            .unwrap_or(false)
    });
    assert!(interface_chunk.is_some());
    let ctx: JsDocContext = interface_chunk.unwrap().context.deserialize_as().unwrap();
    assert_eq!(ctx.item_name.to_string().as_str(), "User");
}

#[test]
fn test_ts_type_alias() {
    let chunker = JsDocChunker::for_typescript(&test_config()).unwrap();
    let input = make_input(
        r#"
/** String or number type */
export type StringOrNumber = string | number;
"#,
        "test.ts",
    );
    let chunks = chunker.chunk(&input).unwrap();
    let type_chunk = chunks.iter().find(|c| {
        c.context
            .deserialize_as::<JsDocContext>()
            .ok()
            .map(|ctx| ctx.item_type == JsItemType::TypeAlias)
            .unwrap_or(false)
    });
    assert!(type_chunk.is_some());
    let ctx: JsDocContext = type_chunk.unwrap().context.deserialize_as().unwrap();
    assert_eq!(ctx.item_name.to_string().as_str(), "StringOrNumber");
}

#[test]
fn test_ts_enum() {
    let chunker = JsDocChunker::for_typescript(&test_config()).unwrap();
    let input = make_input(
        r#"
/** Status codes */
export enum Status {
    Active,
    Inactive
}
"#,
        "test.ts",
    );
    let chunks = chunker.chunk(&input).unwrap();
    let enum_chunk = chunks.iter().find(|c| {
        c.context
            .deserialize_as::<JsDocContext>()
            .ok()
            .map(|ctx| ctx.item_type == JsItemType::Enum)
            .unwrap_or(false)
    });
    assert!(enum_chunk.is_some());
    let ctx: JsDocContext = enum_chunk.unwrap().context.deserialize_as().unwrap();
    assert_eq!(ctx.item_name.to_string().as_str(), "Status");
}

#[test]
fn test_skips_undocumented() {
    let chunker = JsDocChunker::for_javascript(&test_config()).unwrap();
    let input = make_input(
        r#"
export function noDoc() {}
"#,
        "test.js",
    );
    let chunks = chunker.chunk(&input).unwrap();
    assert!(chunks.is_empty());
}

//! Integration tests for boost rules and deduplication.
//!
//! Run with: `cargo test --test boost_and_dedup -- --ignored`

mod common;

use common::{crumbly_build, crumbly_cmd, crumbly_status_chunk_count, setup_fixture};
use tempfile::TempDir;

// =============================================================================
// Boost Rules Tests
// =============================================================================

#[test]
#[ignore]
fn boost_rules_ranks_boosted_content_higher() {
    // Given: A workspace where docs/** has 2x boost multiplier
    // Both docs/boot.md and src/boot.rs contain similar "startup" content
    let workspace = setup_fixture("boost_rules");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for content that exists in both locations
    let (code, stdout, _) = crumbly_cmd(
        workspace.path(),
        &[
            "search",
            "--show-chunks",
            "startup process configuration initialization",
        ],
    );

    // Then: Search succeeds
    assert_eq!(code, 0, "Search failed");

    // And: The boosted docs/boot.md should appear before src/boot.rs
    // Find positions of each file in the output
    let docs_pos = stdout.find("docs/boot.md");
    let src_pos = stdout.find("src/boot.rs");

    assert!(
        docs_pos.is_some(),
        "Should find docs/boot.md in results: {}",
        stdout
    );
    assert!(
        src_pos.is_some(),
        "Should find src/boot.rs in results: {}",
        stdout
    );

    // Boosted content should appear first (lower position = earlier in output)
    assert!(
        docs_pos.unwrap() < src_pos.unwrap(),
        "Boosted docs/boot.md should rank higher than src/boot.rs. docs_pos={}, src_pos={}\nOutput: {}",
        docs_pos.unwrap(),
        src_pos.unwrap(),
        stdout
    );
}

// =============================================================================
// Deduplication Tests
// =============================================================================

#[test]
#[ignore]
fn deduplication_identical_content_shares_embeddings() {
    // Given: A workspace with files sharing identical license headers
    let workspace = setup_fixture("deduplication");

    // When: Building the index
    let (code, stdout, stderr) = crumbly_build(workspace.path());

    // Then: Build succeeds without duplicate key errors
    assert_eq!(
        code, 0,
        "Build should succeed with deduplication: {}\n{}",
        stdout, stderr
    );
    assert!(
        !stderr.contains("UNIQUE constraint"),
        "Should not have duplicate key errors"
    );
}

#[test]
#[ignore]
fn deduplication_adding_identical_context_does_not_grow_db() {
    // Given: A temp workspace with two identical directories
    let temp = TempDir::new().unwrap();

    // Create crumbly.toml
    std::fs::write(
        temp.path().join("crumbly.toml"),
        r#"targets = ["."]
"#,
    )
    .unwrap();

    // Create dir_a with some content
    std::fs::create_dir_all(temp.path().join("dir_a")).unwrap();
    std::fs::write(
        temp.path().join("dir_a/readme.md"),
        r#"# Project Documentation

This is the main documentation for the project.

## Overview

The project provides functionality for processing data.
It includes modules for parsing, transforming, and outputting results.

## Installation

Run the installer script to set up the environment.
"#,
    )
    .unwrap();

    // Create dir_b with IDENTICAL content
    std::fs::create_dir_all(temp.path().join("dir_b")).unwrap();
    std::fs::copy(
        temp.path().join("dir_a/readme.md"),
        temp.path().join("dir_b/readme.md"),
    )
    .unwrap();

    // When: Building the first context
    let (code, _, stderr) = crumbly_cmd(temp.path(), &["build", "--context", "dir_a"]);
    assert_eq!(code, 0, "First build failed: {}", stderr);

    // Record chunk count after first context
    let chunks_after_first = crumbly_status_chunk_count(temp.path());
    assert!(chunks_after_first > 0, "Should have indexed some chunks");

    // When: Adding the second context with identical content
    let (code, _, stderr) = crumbly_cmd(temp.path(), &["update", "--context", "dir_b"]);
    assert_eq!(code, 0, "Second context update failed: {}", stderr);

    // Then: Chunk count should NOT double (embeddings are deduplicated)
    let chunks_after_second = crumbly_status_chunk_count(temp.path());

    // The chunk count might increase slightly due to file metadata differences,
    // but should NOT double. Allow up to 20% growth for metadata overhead.
    let max_expected = (chunks_after_first as f64 * 1.2) as usize;
    assert!(
        chunks_after_second <= max_expected,
        "Adding identical content should not significantly grow DB. First: {}, Second: {}, Max expected: {}",
        chunks_after_first,
        chunks_after_second,
        max_expected
    );
}

#[test]
#[ignore]
fn deduplication_rebuild_succeeds() {
    // Given: An existing index
    let workspace = setup_fixture("deduplication");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Initial build failed: {}", stderr);

    // When: Rebuilding the index
    let (code, _, stderr) = crumbly_cmd(workspace.path(), &["rebuild"]);

    // Then: Rebuild succeeds
    assert_eq!(code, 0, "Rebuild should succeed: {}", stderr);
}

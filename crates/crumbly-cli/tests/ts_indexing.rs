//! Integration tests for TypeScript code indexing.
//!
//! Tests TypeScript-specific features: interfaces, type aliases, enums, visibility filtering.
//!
//! Run with: `cargo test --test ts_indexing -- --ignored`

mod common;

use common::{crumbly_build, crumbly_search_with_chunks, setup_fixture};

// =============================================================================
// TypeScript Basic Indexing Tests
// =============================================================================

#[test]
#[ignore]
fn ts_basic_indexes_and_searches() {
    // Given: A workspace with TypeScript source files
    let workspace = setup_fixture("ts_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for interface content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "authentication user identity credentials");

    // Then: Interface is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("AuthenticationUser") || stdout.contains("authentication"),
        "Should find AuthenticationUser interface: {}",
        stdout
    );
}

#[test]
#[ignore]
fn ts_basic_finds_type_aliases() {
    // Given: An indexed TypeScript workspace
    let workspace = setup_fixture("ts_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for type alias content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "validation pipeline config strict");

    // Then: Type alias is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("ValidationPipelineConfig") || stdout.contains("validation"),
        "Should find ValidationPipelineConfig type alias: {}",
        stdout
    );
}

// =============================================================================
// TypeScript Visibility Filtering Tests
// =============================================================================

#[test]
#[ignore]
fn ts_visibility_indexes_exported_items() {
    // Given: A workspace with visibility = ["public"]
    let workspace = setup_fixture("ts_visibility");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for exported interface content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "database connection settings persistence");

    // Then: Exported interface is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("DatabaseConnectionSettings") || stdout.contains("database"),
        "Should find exported DatabaseConnectionSettings: {}",
        stdout
    );
}

#[test]
#[ignore]
fn ts_visibility_excludes_local_items() {
    // Given: A workspace with visibility = ["public"]
    let workspace = setup_fixture("ts_visibility");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for local type alias content
    let (code, stdout, _) = crumbly_search_with_chunks(
        workspace.path(),
        "internal cache configuration memory eviction",
    );

    // Then: Local type alias should not appear in results
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("InternalCacheConfiguration"),
        "Should NOT find local InternalCacheConfiguration: {}",
        stdout
    );
}

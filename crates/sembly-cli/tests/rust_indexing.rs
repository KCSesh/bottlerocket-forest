//! Integration tests for Rust code indexing.
//!
//! Tests Rust-specific features: visibility filtering, item type filtering.
//!
//! Run with: `cargo test --test rust_indexing -- --ignored`

mod common;

use common::{sembly_build, sembly_search_with_chunks, setup_fixture};

// =============================================================================
// Rust Visibility Filtering Tests
// =============================================================================

#[test]
#[ignore]
fn rust_visibility_indexes_only_public_items() {
    // Given: A workspace with visibility = "public"
    let workspace = setup_fixture("rust_visibility");
    let (code, _, stderr) = sembly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for public function
    let (code, stdout, _) = sembly_search_with_chunks(workspace.path(), "initialization app");

    // Then: Public function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("init") || stdout.contains("app"),
        "Should find public init_app: {}",
        stdout
    );
}

#[test]
#[ignore]
fn rust_visibility_excludes_private_items() {
    // Given: A workspace with visibility = "public"
    let workspace = setup_fixture("rust_visibility");
    let (code, _, stderr) = sembly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for private function content
    let (code, stdout, _) =
        sembly_search_with_chunks(workspace.path(), "memory pools caching layers allocates");

    // Then: Private function should not appear in results
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("setup_memory"),
        "Should NOT find private setup_memory: {}",
        stdout
    );
}

// =============================================================================
// Rust Item Type Filtering Tests
// =============================================================================

#[test]
#[ignore]
fn rust_item_types_indexes_functions_and_structs() {
    // Given: A workspace with item_types = ["function", "struct"]
    let workspace = setup_fixture("rust_item_types");
    let (code, _, stderr) = sembly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for function content
    let (code, stdout, _) = sembly_search_with_chunks(workspace.path(), "process request handler");

    // Then: Function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("process") || stdout.contains("request"),
        "Should find function: {}",
        stdout
    );

    // When: Searching for struct content
    let (code, stdout, _) = sembly_search_with_chunks(workspace.path(), "request path method");

    // Then: Struct is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("Request") || stdout.contains("request"),
        "Should find struct: {}",
        stdout
    );
}

#[test]
#[ignore]
fn rust_item_types_excludes_enums_and_traits() {
    // Given: A workspace with item_types = ["function", "struct"]
    let workspace = setup_fixture("rust_item_types");
    let (code, _, stderr) = sembly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for enum content
    let (code, stdout, _) =
        sembly_search_with_chunks(workspace.path(), "response status ok bad request error");

    // Then: Enum should not be in results
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("ResponseStatus"),
        "Should NOT find enum ResponseStatus: {}",
        stdout
    );

    // When: Searching for trait content
    let (code, stdout, _) =
        sembly_search_with_chunks(workspace.path(), "handler trait implement endpoint");

    // Then: Trait should not be in results
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("RequestHandler"),
        "Should NOT find trait RequestHandler: {}",
        stdout
    );
}

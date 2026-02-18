//! Integration tests for Shell/Bash script indexing.
//!
//! Tests shell-specific features: function extraction, standalone comments, item type filtering.
//!
//! Run with: `cargo test --test shell_indexing -- --ignored`

mod common;

use common::{crumbly_build, crumbly_search_with_chunks, setup_fixture};

// =============================================================================
// Shell Basic Indexing Tests
// =============================================================================

#[test]
#[ignore]
fn shell_basic_indexes_and_searches() {
    // Given: A workspace with shell scripts
    let workspace = setup_fixture("shell_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for function content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "deploy application container orchestration");

    // Then: Function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("deploy") || stdout.contains("application"),
        "Should find deploy_application function: {}",
        stdout
    );
}

#[test]
#[ignore]
fn shell_basic_finds_standalone_comments() {
    // Given: A workspace with shell scripts
    let workspace = setup_fixture("shell_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for standalone comment content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "deployment strategy blue-green methodology");

    // Then: Standalone comment is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("deployment") || stdout.contains("blue-green"),
        "Should find standalone comment about deployment: {}",
        stdout
    );
}

// =============================================================================
// Shell Item Type Filtering Tests
// =============================================================================

#[test]
#[ignore]
fn shell_item_types_indexes_functions_only() {
    // Given: A workspace with items = ["function"]
    let workspace = setup_fixture("shell_item_types");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for function content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "initialize database connections schemas");

    // Then: Function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("database") || stdout.contains("initialize"),
        "Should find initialize_database function: {}",
        stdout
    );
}

#[test]
#[ignore]
fn shell_item_types_excludes_standalone_comments() {
    // Given: A workspace with items = ["function"]
    let workspace = setup_fixture("shell_item_types");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for standalone comment content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "initialization procedures bootstrap sequence");

    // Then: Standalone comment should NOT be in results
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("bootstrap") && !stdout.contains("initialization procedures"),
        "Should NOT find standalone comment when items=[function]: {}",
        stdout
    );
}

//! Integration tests for Markdown documentation indexing.
//!
//! Run with: `cargo test --test markdown_indexing -- --ignored`

mod common;

use common::{sembly_build, sembly_search_with_chunks, setup_fixture};

#[test]
#[ignore]
fn markdown_docs_indexes_and_searches() {
    // Given: A workspace with markdown documentation
    let workspace = setup_fixture("markdown_docs");
    let (code, _, stderr) = sembly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for startup process content
    let (code, stdout, _) =
        sembly_search_with_chunks(workspace.path(), "startup process stages configuration");

    // Then: Startup process content is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("Startup")
            || stdout.contains("startup")
            || stdout.contains("initialization"),
        "Should find startup process content: {}",
        stdout
    );
}

#[test]
#[ignore]
fn markdown_docs_finds_configuration_content() {
    // Given: An indexed markdown workspace
    let workspace = setup_fixture("markdown_docs");
    let (code, _, stderr) = sembly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for database configuration
    let (code, stdout, _) =
        sembly_search_with_chunks(workspace.path(), "database connection settings host port");

    // Then: Database configuration content is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("database") || stdout.contains("Database") || stdout.contains("connection"),
        "Should find database content: {}",
        stdout
    );
}

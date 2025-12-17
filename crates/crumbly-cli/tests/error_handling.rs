//! Integration tests for error handling and diagnostics.
//!
//! Run with: `cargo test --test error_handling -- --ignored`

mod common;

use common::{crumbly_build_context, crumbly_cmd, setup_fixture};
use tempfile::TempDir;

#[test]
#[ignore]
fn error_unregistered_context_lists_available() {
    // Given: A workspace with only "main" context (NOT the default "." context)
    let workspace = setup_fixture("context_boost");
    let (code, _, stderr) = crumbly_build_context(workspace.path(), "main");
    assert_eq!(code, 0, "Main context build failed: {}", stderr);

    // When: Running crumbly search from workspace root (which is NOT inside "main" context)
    let (code, _, stderr) = crumbly_cmd(workspace.path(), &["search", "test"]);

    // Then: Command fails with non-zero exit code
    assert_ne!(
        code, 0,
        "Search should fail when cwd is not in a registered context"
    );

    // And: Error message lists available contexts (should mention "main")
    assert!(
        stderr.contains("main"),
        "Error should list available contexts including 'main': {}",
        stderr
    );
}

#[test]
#[ignore]
fn error_missing_database_suggests_build() {
    // Given: A directory with only crumbly.toml (no database)
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("crumbly.toml"), "targets = [\".\"]").unwrap();

    // When: Running crumbly search
    let (code, _, stderr) = crumbly_cmd(temp.path(), &["search", "test"]);

    // Then: Command fails with non-zero exit code
    assert_ne!(code, 0, "Search should fail when no database exists");

    // And: Error message suggests how to create an index
    assert!(
        stderr.contains("build") || stderr.contains("crumbly build"),
        "Error should suggest 'build' or 'crumbly build': {}",
        stderr
    );
}

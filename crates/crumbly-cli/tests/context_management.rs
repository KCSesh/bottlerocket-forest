//! Integration tests for multi-context management.
//!
//! Tests context creation, isolation, removal, garbage collection, and clear.
//!
//! Run with: `cargo test --test context_management -- --ignored`

mod common;

use common::{
    crumbly_build, crumbly_build_context, crumbly_cmd, crumbly_search_context,
    crumbly_search_with_chunks, crumbly_status_chunk_count, crumbly_update_context, setup_fixture,
};

// =============================================================================
// Multi-Context Tests
// =============================================================================

#[test]
#[ignore]
fn context_multiple_contexts_can_be_indexed() {
    // Given: A workspace with multiple context directories
    let workspace = setup_fixture("context_boost");

    // When: Building the main context first (creates the index)
    let (code, stdout, stderr) = crumbly_build_context(workspace.path(), "main");
    assert_eq!(code, 0, "Main context build failed: {}\n{}", stdout, stderr);

    // When: Adding the worktree context using update
    let (code, stdout, stderr) = crumbly_update_context(workspace.path(), "worktree");
    assert_eq!(
        code, 0,
        "Worktree context update failed: {}\n{}",
        stdout, stderr
    );

    // Then: Both contexts should be searchable
    let (code, stdout, _) =
        crumbly_search_context(workspace.path(), "database connection handler", "main");
    assert_eq!(code, 0, "Search in main context failed");
    assert!(
        stdout.contains("database") || stdout.contains("Database"),
        "Should find database content in main: {}",
        stdout
    );

    let (code, stdout, _) =
        crumbly_search_context(workspace.path(), "database connection handler", "worktree");
    assert_eq!(code, 0, "Search in worktree context failed");
    assert!(
        stdout.contains("database") || stdout.contains("Database"),
        "Should find database content in worktree: {}",
        stdout
    );
}

#[test]
#[ignore]
fn context_list_shows_registered_contexts() {
    // Given: A workspace with multiple contexts
    let workspace = setup_fixture("context_boost");
    let (code, _, _) = crumbly_build_context(workspace.path(), "main");
    assert_eq!(code, 0);
    let (code, _, _) = crumbly_update_context(workspace.path(), "worktree");
    assert_eq!(code, 0);

    // When: Listing contexts
    let (code, stdout, _) = crumbly_cmd(workspace.path(), &["context", "list"]);

    // Then: Both contexts should be listed
    assert_eq!(code, 0, "Context list failed");
    assert!(
        stdout.contains("main"),
        "Should list main context: {}",
        stdout
    );
    assert!(
        stdout.contains("worktree"),
        "Should list worktree context: {}",
        stdout
    );
}

#[test]
#[ignore]
fn context_isolation_prevents_cross_context_results() {
    // Given: A workspace with two contexts containing different content
    let workspace = setup_fixture("context_boost");

    let (code, _, stderr) = crumbly_build_context(workspace.path(), "main");
    assert_eq!(code, 0, "Main context build failed: {}", stderr);
    let (code, _, stderr) = crumbly_update_context(workspace.path(), "worktree");
    assert_eq!(code, 0, "Worktree context update failed: {}", stderr);

    // When: Searching in main context for content that only exists in worktree
    let (code, stdout, _) = crumbly_search_context(
        workspace.path(),
        "async connection pool new strategy",
        "main",
    );

    // Then: Should NOT find worktree-specific content when searching main context
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("async") && !stdout.contains("new strategy"),
        "Main context should not leak worktree content: {}",
        stdout
    );
}

#[test]
#[ignore]
fn context_default_build_creates_dot_context() {
    // Given: A workspace with Rust files
    let workspace = setup_fixture("rust_basic");

    // When: Running crumbly build without --context
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Initial build failed: {}", stderr);

    // When: Listing contexts
    let (code, stdout, _) = crumbly_cmd(workspace.path(), &["context", "list"]);

    // Then: The "." context should be listed
    assert_eq!(code, 0, "Context list failed");
    assert!(
        stdout.contains("."),
        "Should list default '.' context: {}",
        stdout
    );

    // When: Running crumbly build again without --context
    let (code, _, stderr) = crumbly_build(workspace.path());

    // Then: Build should fail with diagnostic suggesting update or clear
    assert_ne!(code, 0, "Second build should fail when context exists");
    assert!(
        stderr.contains("update") || stderr.contains("clear"),
        "Error should suggest 'update' or 'clear': {}",
        stderr
    );
}

#[test]
#[ignore]
fn context_remove_deletes_context() {
    // Given: A workspace with two contexts
    let workspace = setup_fixture("context_boost");
    let (code, _, stderr) = crumbly_build_context(workspace.path(), "main");
    assert_eq!(code, 0, "Main context build failed: {}", stderr);
    let (code, _, stderr) = crumbly_update_context(workspace.path(), "worktree");
    assert_eq!(code, 0, "Worktree context update failed: {}", stderr);

    // Verify both contexts exist
    let (_, stdout, _) = crumbly_cmd(workspace.path(), &["context", "list"]);
    assert!(stdout.contains("main") && stdout.contains("worktree"));

    // When: Removing the worktree context
    let (code, _, stderr) = crumbly_cmd(workspace.path(), &["context", "remove", "worktree"]);
    assert_eq!(code, 0, "Context remove failed: {}", stderr);

    // Then: worktree context should no longer be listed
    let (_, stdout, _) = crumbly_cmd(workspace.path(), &["context", "list"]);
    assert!(
        !stdout.contains("worktree"),
        "worktree should be removed: {}",
        stdout
    );
    assert!(
        stdout.contains("main"),
        "main context should still exist: {}",
        stdout
    );
}

#[test]
#[ignore]
fn context_remove_nonexistent_fails() {
    // Given: A workspace with only the main context
    let workspace = setup_fixture("context_boost");
    let (code, _, stderr) = crumbly_build_context(workspace.path(), "main");
    assert_eq!(code, 0, "Main context build failed: {}", stderr);

    // When: Attempting to remove a nonexistent context
    let (code, _, _) = crumbly_cmd(workspace.path(), &["context", "remove", "nonexistent"]);

    // Then: Command should fail
    assert_ne!(code, 0, "Removing nonexistent context should fail");
}

#[test]
#[ignore]
fn context_remove_default_fails() {
    // Given: A workspace with the default "." context
    let workspace = setup_fixture("rust_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Attempting to remove the default context
    let (code, _, stderr) = crumbly_cmd(workspace.path(), &["context", "remove", "."]);

    // Then: Command should fail
    assert_ne!(code, 0, "Removing default context should fail");
    assert!(
        stderr.contains("default") || stderr.contains("cannot"),
        "Error should mention default context cannot be removed: {}",
        stderr
    );
}

// =============================================================================
// Garbage Collection Tests
// =============================================================================

#[test]
#[ignore]
fn gc_removes_unreferenced_embeddings() {
    // Given: A workspace with two contexts
    let workspace = setup_fixture("context_boost");
    let (code, _, stderr) = crumbly_build_context(workspace.path(), "main");
    assert_eq!(code, 0, "Main context build failed: {}", stderr);
    let (code, _, stderr) = crumbly_update_context(workspace.path(), "worktree");
    assert_eq!(code, 0, "Worktree context update failed: {}", stderr);

    // Record chunk count with both contexts
    let chunks_before_remove = crumbly_status_chunk_count(workspace.path());
    assert!(chunks_before_remove > 0, "Should have chunks indexed");

    // When: Removing the worktree context (leaves orphaned embeddings)
    let (code, _, stderr) = crumbly_cmd(workspace.path(), &["context", "remove", "worktree"]);
    assert_eq!(code, 0, "Context remove failed: {}", stderr);

    // Chunk count should be unchanged (orphans still exist)
    let chunks_after_remove = crumbly_status_chunk_count(workspace.path());
    assert_eq!(
        chunks_after_remove, chunks_before_remove,
        "Chunks should not be deleted yet (orphans remain)"
    );

    // When: Running garbage collection
    let (code, stdout, stderr) = crumbly_cmd(workspace.path(), &["gc"]);
    assert_eq!(code, 0, "GC failed: stdout={}\nstderr={}", stdout, stderr);

    // Then: Chunk count should decrease (orphans removed)
    let chunks_after_gc = crumbly_status_chunk_count(workspace.path());
    assert!(
        chunks_after_gc < chunks_before_remove,
        "GC should have removed orphaned chunks: before={}, after={}",
        chunks_before_remove,
        chunks_after_gc
    );
}

// =============================================================================
// Clear Tests
// =============================================================================

#[test]
#[ignore]
fn context_clear_removes_file_mappings() {
    // Given: A workspace with an indexed default context
    let workspace = setup_fixture("rust_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // And: Search returns results
    let (code, stdout, _) = crumbly_search_with_chunks(workspace.path(), "startup initialization");
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("Found") && stdout.contains("unique files"),
        "Should find results before clear: {}",
        stdout
    );

    // When: Running crumbly clear
    let (code, _, stderr) = crumbly_cmd(workspace.path(), &["clear"]);
    assert_eq!(code, 0, "Clear failed: {}", stderr);

    // Then: Context still exists in context list
    let (_, stdout, _) = crumbly_cmd(workspace.path(), &["context", "list"]);
    assert!(
        stdout.contains("."),
        "Context should still be registered after clear: {}",
        stdout
    );

    // And: Search returns no results (file mappings removed)
    let (code, stdout, _) = crumbly_search_with_chunks(workspace.path(), "startup initialization");
    assert_eq!(code, 0, "Search after clear failed");
    assert!(
        stdout.contains("Found 0") || stdout.contains("No results"),
        "Should find no results after clear: {}",
        stdout
    );
}

// =============================================================================
// Update Tests
// =============================================================================

#[test]
#[ignore]
fn update_only_affects_current_context() {
    // Given: A workspace with main context built
    let workspace = setup_fixture("context_boost");
    let (code, _, stderr) = crumbly_build_context(workspace.path(), "main");
    assert_eq!(code, 0, "Main context build failed: {}", stderr);

    // And: worktree context added
    let (code, _, stderr) = crumbly_update_context(workspace.path(), "worktree");
    assert_eq!(code, 0, "Worktree context update failed: {}", stderr);

    // When: Adding a NEW file in main context with unique content
    let new_file_path = workspace.path().join("main/docs/zebra_unicorn.md");
    let new_content = r#"# Zebra Unicorn Documentation

This document describes the zebra unicorn integration.

## Overview

The zebra unicorn module provides magical functionality.
Zebra unicorn handlers process rainbow data efficiently.
"#;
    std::fs::write(&new_file_path, new_content).unwrap();

    // And: Running crumbly update for main context only
    let (code, stdout, stderr) = crumbly_update_context(workspace.path(), "main");
    assert_eq!(
        code, 0,
        "Update should succeed: stdout={}\nstderr={}",
        stdout, stderr
    );

    // And: Searching in main context finds the new content
    let (code, stdout, _) =
        crumbly_search_context(workspace.path(), "zebra unicorn rainbow", "main");
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("zebra") || stdout.contains("Zebra") || stdout.contains("unicorn"),
        "Main context should find new content after update: {}",
        stdout
    );

    // And: Worktree context should NOT have the new content (isolation test)
    let (code, stdout, _) =
        crumbly_search_context(workspace.path(), "zebra unicorn rainbow", "worktree");
    assert_eq!(code, 0, "Search in worktree failed");
    assert!(
        stdout.contains("No results") || (!stdout.contains("zebra") && !stdout.contains("unicorn")),
        "Worktree context should NOT find main's new content: {}",
        stdout
    );
}

//! Integration tests for forester seed and worktree commands.
//!
//! These tests use local git repos to avoid network dependencies.

mod common;

use common::{create_bare_repo, forester_seed, forester_worktree, temp_forest};
use std::fs;

fn setup_forest_with_local_repos() -> tempfile::TempDir {
    let temp = temp_forest();
    
    // Create local bare repos to use as "remotes"
    let repos_dir = temp.path().join("repos");
    fs::create_dir_all(&repos_dir).unwrap();
    
    let repo_a = create_bare_repo(&repos_dir, "repo-a");
    let repo_b = create_bare_repo(&repos_dir, "repo-b");
    
    // Create forester.toml pointing to local repos
    let forester_toml = format!(
        r#"[forest]
name = "test-forest"

[[member]]
name = "repo-a"
remote = "{}"
path = "repo-a"
default_branch = "main"

[[member]]
name = "repo-b"
remote = "{}"
path = "nested/repo-b"
default_branch = "main"
"#,
        repo_a.display(),
        repo_b.display()
    );
    fs::write(temp.path().join("forester.toml"), forester_toml).unwrap();
    
    // Create sembly.toml
    fs::write(temp.path().join("sembly.toml"), "targets = [\".\"]\n").unwrap();
    
    temp
}

#[test]
fn seed_clones_bare_repos() {
    // Given: A forest with local repo remotes
    let temp = setup_forest_with_local_repos();

    // When: Running forester seed
    let (code, stdout, stderr) = forester_seed(temp.path(), true);

    // Then: Command succeeds
    assert_eq!(code, 0, "Seed failed: {} {}", stdout, stderr);

    // And: Bare repos are created
    assert!(temp.path().join(".forest/bare/repo-a.git").exists());
    assert!(temp.path().join(".forest/bare/repo-b.git").exists());
}

#[test]
fn seed_creates_develop_worktree() {
    // Given: A forest with local repo remotes
    let temp = setup_forest_with_local_repos();

    // When: Running forester seed
    let (code, _, _) = forester_seed(temp.path(), false);
    assert_eq!(code, 0);

    // Then: worktrees/develop is created
    assert!(temp.path().join("worktrees/develop").exists());
    
    // And: Member repos are checked out in correct paths
    assert!(temp.path().join("worktrees/develop/repo-a").exists());
    assert!(temp.path().join("worktrees/develop/nested/repo-b").exists());
    
    // And: README.md exists (from our test commit)
    assert!(temp.path().join("worktrees/develop/repo-a/README.md").exists());
}

#[test]
fn seed_is_idempotent() {
    // Given: A forest that has already been seeded
    let temp = setup_forest_with_local_repos();
    let (code, _, _) = forester_seed(temp.path(), false);
    assert_eq!(code, 0);

    // When: Running forester seed again
    let (code, stdout, stderr) = forester_seed(temp.path(), true);

    // Then: Command succeeds
    assert_eq!(code, 0, "Second seed failed: {} {}", stdout, stderr);
    
    // And: Shows repos already exist
    assert!(stdout.contains("already exists") || stdout.contains("✓"));
}

#[test]
fn worktree_create_makes_new_worktree() {
    // Given: A seeded forest
    let temp = setup_forest_with_local_repos();
    let (code, _, _) = forester_seed(temp.path(), false);
    assert_eq!(code, 0);

    // When: Creating a new worktree
    let (code, stdout, stderr) = forester_worktree(temp.path(), "create", &["feature-x"]);

    // Then: Command succeeds
    assert_eq!(code, 0, "Worktree create failed: {} {}", stdout, stderr);

    // And: New worktree directory exists
    assert!(temp.path().join("worktrees/feature-x").exists());
    assert!(temp.path().join("worktrees/feature-x/repo-a").exists());
    assert!(temp.path().join("worktrees/feature-x/nested/repo-b").exists());
}

#[test]
fn worktree_list_shows_worktrees() {
    // Given: A seeded forest with an additional worktree
    let temp = setup_forest_with_local_repos();
    forester_seed(temp.path(), false);
    forester_worktree(temp.path(), "create", &["feature-x"]);

    // When: Listing worktrees
    let (code, stdout, _) = forester_worktree(temp.path(), "list", &[]);

    // Then: Both worktrees are listed
    assert_eq!(code, 0);
    assert!(stdout.contains("develop"));
    assert!(stdout.contains("feature-x"));
}

#[test]
fn worktree_remove_deletes_worktree() {
    // Given: A seeded forest with an additional worktree
    let temp = setup_forest_with_local_repos();
    forester_seed(temp.path(), false);
    forester_worktree(temp.path(), "create", &["feature-x"]);
    assert!(temp.path().join("worktrees/feature-x").exists());

    // When: Removing the worktree
    let (code, stdout, stderr) = forester_worktree(temp.path(), "remove", &["feature-x"]);

    // Then: Command succeeds
    assert_eq!(code, 0, "Worktree remove failed: {} {}", stdout, stderr);

    // And: Worktree directory is gone
    assert!(!temp.path().join("worktrees/feature-x").exists());
    
    // And: develop worktree still exists
    assert!(temp.path().join("worktrees/develop").exists());
}

#[test]
fn no_repos_at_forest_root() {
    // Given: A seeded forest
    let temp = setup_forest_with_local_repos();
    forester_seed(temp.path(), false);

    // Then: No repo directories at forest root (only in worktrees/)
    assert!(!temp.path().join("repo-a").exists());
    assert!(!temp.path().join("nested").exists());
    
    // And: Only config files and directories at root
    let entries: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    
    // Should only have: forester.toml, sembly.toml, .forest, worktrees, repos (our test remotes)
    for entry in &entries {
        assert!(
            ["forester.toml", "sembly.toml", ".forest", "worktrees", "repos"].contains(&entry.as_str()),
            "Unexpected entry at forest root: {}",
            entry
        );
    }
}

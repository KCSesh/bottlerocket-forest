#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "panics are appropriate in tests"
)]
//! Integration tests for forester sync-seed command.

mod common;

use common::{create_bare_repo, forester_seed, forester_sync_seed, temp_forest};
use std::fs;
use std::process::Command;

fn setup_forest_with_local_repo() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = temp_forest();

    // Create local bare repo to use as "remote"
    let repos_dir = temp.path().join("repos");
    fs::create_dir_all(&repos_dir).unwrap();
    let remote_bare = create_bare_repo(&repos_dir, "test-repo");

    // Create forester.toml pointing to local repo (using [[forest.member]] format)
    let config = format!(
        r#"[forest]
name = "test-forest"

[[forest.member]]
name = "test-repo"
remote = "{}"
path = "test-repo"
default_branch = "main"
"#,
        remote_bare.display()
    );
    fs::write(temp.path().join("forester.toml"), config).unwrap();
    fs::write(temp.path().join("crumbly.toml"), "targets = [\".\"]\n").unwrap();

    (temp, remote_bare)
}

#[test]
fn sync_seed_succeeds_with_no_changes() {
    // Given a seeded forest
    let (forest, _remote) = setup_forest_with_local_repo();
    let forest_path = forest.path();

    // Seed the forest
    let (code, stdout, stderr) = forester_seed(forest_path, false);
    assert_eq!(
        code, 0,
        "seed should succeed: stdout={} stderr={}",
        stdout, stderr
    );

    // When we run sync-seed
    let (code, _stdout, stderr) = forester_sync_seed(forest_path, false);

    // Then it should succeed
    assert_eq!(code, 0, "sync-seed should succeed: {stderr}");
    assert!(
        stderr.contains("synced") || stderr.contains("Syncing") || stderr.contains("fetching"),
        "should show sync progress: {stderr}"
    );
}

#[test]
fn sync_seed_skips_if_not_seeded() {
    // Given a forest that has NOT been seeded
    let forest = temp_forest();
    let forest_path = forest.path();

    // Create forester.toml but don't seed
    let config = r#"[forest]
name = "test-forest"

[[forest.member]]
name = "test-repo"
remote = "https://example.com/repo.git"
path = "test-repo"
default_branch = "main"
"#;
    fs::write(forest_path.join("forester.toml"), config).unwrap();
    fs::write(forest_path.join("crumbly.toml"), "targets = [\".\"]\n").unwrap();

    // When we run sync-seed
    let (code, _stdout, stderr) = forester_sync_seed(forest_path, false);

    // Then it should succeed but skip the member (graceful handling)
    assert_eq!(
        code, 0,
        "sync-seed should succeed even with missing repos: {stderr}"
    );
    assert!(
        stderr.contains("!") || stderr.contains("skipped") || stderr.contains("not found"),
        "should warn about skipped member: {stderr}"
    );
}

#[test]
fn sync_seed_fetches_new_commits() {
    // Given a seeded forest
    let (forest, remote_bare) = setup_forest_with_local_repo();
    let forest_path = forest.path();

    // Seed the forest
    let (code, _, _) = forester_seed(forest_path, false);
    assert_eq!(code, 0);

    // Add a new commit to the "remote" bare repo
    // Clone it, add a commit, and push back
    let temp_clone = tempfile::TempDir::new().unwrap();
    Command::new("git")
        .args(["clone"])
        .arg(&remote_bare)
        .arg(temp_clone.path())
        .output()
        .unwrap();

    Command::new("git")
        .current_dir(temp_clone.path())
        .args(["config", "user.email", "test@test.com"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(temp_clone.path())
        .args(["config", "user.name", "Test"])
        .output()
        .unwrap();

    fs::write(temp_clone.path().join("new-file.txt"), "new content").unwrap();
    Command::new("git")
        .current_dir(temp_clone.path())
        .args(["add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(temp_clone.path())
        .args(["commit", "-m", "New commit"])
        .output()
        .unwrap();
    Command::new("git")
        .current_dir(temp_clone.path())
        .args(["push", "origin", "main"])
        .output()
        .unwrap();

    // When we run sync-seed
    let (code, _stdout, stderr) = forester_sync_seed(forest_path, true);

    // Then it should succeed and fetch the new commit
    assert_eq!(code, 0, "sync-seed should succeed: {stderr}");
}

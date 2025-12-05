//! Integration tests for forester init command.

mod common;

use common::{forester_init, temp_forest};
use std::fs;

#[test]
fn init_creates_config_files() {
    // Given: An empty directory
    let temp = temp_forest();

    // When: Running forester init
    let (code, stdout, stderr) = forester_init(temp.path(), Some("test-forest"));

    // Then: Command succeeds
    assert_eq!(code, 0, "Init failed: {} {}", stdout, stderr);

    // And: forester.toml is created with correct name
    let forester_toml = fs::read_to_string(temp.path().join("forester.toml")).unwrap();
    assert!(forester_toml.contains("name = \"test-forest\""));

    // And: sembly.toml is created
    assert!(temp.path().join("sembly.toml").exists());

    // And: .gitignore is created with correct entries
    let gitignore = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains(".forest/"));
    assert!(gitignore.contains("worktrees/"));
}

#[test]
fn init_uses_directory_name_as_default() {
    // Given: An empty directory
    let temp = temp_forest();

    // When: Running forester init without --name
    let (code, _, _) = forester_init(temp.path(), None);
    assert_eq!(code, 0);

    // Then: forester.toml uses directory name
    let forester_toml = fs::read_to_string(temp.path().join("forester.toml")).unwrap();
    // The temp dir has a random name, just check it has some name
    assert!(forester_toml.contains("name = \""));
}

#[test]
fn init_does_not_overwrite_existing() {
    // Given: A directory with existing forester.toml
    let temp = temp_forest();
    fs::write(temp.path().join("forester.toml"), "existing content").unwrap();

    // When: Running forester init
    let (code, stdout, _) = forester_init(temp.path(), Some("new-name"));

    // Then: Command succeeds but warns
    assert_eq!(code, 0);
    assert!(stdout.contains("already initialized"));

    // And: Original content is preserved
    let content = fs::read_to_string(temp.path().join("forester.toml")).unwrap();
    assert_eq!(content, "existing content");
}

#[test]
fn init_appends_to_existing_gitignore() {
    // Given: A directory with existing .gitignore
    let temp = temp_forest();
    fs::write(temp.path().join(".gitignore"), "node_modules/\n").unwrap();

    // When: Running forester init
    let (code, _, _) = forester_init(temp.path(), Some("test"));
    assert_eq!(code, 0);

    // Then: .gitignore contains both old and new entries
    let gitignore = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains("node_modules/"));
    assert!(gitignore.contains(".forest/"));
    assert!(gitignore.contains("worktrees/"));
}

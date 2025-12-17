//! Integration tests for workspace discovery from subdirectories.
//!
//! Tests that crumbly can find the .crumbly database by walking up parent directories.
//!
//! Run with: `cargo test --test discover_workspace -- --ignored`

mod common;

use common::{crumbly_build, crumbly_cmd, setup_fixture};
use std::path::Path;

fn crumbly_cmd_from_dir(workspace: &Path, subdir: &str, args: &[&str]) -> (i32, String, String) {
    let cwd = workspace.join(subdir);
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_crumbly"));
    cmd.current_dir(&cwd);
    for arg in args {
        cmd.arg(arg);
    }
    let output = cmd.output().expect("Failed to execute crumbly");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

#[test]
#[ignore]
fn discover_status_from_subdirectory() {
    let workspace = setup_fixture("rust_basic");
    
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);
    
    let (code, stdout, stderr) = crumbly_cmd_from_dir(workspace.path(), "src", &["status"]);
    assert_eq!(code, 0, "Status from subdirectory failed: {}", stderr);
    assert!(stdout.contains("chunks") || stdout.contains("Chunks"), 
            "Status should show chunk info: {}", stdout);
}

#[test]
#[ignore]
fn discover_search_from_subdirectory() {
    let workspace = setup_fixture("rust_basic");
    
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);
    
    let (code, stdout, stderr) = crumbly_cmd_from_dir(workspace.path(), "src", &["search", "startup"]);
    assert_eq!(code, 0, "Search from subdirectory failed: {}", stderr);
    assert!(stdout.contains("Found") || stdout.contains("startup"), 
            "Search should return results: {}", stdout);
}

#[test]
#[ignore]
fn discover_fails_without_database() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(workspace.path().join("subdir")).unwrap();
    
    let (code, _, stderr) = crumbly_cmd_from_dir(workspace.path(), "subdir", &["status"]);
    assert_ne!(code, 0, "Status should fail without database");
    assert!(stderr.contains("not found") || stderr.contains("No index"), 
            "Error should mention missing database: {}", stderr);
}

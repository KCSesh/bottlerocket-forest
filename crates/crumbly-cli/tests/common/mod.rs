#![allow(dead_code)]
//! Common test utilities for crumbly integration tests.

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Get path to crumbly binary
pub fn crumbly_bin() -> &'static Path {
    assert_cmd::cargo::cargo_bin!("crumbly")
}

/// Copies a fixture directory to a temp location for testing
pub fn setup_fixture(fixture_name: &str) -> TempDir {
    let temp = TempDir::new().unwrap();
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture_name);

    copy_dir_recursive(&fixture_path, temp.path()).unwrap();
    temp
}

/// Recursively copy a directory
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Run crumbly build in the given directory
pub fn crumbly_build(workspace: &Path) -> (i32, String, String) {
    let output = Command::new(crumbly_bin())
        .current_dir(workspace)
        .args(["build"])
        .output()
        .expect("Failed to execute crumbly");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

/// Run crumbly build with --context flag
pub fn crumbly_build_context(workspace: &Path, context: &str) -> (i32, String, String) {
    let output = Command::new(crumbly_bin())
        .current_dir(workspace)
        .args(["build", "--context", context])
        .output()
        .expect("Failed to execute crumbly");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

/// Run crumbly search with --show-chunks flag
pub fn crumbly_search_with_chunks(workspace: &Path, query: &str) -> (i32, String, String) {
    let output = Command::new(crumbly_bin())
        .current_dir(workspace)
        .args(["search", "--show-chunks", query])
        .output()
        .expect("Failed to execute crumbly");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

/// Run crumbly update with --context flag
pub fn crumbly_update_context(workspace: &Path, context: &str) -> (i32, String, String) {
    let output = Command::new(crumbly_bin())
        .current_dir(workspace)
        .args(["update", "--context", context])
        .output()
        .expect("Failed to execute crumbly");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

/// Run crumbly search with --context flag
pub fn crumbly_search_context(
    workspace: &Path,
    query: &str,
    context: &str,
) -> (i32, String, String) {
    let output = Command::new(crumbly_bin())
        .current_dir(workspace)
        .args(["search", "--show-chunks", "--context", context, query])
        .output()
        .expect("Failed to execute crumbly");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

/// Run crumbly status and return chunk count
pub fn crumbly_status_chunk_count(workspace: &Path) -> usize {
    let output = Command::new(crumbly_bin())
        .current_dir(workspace)
        .args(["status"])
        .output()
        .expect("Failed to execute crumbly");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Parse "Chunks: 123" from output (strip ANSI codes)
    for line in stdout.lines() {
        if line.contains("Chunks:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(count_str) = parts.last() {
                let clean = count_str
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>();
                return clean.parse().unwrap_or(0);
            }
        }
    }
    0
}

/// Run a crumbly command and return (exit_code, stdout, stderr)
pub fn crumbly_cmd(workspace: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(crumbly_bin())
        .current_dir(workspace)
        .args(args)
        .output()
        .expect("Failed to execute crumbly");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

/// Run crumbly update without arguments
pub fn crumbly_update(workspace: &Path) -> (i32, String, String) {
    crumbly_cmd(workspace, &["update"])
}

/// Run crumbly update with --all flag
pub fn crumbly_update_all(workspace: &Path) -> (i32, String, String) {
    crumbly_cmd(workspace, &["update", "--all"])
}

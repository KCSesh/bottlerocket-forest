//! Integration tests for registry management.
//!
//! These tests create a temporary grove structure and run brdev from within it.
//! Each test cleans up at the start to ensure a fresh environment.
//!
//! These tests are marked with `#[ignore]` because they:
//! - Require Docker to be installed and running
//! - Spawn actual Docker containers (slow, resource-intensive)
//! - Require network access and available ports
//! - Are not suitable for CI environments without Docker
//!
//! Run with: `cargo test --test registry_integration -- --ignored`

use serial_test::serial;
use std::process::Command;
use tempfile::TempDir;

const TEST_PORT: u16 = 5555;
const TEST_GROVE_NAME: &str = "test-grove";

/// Get path to brdev binary (cargo test builds in debug mode)
fn brdev_bin() -> &'static std::path::Path {
    assert_cmd::cargo::cargo_bin!("brdev")
}

/// Creates a temporary grove directory structure for testing
fn create_test_grove() -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let grove_dir = temp.path().join(TEST_GROVE_NAME);
    let dot_grove = grove_dir.join(".grove");
    std::fs::create_dir_all(&dot_grove).expect("Failed to create .grove dir");
    std::fs::write(dot_grove.join("registry-port"), TEST_PORT.to_string())
        .expect("Failed to write registry-port");
    temp
}

/// Helper to run brdev CLI from within a grove and capture output
fn run_brdev(grove_root: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let grove_dir = grove_root.join(TEST_GROVE_NAME);
    let output = Command::new(brdev_bin())
        .current_dir(&grove_dir)
        .args(args)
        .output()
        .expect("Failed to execute brdev");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

/// Helper to check if docker is available
fn docker_available() -> bool {
    Command::new("docker").arg("--version").output().is_ok()
}

/// Clean up any existing test registry before starting a test
fn clean_test_registry(grove_root: &std::path::Path) {
    let _ = run_brdev(grove_root, &["registry", "clean"]);
}

#[test]
#[ignore]
#[serial(registry)]
fn test_registry_start_idempotent() {
    if !docker_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let grove = create_test_grove();
    clean_test_registry(grove.path());

    let (code1, stdout1, _) = run_brdev(grove.path(), &["registry", "start"]);
    let (code2, stdout2, _) = run_brdev(grove.path(), &["registry", "start"]);

    assert_eq!(code1, 0, "First start should succeed");
    assert_eq!(code2, 0, "Second start should succeed (idempotent)");
    assert!(stdout1.contains(&TEST_PORT.to_string()));
    assert!(stdout2.contains(&TEST_PORT.to_string()));

    clean_test_registry(grove.path());
}

#[test]
#[ignore]
#[serial(registry)]
fn test_registry_status_not_created() {
    if !docker_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let grove = create_test_grove();
    clean_test_registry(grove.path());

    let (code, stdout, _) = run_brdev(grove.path(), &["registry", "status"]);

    assert_eq!(code, 0, "Status should return 0 even when not running");
    assert!(
        stdout.contains("not created") || stdout.contains("not running") || stdout.contains("Not"),
        "Should indicate registry is not created"
    );
}

#[test]
#[ignore]
#[serial(registry)]
fn test_registry_status_running() {
    if !docker_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let grove = create_test_grove();
    clean_test_registry(grove.path());
    let _ = run_brdev(grove.path(), &["registry", "start"]);

    let (code, stdout, _) = run_brdev(grove.path(), &["registry", "status"]);

    assert_eq!(code, 0, "Status should return 0 when running");
    assert!(stdout.contains("running") || stdout.contains("Running"));
    assert!(stdout.contains(&TEST_PORT.to_string()));

    clean_test_registry(grove.path());
}

#[test]
#[ignore]
#[serial(registry)]
fn test_registry_stop() {
    if !docker_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let grove = create_test_grove();
    clean_test_registry(grove.path());
    let _ = run_brdev(grove.path(), &["registry", "start"]);

    let (code, _, _) = run_brdev(grove.path(), &["registry", "stop"]);

    assert_eq!(code, 0, "Stop should succeed");

    let (status_code, _, _) = run_brdev(grove.path(), &["registry", "status"]);
    assert_eq!(status_code, 0, "Status should return 0 even when stopped");
}

#[test]
#[ignore]
#[serial(registry)]
fn test_registry_stop_idempotent() {
    if !docker_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let grove = create_test_grove();
    clean_test_registry(grove.path());

    let (code, _, _) = run_brdev(grove.path(), &["registry", "stop"]);

    assert_eq!(code, 0, "Stop should succeed even when not running");
}

#[test]
#[ignore]
#[serial(registry)]
fn test_registry_clean() {
    if !docker_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let grove = create_test_grove();
    clean_test_registry(grove.path());
    let _ = run_brdev(grove.path(), &["registry", "start"]);

    let (code, _, _) = run_brdev(grove.path(), &["registry", "clean"]);

    assert_eq!(code, 0, "Clean should succeed");

    let (status_code, _, _) = run_brdev(grove.path(), &["registry", "status"]);
    assert_eq!(status_code, 0, "Status should return 0 even after clean");
}

#[test]
#[ignore]
#[serial(registry)]
fn test_registry_logs() {
    if !docker_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let grove = create_test_grove();
    clean_test_registry(grove.path());
    let _ = run_brdev(grove.path(), &["registry", "start"]);

    let (code, stdout, _) = run_brdev(grove.path(), &["registry", "logs"]);

    assert_eq!(code, 0, "Logs should succeed");
    assert!(!stdout.is_empty(), "Should output logs");

    clean_test_registry(grove.path());
}

#[test]
#[ignore]
#[serial(registry)]
fn test_registry_port_from_grove_file() {
    if !docker_available() {
        eprintln!("Skipping test: Docker not available");
        return;
    }

    let grove = create_test_grove();
    clean_test_registry(grove.path());

    let (_, stdout, _) = run_brdev(grove.path(), &["registry", "start"]);

    assert!(
        stdout.contains(&TEST_PORT.to_string()),
        "Should use port from .grove/registry-port"
    );

    clean_test_registry(grove.path());
}

//! Integration tests for twoliter configuration management.

use std::process::Command;
use std::sync::atomic::{AtomicU16, Ordering};
use tempfile::TempDir;

static PORT_COUNTER: AtomicU16 = AtomicU16::new(9000);

fn random_port() -> u16 {
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst) % 2000 + 9000
}

fn grove_name(port: u16) -> String {
    format!("test-grove-{}", port)
}

fn brdev_bin() -> &'static std::path::Path {
    assert_cmd::cargo::cargo_bin!("brdev")
}

fn create_test_grove() -> (TempDir, u16, String) {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let port = random_port();
    let name = grove_name(port);
    let grove_dir = temp.path().join(&name);
    let dot_grove = grove_dir.join(".grove");
    std::fs::create_dir_all(&dot_grove).expect("Failed to create .grove dir");
    std::fs::write(dot_grove.join("registry-port"), port.to_string())
        .expect("Failed to write registry-port");
    (temp, port, name)
}

fn run_brdev(grove_root: &std::path::Path, name: &str, args: &[&str]) -> (i32, String, String) {
    let grove_dir = grove_root.join(name);
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

#[test]
#[ignore]
fn test_twoliter_use_local_creates_files() {
    let (grove, port, name) = create_test_grove();
    let grove_dir = grove.path().join(&name);

    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-local"]);

    assert_eq!(code, 0, "use-local should succeed");
    assert!(stdout.contains("Twoliter.override"));
    assert!(stdout.contains("Infra.toml"));

    let override_path = grove_dir.join("Twoliter.override");
    let infra_path = grove_dir.join("Infra.toml");

    assert!(override_path.exists(), "Twoliter.override should exist");
    assert!(infra_path.exists(), "Infra.toml should exist");

    let override_content = std::fs::read_to_string(&override_path).unwrap();
    let infra_content = std::fs::read_to_string(&infra_path).unwrap();

    assert!(override_content.contains(&format!("localhost:{}", port)));
    assert!(infra_content.contains(&format!("localhost:{}", port)));
}

#[test]
#[ignore]
fn test_twoliter_use_upstream_removes_files() {
    let (grove, _port, name) = create_test_grove();
    let grove_dir = grove.path().join(&name);

    let _ = run_brdev(grove.path(), &name, &["twoliter", "use-local"]);

    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-upstream"]);

    assert_eq!(code, 0, "use-upstream should succeed");
    assert!(stdout.contains("Removed"));

    assert!(!grove_dir.join("Twoliter.override").exists());
    assert!(!grove_dir.join("Infra.toml").exists());
}

#[test]
#[ignore]
fn test_twoliter_use_upstream_idempotent() {
    let (grove, _port, name) = create_test_grove();

    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-upstream"]);

    assert_eq!(code, 0, "use-upstream should succeed even with no files");
    assert!(stdout.contains("No local configuration"));
}

#[test]
#[ignore]
fn test_twoliter_use_local_overwrites() {
    let (grove, port, name) = create_test_grove();
    let grove_dir = grove.path().join(&name);

    let (code1, _, _) = run_brdev(grove.path(), &name, &["twoliter", "use-local"]);
    let (code2, _, _) = run_brdev(grove.path(), &name, &["twoliter", "use-local"]);

    assert_eq!(code1, 0, "First use-local should succeed");
    assert_eq!(code2, 0, "Second use-local should succeed (idempotent)");

    let override_content = std::fs::read_to_string(grove_dir.join("Twoliter.override")).unwrap();
    assert!(override_content.contains(&format!("localhost:{}", port)));
}

#[test]
#[ignore]
fn test_twoliter_use_local_deps_creates_only_override() {
    let (grove, port, name) = create_test_grove();
    let grove_dir = grove.path().join(&name);

    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-local-deps"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Twoliter.override"));
    assert!(grove_dir.join("Twoliter.override").exists());
    assert!(!grove_dir.join("Infra.toml").exists());

    let content = std::fs::read_to_string(grove_dir.join("Twoliter.override")).unwrap();
    assert!(content.contains(&format!("localhost:{}", port)));
}

#[test]
#[ignore]
fn test_twoliter_use_local_publish_creates_only_infra() {
    let (grove, port, name) = create_test_grove();
    let grove_dir = grove.path().join(&name);

    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-local-publish"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Infra.toml"));
    assert!(!grove_dir.join("Twoliter.override").exists());
    assert!(grove_dir.join("Infra.toml").exists());

    let content = std::fs::read_to_string(grove_dir.join("Infra.toml")).unwrap();
    assert!(content.contains(&format!("localhost:{}", port)));
}

#[test]
#[ignore]
fn test_twoliter_use_upstream_deps_removes_only_override() {
    let (grove, _port, name) = create_test_grove();
    let grove_dir = grove.path().join(&name);

    let _ = run_brdev(grove.path(), &name, &["twoliter", "use-local"]);
    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-upstream-deps"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Removed Twoliter.override"));
    assert!(!grove_dir.join("Twoliter.override").exists());
    assert!(grove_dir.join("Infra.toml").exists());
}

#[test]
#[ignore]
fn test_twoliter_use_upstream_publish_removes_only_infra() {
    let (grove, _port, name) = create_test_grove();
    let grove_dir = grove.path().join(&name);

    let _ = run_brdev(grove.path(), &name, &["twoliter", "use-local"]);
    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-upstream-publish"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Removed Infra.toml"));
    assert!(grove_dir.join("Twoliter.override").exists());
    assert!(!grove_dir.join("Infra.toml").exists());
}

#[test]
#[ignore]
fn test_twoliter_use_upstream_deps_idempotent() {
    let (grove, _port, name) = create_test_grove();

    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-upstream-deps"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("not found"));
}

#[test]
#[ignore]
fn test_twoliter_use_upstream_publish_idempotent() {
    let (grove, _port, name) = create_test_grove();

    let (code, stdout, _) = run_brdev(grove.path(), &name, &["twoliter", "use-upstream-publish"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("not found"));
}

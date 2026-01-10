//! Tests for basic scanning functionality

#[cfg(test)]
mod test {
    use crate::knowledge::domain::{FileType, IndexRelativePath, RepoName};
    use crate::knowledge::indexing::scanner::{FileScanner, IndexableFile};
    use std::fs;
    use tempfile::TempDir;

    use super::super::internals::test_helpers::*;

    #[test]
    fn test_scan_finds_markdown_files() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "bottlerocket",
            &[("README.md", "# Test"), ("DESIGN.md", "# Design")],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.file_type == FileType::Markdown));
    }

    #[test]
    fn test_scan_finds_rust_files() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "twoliter",
            &[
                ("src/main.rs", "fn main() {}"),
                ("src/lib.rs", "pub fn test() {}"),
            ],
        );
        let scanner = scanner_no_git(temp_dir.path());
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.file_type == FileType::Rust));
    }

    #[test]
    fn test_scan_ignores_unsupported_files() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "bottlerocket",
            &[
                ("README.md", "# Test"),
                ("Cargo.toml", "[package]"),
                ("LICENSE", "MIT"),
                ("data.json", "{}"),
            ],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_type, FileType::Markdown);
    }

    #[test]
    fn test_scan_extracts_repo_name() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(temp_dir.path(), "bottlerocket", &[("README.md", "# Test")]);
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].repo_name,
            RepoName::try_new("bottlerocket").unwrap()
        );
    }

    #[test]
    fn test_scan_captures_last_modified() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        let file_path = repo_dir.join("test.md");
        fs::write(&file_path, "content").unwrap();
        let metadata = fs::metadata(&file_path).unwrap();
        let expected_mtime = metadata
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].last_modified.as_secs(), expected_mtime);
    }

    #[test]
    fn test_scan_repo_filters_by_repo() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(temp_dir.path(), "bottlerocket", &[("README.md", "# BR")]);
        setup_repo_with_files(temp_dir.path(), "twoliter", &[("README.md", "# TL")]);
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let repo = RepoName::try_new("bottlerocket").unwrap();
        let files = scanner.scan_repo(&repo).unwrap();
        assert_eq!(files.len(), 1);
        assert!(
            files
                .iter()
                .all(|f| f.repo_name == RepoName::try_new("bottlerocket").unwrap())
        );
    }

    #[test]
    fn test_scan_handles_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "bottlerocket",
            &[
                ("docs/README.md", "# Docs"),
                ("docs/architecture/boot.md", "# Boot"),
            ],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.file_type == FileType::Markdown));
    }

    #[test]
    fn test_scan_preserves_relative_paths() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "bottlerocket",
            &[("docs/guide.md", "# Guide")],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].relative_path,
            IndexRelativePath::try_new("bottlerocket/docs/guide.md").unwrap()
        );
    }

    #[test]
    fn test_always_ignores_crumbly_directory() {
        let temp_dir = TempDir::new().unwrap();
        let crumbly_dir = temp_dir.path().join(".crumbly");
        fs::create_dir(&crumbly_dir).unwrap();
        fs::write(crumbly_dir.join("index.db"), "data").unwrap();
        fs::write(crumbly_dir.join("notes.md"), "# Notes").unwrap();
        let files = scan_forest(temp_dir.path());
        assert!(files.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn test_scanner_does_not_follow_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let outside_dir = TempDir::new().unwrap();
        fs::write(outside_dir.path().join("external.md"), "# External").unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("internal.md"), "# Internal").unwrap();
        std::os::unix::fs::symlink(
            outside_dir.path().join("external.md"),
            repo_dir.join("link.md"),
        )
        .unwrap();
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("internal.md"));
        assert!(
            !files
                .iter()
                .any(|f| f.relative_path.to_string().contains("link.md"))
        );
        assert!(
            !files
                .iter()
                .any(|f| f.relative_path.to_string().contains("external.md"))
        );
    }
}

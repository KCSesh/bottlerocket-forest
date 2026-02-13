//! Directory traversal for file scanning

use super::{FileScanner, IndexableFile, ScanError};
use crate::knowledge::constants::{SEMBLY_DIR, SEMBLY_IGNORE};
use crate::knowledge::domain::RepoName;
use snafu::ResultExt;
use std::path::Path;

impl FileScanner {
    pub(super) fn scan_internal(
        &self,
        filter_repo: Option<&RepoName>,
    ) -> Result<Vec<IndexableFile>, ScanError> {
        if self.config.targets.is_empty() {
            return self.scan_from_root(filter_repo);
        }

        let mut all_files = Vec::new();
        for target in &self.config.targets {
            let target_path = self.index_root.join(target);
            if !target_path.exists() {
                continue;
            }
            let files = self.scan_target(&target_path, filter_repo)?;
            all_files.extend(files);
        }
        Ok(all_files)
    }

    pub(super) fn scan_from_root(
        &self,
        filter_repo: Option<&RepoName>,
    ) -> Result<Vec<IndexableFile>, ScanError> {
        self.scan_with_builder(&self.index_root, filter_repo)
    }

    pub(super) fn scan_target(
        &self,
        target_path: &Path,
        filter_repo: Option<&RepoName>,
    ) -> Result<Vec<IndexableFile>, ScanError> {
        self.scan_with_builder(target_path, filter_repo)
    }

    fn scan_with_builder(
        &self,
        root: &Path,
        filter_repo: Option<&RepoName>,
    ) -> Result<Vec<IndexableFile>, ScanError> {
        use super::types::scan_error::*;

        let mut files = Vec::new();

        let mut builder = ignore::WalkBuilder::new(root);
        builder
            .follow_links(false)
            .git_ignore(self.config.respect_gitignore)
            .filter_entry(|entry| {
                let file_name = entry.file_name().to_string_lossy();
                file_name != SEMBLY_DIR
            });

        if self.config.use_crumblyignore {
            builder.add_custom_ignore_filename(SEMBLY_IGNORE);
        }

        for result in builder.build() {
            let entry = result.context(WalkSnafu)?;
            let Some(file) = self.process_entry(entry, filter_repo)? else {
                continue;
            };
            files.push(file);
        }

        Ok(files)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::ScanConfig;
    use crate::knowledge::domain::{FileType, IndexRelativePath};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn scanner_no_git(path: impl AsRef<Path>) -> FileScanner {
        let config = ScanConfig::builder()
            .respect_gitignore(false)
            .use_crumblyignore(false)
            .build();
        FileScanner::with_config(path, config).unwrap()
    }

    fn setup_repo_with_files(root: &Path, repo: &str, files: &[(&str, &str)]) {
        let repo_dir = root.join(repo);
        for (path, content) in files {
            let file_path = repo_dir.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, content).unwrap();
        }
    }

    fn setup_git_repo(root: &Path, repo: &str, gitignore: &str) {
        let repo_dir = root.join(repo);
        fs::create_dir_all(&repo_dir).unwrap();
        fs::create_dir(repo_dir.join(".git")).unwrap();
        fs::write(repo_dir.join(".gitignore"), gitignore).unwrap();
    }

    fn scan_forest(path: &Path) -> Vec<IndexableFile> {
        FileScanner::new(path).unwrap().scan().unwrap()
    }

    #[test]
    fn test_scanner_rejects_nonexistent_root() {
        let nonexistent = Path::new("/nonexistent/path/to/forest");
        let result = FileScanner::new(nonexistent);
        assert!(matches!(result, Err(ScanError::IndexRootNotFound { .. })));
    }

    #[test]
    fn test_scanner_rejects_file_as_root() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_directory.txt");
        fs::write(&file_path, "content").unwrap();
        let result = FileScanner::new(&file_path);
        assert!(matches!(
            result,
            Err(ScanError::IndexRootNotDirectory { .. })
        ));
    }

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
        assert!(
            files
                .iter()
                .all(|f| f.file_type == FileType::new("markdown"))
        );
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
        assert!(
            files
                .iter()
                .all(|f| f.file_type == FileType::new("rust_doc"))
        );
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
        assert_eq!(files[0].file_type, FileType::new("markdown"));
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
        assert!(
            files
                .iter()
                .all(|f| f.file_type == FileType::new("markdown"))
        );
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
    fn test_scan_config_default_values() {
        let config = ScanConfig::default();
        assert!(config.respect_gitignore);
        assert!(config.use_crumblyignore);
        assert!(config.targets.is_empty());
    }

    #[test]
    fn test_scanner_with_custom_config() {
        let config = ScanConfig::builder()
            .respect_gitignore(false)
            .use_crumblyignore(false)
            .build();
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(temp_dir.path(), "repo", &[("test.md", "# Test")]);
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_gitignore_respected_by_default() {
        let temp_dir = TempDir::new().unwrap();
        setup_git_repo(temp_dir.path(), "repo", "ignored.md\n");
        setup_repo_with_files(
            temp_dir.path(),
            "repo",
            &[("included.md", "# Included"), ("ignored.md", "# Ignored")],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("included.md"));
    }

    #[test]
    fn test_gitignore_can_be_disabled() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "repo",
            &[
                (".gitignore", "ignored.md\n"),
                ("included.md", "# Included"),
                ("ignored.md", "# Ignored"),
            ],
        );
        let config = ScanConfig::builder().respect_gitignore(false).build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_scan_with_empty_targets_uses_default_behavior() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(temp_dir.path(), "repo", &[("test.md", "# Test")]);
        let config = ScanConfig::builder().targets(vec![]).build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_scan_with_single_target() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(temp_dir.path(), "docs", &[("guide.md", "# Guide")]);
        setup_repo_with_files(temp_dir.path(), "bottlerocket", &[("README.md", "# BR")]);
        let config = ScanConfig::builder()
            .targets(vec![PathBuf::from("docs")])
            .build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("docs"));
    }

    #[test]
    fn test_scan_with_multiple_targets() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(temp_dir.path(), "docs", &[("guide.md", "# Guide")]);
        setup_repo_with_files(temp_dir.path(), "skills", &[("skill.md", "# Skill")]);
        setup_repo_with_files(temp_dir.path(), "bottlerocket", &[("README.md", "# BR")]);
        let config = ScanConfig::builder()
            .targets(vec![PathBuf::from("docs"), PathBuf::from("skills")])
            .build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .any(|f| f.relative_path.to_string().contains("docs"))
        );
        assert!(
            files
                .iter()
                .any(|f| f.relative_path.to_string().contains("skills"))
        );
        assert!(
            !files
                .iter()
                .any(|f| f.relative_path.to_string().contains("bottlerocket"))
        );
    }

    #[test]
    fn test_scan_with_nonexistent_target() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(temp_dir.path(), "docs", &[("guide.md", "# Guide")]);
        let config = ScanConfig::builder()
            .targets(vec![PathBuf::from("docs"), PathBuf::from("nonexistent")])
            .build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("docs"));
    }

    #[test]
    fn test_scan_with_overlapping_targets() {
        let temp_dir = TempDir::new().unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "repo",
            &[("top.md", "# Top"), ("subdir/nested.md", "# Nested")],
        );
        let config = ScanConfig::builder()
            .targets(vec![PathBuf::from("repo"), PathBuf::from("repo/subdir")])
            .build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert!(files.len() >= 2);
    }

    #[test]
    fn test_scan_respects_target_specific_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        setup_git_repo(temp_dir.path(), "repo1", "ignored.md\n");
        setup_repo_with_files(
            temp_dir.path(),
            "repo1",
            &[("included.md", "# Included"), ("ignored.md", "# Ignored")],
        );
        setup_git_repo(temp_dir.path(), "repo2", "secret.md\n");
        setup_repo_with_files(
            temp_dir.path(),
            "repo2",
            &[("public.md", "# Public"), ("secret.md", "# Secret")],
        );
        let config = ScanConfig::builder()
            .respect_gitignore(true)
            .targets(vec![PathBuf::from("repo1"), PathBuf::from("repo2")])
            .build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .any(|f| f.relative_path.to_string().contains("included.md"))
        );
        assert!(
            files
                .iter()
                .any(|f| f.relative_path.to_string().contains("public.md"))
        );
        assert!(
            !files
                .iter()
                .any(|f| f.relative_path.to_string().contains("ignored.md"))
        );
        assert!(
            !files
                .iter()
                .any(|f| f.relative_path.to_string().contains("secret.md"))
        );
    }

    #[test]
    fn test_crumblyignore_excludes_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "excluded/\n").unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "repo",
            &[
                ("excluded/doc.md", "# Excluded"),
                ("included/doc.md", "# Included"),
            ],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("included"));
    }

    #[test]
    fn test_crumblyignore_can_be_disabled() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "excluded/\n").unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "repo",
            &[("excluded/doc.md", "# Excluded")],
        );
        let config = ScanConfig::builder().use_crumblyignore(false).build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_crumblyignore_with_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join(".git")).unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "*/vendor/\n").unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "repo",
            &[("vendor/doc.md", "# Vendor"), ("docs/doc.md", "# Docs")],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("docs"));
    }

    #[test]
    fn test_crumblyignore_wildcard_patterns() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "*.tmp.md\n").unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "repo",
            &[("keep.md", "# Keep"), ("ignore.tmp.md", "# Ignore")],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("keep.md"));
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

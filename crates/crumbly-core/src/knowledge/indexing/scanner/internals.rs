//! Internal scanning implementation

use super::{FileScanner, IndexableFile, ScanError};
use crate::knowledge::constants::{SEMBLY_DIR, SEMBLY_IGNORE};
use crate::knowledge::domain::{AbsolutePath, FileType, IndexRelativePath, RepoName, Timestamp};
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

    /// Extract repository name from a forest-relative path
    fn extract_repo_name(&self, path: &Path) -> Result<RepoName, ScanError> {
        let relative =
            path.strip_prefix(&self.index_root)
                .map_err(|_| ScanError::InvalidPathStructure {
                    path: path.display().to_string(),
                })?;

        let repo_name_str = relative
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .ok_or_else(|| ScanError::InvalidPathStructure {
                path: path.display().to_string(),
            })?;

        RepoName::try_new(repo_name_str).map_err(|e| ScanError::PathCreation {
            source: Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
        })
    }

    /// Extract last modified timestamp from file metadata
    fn extract_timestamp(&self, path: &Path) -> Result<Timestamp, ScanError> {
        let metadata = std::fs::metadata(path).map_err(|e| ScanError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        let last_modified = metadata
            .modified()
            .map_err(|e| ScanError::IoError {
                path: path.display().to_string(),
                source: e,
            })?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ScanError::IoError {
                path: path.display().to_string(),
                source: std::io::Error::other("invalid modification time"),
            })?
            .as_secs() as i64;

        Ok(Timestamp::from_secs(last_modified))
    }

    /// Create path types from a file path
    fn create_paths(&self, path: &Path) -> Result<(AbsolutePath, IndexRelativePath), ScanError> {
        let absolute_path = AbsolutePath::try_new(path.display().to_string()).map_err(|e| {
            ScanError::PathCreation {
                source: Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
            }
        })?;

        let relative =
            path.strip_prefix(&self.index_root)
                .map_err(|_| ScanError::InvalidPathStructure {
                    path: path.display().to_string(),
                })?;

        let relative_path =
            IndexRelativePath::try_new(relative.display().to_string()).map_err(|e| {
                ScanError::PathCreation {
                    source: Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
                }
            })?;

        Ok((absolute_path, relative_path))
    }

    pub(super) fn process_entry(
        &self,
        entry: ignore::DirEntry,
        filter_repo: Option<&RepoName>,
    ) -> Result<Option<IndexableFile>, ScanError> {
        let path = entry.path();

        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            return Ok(None);
        }

        let file_type = FileType::from_path(path);

        if !self.filter.should_index_file_type(file_type) || !file_type.is_indexable() {
            return Ok(None);
        }

        let repo_name = self.extract_repo_name(path)?;

        if let Some(filter) = filter_repo
            && &repo_name != filter
        {
            return Ok(None);
        }

        let last_modified = self.extract_timestamp(path)?;
        let (absolute_path, relative_path) = self.create_paths(path)?;

        let indexable_file = IndexableFile::builder()
            .absolute_path(absolute_path)
            .relative_path(relative_path)
            .repo_name(repo_name)
            .file_type(file_type)
            .last_modified(last_modified)
            .build();

        if let Some(progress) = &self.progress {
            progress.file_discovered(path);
        }

        Ok(Some(indexable_file))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::domain::ScanConfig;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn scanner_no_git(path: impl AsRef<Path>) -> FileScanner {
        let config = ScanConfig::builder()
            .respect_gitignore(false)
            .use_crumblyignore(false)
            .build();
        FileScanner::with_config(path, config).unwrap()
    }

    #[test]
    fn test_scanner_rejects_nonexistent_root() {
        // Given A nonexistent directory
        let nonexistent = Path::new("/nonexistent/path/to/forest");

        // When Creating a scanner
        let result = FileScanner::new(nonexistent);

        // Then It should fail with IndexRootNotFound
        assert!(matches!(result, Err(ScanError::IndexRootNotFound { .. })));
    }

    #[test]
    fn test_scanner_rejects_file_as_root() {
        // Given A file path instead of directory
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_directory.txt");
        fs::write(&file_path, "content").unwrap();

        // When Creating a scanner with file path
        let result = FileScanner::new(&file_path);

        // Then It should fail with IndexRootNotDirectory
        assert!(matches!(
            result,
            Err(ScanError::IndexRootNotDirectory { .. })
        ));
    }

    #[test]
    fn test_scan_finds_markdown_files() {
        // Given A forest with markdown files
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("bottlerocket");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("README.md"), "# Test").unwrap();
        fs::write(repo_dir.join("DESIGN.md"), "# Design").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then It should find markdown files
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.file_type == FileType::Markdown));
    }

    #[test]
    fn test_scan_finds_rust_files() {
        // Given A forest with rust files
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("twoliter");
        fs::create_dir_all(repo_dir.join("src")).unwrap();
        fs::write(repo_dir.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(repo_dir.join("src/lib.rs"), "pub fn test() {}").unwrap();

        // When Scanning without gitignore
        let scanner = scanner_no_git(temp_dir.path());
        let files = scanner.scan().unwrap();

        // Then It should find rust files
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.file_type == FileType::Rust));
    }

    #[test]
    fn test_scan_ignores_unsupported_files() {
        // Given A forest with mixed file types
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("bottlerocket");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("README.md"), "# Test").unwrap();
        fs::write(repo_dir.join("Cargo.toml"), "[package]").unwrap();
        fs::write(repo_dir.join("LICENSE"), "MIT").unwrap();
        fs::write(repo_dir.join("data.json"), "{}").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then It should only return .md and .rs files
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_type, FileType::Markdown);
    }

    #[test]
    fn test_scan_extracts_repo_name() {
        // Given A forest with repo structure
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("bottlerocket");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("README.md"), "# Test").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then Repo name should be extracted correctly
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].repo_name,
            RepoName::try_new("bottlerocket").unwrap()
        );
    }

    #[test]
    fn test_scan_captures_last_modified() {
        // Given A file with known modification time
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

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then Last modified should match file metadata
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].last_modified.as_secs(), expected_mtime);
    }

    #[test]
    fn test_scan_repo_filters_by_repo() {
        // Given A forest with multiple repos
        let temp_dir = TempDir::new().unwrap();
        let bottlerocket_dir = temp_dir.path().join("bottlerocket");
        let twoliter_dir = temp_dir.path().join("twoliter");
        fs::create_dir(&bottlerocket_dir).unwrap();
        fs::create_dir(&twoliter_dir).unwrap();
        fs::write(bottlerocket_dir.join("README.md"), "# BR").unwrap();
        fs::write(twoliter_dir.join("README.md"), "# TL").unwrap();

        // When Scanning specific repo
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let repo = RepoName::try_new("bottlerocket").unwrap();
        let files = scanner.scan_repo(&repo).unwrap();

        // Then Only files from that repo should be returned
        assert_eq!(files.len(), 1);
        assert!(
            files
                .iter()
                .all(|f| f.repo_name == RepoName::try_new("bottlerocket").unwrap())
        );
    }

    #[test]
    fn test_scan_handles_nested_directories() {
        // Given A forest with nested directory structure
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("bottlerocket");
        fs::create_dir_all(repo_dir.join("docs/architecture")).unwrap();
        fs::write(repo_dir.join("docs/README.md"), "# Docs").unwrap();
        fs::write(repo_dir.join("docs/architecture/boot.md"), "# Boot").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then It should find files in nested directories
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.file_type == FileType::Markdown));
    }

    #[test]
    fn test_scan_preserves_relative_paths() {
        // Given A forest with files in subdirectories
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("bottlerocket");
        fs::create_dir_all(repo_dir.join("docs")).unwrap();
        fs::write(repo_dir.join("docs/guide.md"), "# Guide").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then Relative path should be preserved
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].relative_path,
            IndexRelativePath::try_new("bottlerocket/docs/guide.md").unwrap()
        );
    }

    #[test]
    fn test_always_ignores_crumbly_directory() {
        // Given A forest with .crumbly directory containing indexable files
        let temp_dir = TempDir::new().unwrap();
        let crumbly_dir = temp_dir.path().join(".crumbly");
        fs::create_dir(&crumbly_dir).unwrap();
        fs::write(crumbly_dir.join("index.db"), "data").unwrap();
        fs::write(crumbly_dir.join("notes.md"), "# Notes").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then .crumbly directory should never be scanned
        assert!(files.is_empty());
    }

    #[test]
    fn test_scan_config_default_values() {
        // Given Default ScanConfig
        let config = ScanConfig::default();

        // Then It should have expected defaults
        assert!(config.respect_gitignore);
        assert!(config.use_crumblyignore);
        assert!(config.targets.is_empty());
    }

    #[test]
    fn test_scanner_with_custom_config() {
        // Given A custom ScanConfig
        let config = ScanConfig::builder()
            .respect_gitignore(false)
            .use_crumblyignore(false)
            .build();

        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("test.md"), "# Test").unwrap();

        // When Creating scanner with custom config
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then Scanner should be created successfully
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_gitignore_respected_by_default() {
        // Given A forest with .gitignore in a git repo
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir(&repo_dir).unwrap();

        // Create .git directory (required for .gitignore to work)
        fs::create_dir(repo_dir.join(".git")).unwrap();

        fs::write(repo_dir.join(".gitignore"), "ignored.md\n").unwrap();
        fs::write(repo_dir.join("included.md"), "# Included").unwrap();
        fs::write(repo_dir.join("ignored.md"), "# Ignored").unwrap();

        // When Scanning with default config
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then Gitignored file should not be found (only included.md)
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("included.md"));
    }

    #[test]
    fn test_gitignore_can_be_disabled() {
        // Given A forest with .gitignore
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join(".gitignore"), "ignored.md\n").unwrap();
        fs::write(repo_dir.join("included.md"), "# Included").unwrap();
        fs::write(repo_dir.join("ignored.md"), "# Ignored").unwrap();

        // When Scanning with gitignore disabled
        let config = ScanConfig::builder().respect_gitignore(false).build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then Both files should be found
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_scan_with_empty_targets_uses_default_behavior() {
        // Given A ScanConfig with empty targets vector
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("test.md"), "# Test").unwrap();

        let config = ScanConfig::builder().targets(vec![]).build();

        // When Scanning
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then It should scan from forest root
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_scan_with_single_target() {
        // Given A forest with multiple directories and config targeting one
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = temp_dir.path().join("docs");
        let bottlerocket_dir = temp_dir.path().join("bottlerocket");
        fs::create_dir(&docs_dir).unwrap();
        fs::create_dir(&bottlerocket_dir).unwrap();
        fs::write(docs_dir.join("guide.md"), "# Guide").unwrap();
        fs::write(bottlerocket_dir.join("README.md"), "# BR").unwrap();

        let config = ScanConfig::builder()
            .targets(vec![PathBuf::from("docs")])
            .build();

        // When Scanning
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then Only files from target directory should be returned
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("docs"));
    }

    #[test]
    fn test_scan_with_multiple_targets() {
        // Given A forest with several directories and config with multiple targets
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = temp_dir.path().join("docs");
        let skills_dir = temp_dir.path().join("skills");
        let bottlerocket_dir = temp_dir.path().join("bottlerocket");
        fs::create_dir(&docs_dir).unwrap();
        fs::create_dir(&skills_dir).unwrap();
        fs::create_dir(&bottlerocket_dir).unwrap();
        fs::write(docs_dir.join("guide.md"), "# Guide").unwrap();
        fs::write(skills_dir.join("skill.md"), "# Skill").unwrap();
        fs::write(bottlerocket_dir.join("README.md"), "# BR").unwrap();

        let config = ScanConfig::builder()
            .targets(vec![PathBuf::from("docs"), PathBuf::from("skills")])
            .build();

        // When Scanning
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then Files from all specified targets should be returned
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
        // Given A ScanConfig with a target that doesn't exist
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = temp_dir.path().join("docs");
        fs::create_dir(&docs_dir).unwrap();
        fs::write(docs_dir.join("guide.md"), "# Guide").unwrap();

        let config = ScanConfig::builder()
            .targets(vec![PathBuf::from("docs"), PathBuf::from("nonexistent")])
            .build();

        // When Scanning
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then It should skip nonexistent target and return files from valid target
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("docs"));
    }

    #[test]
    fn test_scan_with_overlapping_targets() {
        // Given Targets that overlap
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir_all(repo_dir.join("subdir")).unwrap();
        fs::write(repo_dir.join("top.md"), "# Top").unwrap();
        fs::write(repo_dir.join("subdir/nested.md"), "# Nested").unwrap();

        let config = ScanConfig::builder()
            .targets(vec![PathBuf::from("repo"), PathBuf::from("repo/subdir")])
            .build();

        // When Scanning
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then It should handle both targets
        assert!(files.len() >= 2);
    }

    #[test]
    fn test_scan_respects_target_specific_gitignore() {
        // Given Multiple targets each with their own .gitignore
        let temp_dir = TempDir::new().unwrap();
        let repo1_dir = temp_dir.path().join("repo1");
        let repo2_dir = temp_dir.path().join("repo2");
        fs::create_dir(&repo1_dir).unwrap();
        fs::create_dir(&repo2_dir).unwrap();

        // Create .git directories
        fs::create_dir(repo1_dir.join(".git")).unwrap();
        fs::create_dir(repo2_dir.join(".git")).unwrap();

        // repo1 ignores "ignored.md"
        fs::write(repo1_dir.join(".gitignore"), "ignored.md\n").unwrap();
        fs::write(repo1_dir.join("included.md"), "# Included").unwrap();
        fs::write(repo1_dir.join("ignored.md"), "# Ignored").unwrap();

        // repo2 ignores "secret.md"
        fs::write(repo2_dir.join(".gitignore"), "secret.md\n").unwrap();
        fs::write(repo2_dir.join("public.md"), "# Public").unwrap();
        fs::write(repo2_dir.join("secret.md"), "# Secret").unwrap();

        let config = ScanConfig::builder()
            .respect_gitignore(true)
            .targets(vec![PathBuf::from("repo1"), PathBuf::from("repo2")])
            .build();

        // When Scanning
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then Each target should respect its own gitignore
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
        // Given A forest with .crumblyignore
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "excluded/\n").unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir_all(repo_dir.join("excluded")).unwrap();
        fs::create_dir_all(repo_dir.join("included")).unwrap();
        fs::write(repo_dir.join("excluded/doc.md"), "# Excluded").unwrap();
        fs::write(repo_dir.join("included/doc.md"), "# Included").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then Only included file should be found
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("included"));
    }

    #[test]
    fn test_crumblyignore_can_be_disabled() {
        // Given A forest with .crumblyignore
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "excluded/\n").unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir_all(repo_dir.join("excluded")).unwrap();
        fs::write(repo_dir.join("excluded/doc.md"), "# Excluded").unwrap();

        // When Scanning with crumblyignore disabled
        let config = ScanConfig::builder().use_crumblyignore(false).build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();

        // Then File should be found
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_crumblyignore_with_subdirectories() {
        // Given A forest with .crumblyignore excluding a subdirectory
        let temp_dir = TempDir::new().unwrap();

        // Create .git directory to prevent global gitignore interference
        fs::create_dir(temp_dir.path().join(".git")).unwrap();

        fs::write(temp_dir.path().join(".crumblyignore"), "*/vendor/\n").unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir_all(repo_dir.join("vendor")).unwrap();
        fs::create_dir_all(repo_dir.join("docs")).unwrap();
        fs::write(repo_dir.join("vendor/doc.md"), "# Vendor").unwrap();
        fs::write(repo_dir.join("docs/doc.md"), "# Docs").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then Only docs file should be found (vendor excluded)
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("docs"));
    }

    #[test]
    fn test_crumblyignore_wildcard_patterns() {
        // Given A forest with .crumblyignore using wildcards
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "*.tmp.md\n").unwrap();
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("keep.md"), "# Keep").unwrap();
        fs::write(repo_dir.join("ignore.tmp.md"), "# Ignore").unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then Only non-matching file should be found
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("keep.md"));
    }

    #[test]
    #[cfg(unix)]
    fn test_scanner_does_not_follow_symlinks() {
        // Given A forest with a symlink pointing outside
        let temp_dir = TempDir::new().unwrap();
        let outside_dir = TempDir::new().unwrap();

        // Create file outside forest
        fs::write(outside_dir.path().join("external.md"), "# External").unwrap();

        // Create repo with symlink to external file
        let repo_dir = temp_dir.path().join("repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("internal.md"), "# Internal").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(
            outside_dir.path().join("external.md"),
            repo_dir.join("link.md"),
        )
        .unwrap();

        // When Scanning
        let scanner = FileScanner::new(temp_dir.path()).unwrap();
        let files = scanner.scan().unwrap();

        // Then Only internal file should be found (symlink not followed)
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

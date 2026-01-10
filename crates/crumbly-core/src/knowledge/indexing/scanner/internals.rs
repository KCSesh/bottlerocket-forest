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
pub(super) mod test_helpers {
    use super::*;
    use crate::knowledge::domain::ScanConfig;
    use std::fs;
    use std::path::Path;

    pub fn scanner_no_git(path: impl AsRef<Path>) -> FileScanner {
        let config = ScanConfig::builder()
            .respect_gitignore(false)
            .use_crumblyignore(false)
            .build();
        FileScanner::with_config(path, config).unwrap()
    }

    pub fn setup_repo_with_files(root: &Path, repo: &str, files: &[(&str, &str)]) {
        let repo_dir = root.join(repo);
        for (path, content) in files {
            let file_path = repo_dir.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, content).unwrap();
        }
    }

    pub fn setup_git_repo(root: &Path, repo: &str, gitignore: &str) {
        let repo_dir = root.join(repo);
        fs::create_dir_all(&repo_dir).unwrap();
        fs::create_dir(repo_dir.join(".git")).unwrap();
        fs::write(repo_dir.join(".gitignore"), gitignore).unwrap();
    }

    pub fn scan_forest(path: &Path) -> Vec<IndexableFile> {
        FileScanner::new(path).unwrap().scan().unwrap()
    }
}

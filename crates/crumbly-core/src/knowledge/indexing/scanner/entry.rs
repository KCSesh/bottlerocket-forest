//! Entry processing for file scanning

use super::{FileScanner, IndexableFile, ScanError};
use crate::knowledge::domain::{AbsolutePath, FileType, IndexRelativePath, RepoName, Timestamp};
use std::path::Path;

impl FileScanner {
    pub(super) fn process_entry(
        &self,
        entry: ignore::DirEntry,
        filter_repo: Option<&RepoName>,
    ) -> Result<Option<IndexableFile>, ScanError> {
        let path = entry.path();

        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            return Ok(None);
        }

        let Some(file_type) = FileType::from_path(path) else {
            return Ok(None);
        };

        if !self.filter.should_index_file_type(&file_type) {
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
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

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

    fn scan_forest(path: &Path) -> Vec<IndexableFile> {
        FileScanner::new(path).unwrap().scan().unwrap()
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
}

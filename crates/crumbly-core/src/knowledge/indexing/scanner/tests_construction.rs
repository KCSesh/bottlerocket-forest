//! Tests for scanner construction and configuration

#[cfg(test)]
mod test {
    use crate::knowledge::indexing::scanner::{FileScanner, ScanError};
    use crate::knowledge::domain::ScanConfig;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    use super::super::internals::test_helpers::*;

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
        assert!(matches!(result, Err(ScanError::IndexRootNotDirectory { .. })));
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
}

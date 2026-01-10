//! Tests for gitignore and crumblyignore patterns

#[cfg(test)]
mod test {
    use crate::knowledge::indexing::scanner::FileScanner;
    use crate::knowledge::domain::ScanConfig;
    use std::fs;
    use tempfile::TempDir;

    use super::super::internals::test_helpers::*;

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
            &[(".gitignore", "ignored.md\n"), ("included.md", "# Included"), ("ignored.md", "# Ignored")],
        );
        let config = ScanConfig::builder().respect_gitignore(false).build();
        let scanner = FileScanner::with_config(temp_dir.path(), config).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_crumblyignore_excludes_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "excluded/\n").unwrap();
        setup_repo_with_files(
            temp_dir.path(),
            "repo",
            &[("excluded/doc.md", "# Excluded"), ("included/doc.md", "# Included")],
        );
        let files = scan_forest(temp_dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.to_string().contains("included"));
    }

    #[test]
    fn test_crumblyignore_can_be_disabled() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".crumblyignore"), "excluded/\n").unwrap();
        setup_repo_with_files(temp_dir.path(), "repo", &[("excluded/doc.md", "# Excluded")]);
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
}

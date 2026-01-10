//! Tests for target-based scanning

#[cfg(test)]
mod test {
    use crate::knowledge::indexing::scanner::FileScanner;
    use crate::knowledge::domain::ScanConfig;
    use std::path::PathBuf;
    use tempfile::TempDir;

    use super::super::internals::test_helpers::*;

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
        assert!(files.iter().any(|f| f.relative_path.to_string().contains("docs")));
        assert!(files.iter().any(|f| f.relative_path.to_string().contains("skills")));
        assert!(!files.iter().any(|f| f.relative_path.to_string().contains("bottlerocket")));
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
        assert!(files.iter().any(|f| f.relative_path.to_string().contains("included.md")));
        assert!(files.iter().any(|f| f.relative_path.to_string().contains("public.md")));
        assert!(!files.iter().any(|f| f.relative_path.to_string().contains("ignored.md")));
        assert!(!files.iter().any(|f| f.relative_path.to_string().contains("secret.md")));
    }
}

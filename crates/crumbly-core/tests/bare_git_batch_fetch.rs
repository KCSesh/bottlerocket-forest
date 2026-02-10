//! Integration tests for BareGitSource batch_fetch
//!
//! These tests create real bare git repositories to verify batch_fetch behavior.

use crumbly_core::knowledge::domain::{FileType, IndexRelativePath, RepoName};
use crumbly_core::knowledge::indexing::IndexingFilter;
use crumbly_core::knowledge::indexing::source::{
    BareGitSource, ContentEntry, ContentSource, GitBlobRef, GitRev,
};
use std::process::Command;
use tempfile::TempDir;

/// Creates a bare git repo with specified files and returns the temp directory.
fn setup_bare_repo(name: &str, files: &[(&str, &str)]) -> TempDir {
    let temp = TempDir::new().expect("create temp dir");

    // Create a regular repo first, then clone as bare
    let work_dir = temp.path().join("work");
    std::fs::create_dir_all(&work_dir).unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(&work_dir)
        .output()
        .expect("git init");

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&work_dir)
        .output()
        .expect("git config email");

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&work_dir)
        .output()
        .expect("git config name");

    for (path, content) in files {
        let file_path = work_dir.join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
        Command::new("git")
            .args(["add", path])
            .current_dir(&work_dir)
            .output()
            .expect("git add");
    }

    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(&work_dir)
        .output()
        .expect("git commit");

    // Clone as bare
    Command::new("git")
        .args(["clone", "--bare", "work", &format!("{}.git", name)])
        .current_dir(temp.path())
        .output()
        .expect("git clone --bare");

    temp
}

fn make_entry(repo: &str, rev: &str, path: &str) -> ContentEntry<GitBlobRef> {
    ContentEntry::builder()
        .id(GitBlobRef::builder()
            .repo_name(RepoName::try_new(repo).unwrap())
            .rev(GitRev::try_new(rev).unwrap())
            .path(IndexRelativePath::try_new(path).unwrap())
            .build())
        .relative_path(IndexRelativePath::try_new(path).unwrap())
        .repo_name(RepoName::try_new(repo).unwrap())
        .file_type(FileType::Markdown)
        .build()
}

#[test]
fn test_batch_fetch_git_single_file() {
    // Given a bare repo with one file
    let temp = setup_bare_repo("testrepo", &[("doc.md", "# Hello")]);
    let source = BareGitSource::new(
        temp.path(),
        GitRev::try_new("HEAD").unwrap(),
        IndexingFilter::default(),
    );
    let entry = make_entry("testrepo", "HEAD", "doc.md");

    // When batch_fetch is called with that entry
    let results = source.batch_fetch(&[entry]);

    // Then it should return the file content
    assert_eq!(results.len(), 1);
    let (returned_entry, content) = results.into_iter().next().unwrap().expect("should succeed");
    assert_eq!(content, "# Hello");
    assert_eq!(returned_entry.relative_path.to_string(), "doc.md");
}

#[test]
fn test_batch_fetch_git_multiple_files() {
    // Given a bare repo with multiple files
    let temp = setup_bare_repo(
        "testrepo",
        &[
            ("a.md", "content a"),
            ("b.md", "content b"),
            ("sub/c.md", "content c"),
        ],
    );
    let source = BareGitSource::new(
        temp.path(),
        GitRev::try_new("HEAD").unwrap(),
        IndexingFilter::default(),
    );
    let entries = vec![
        make_entry("testrepo", "HEAD", "a.md"),
        make_entry("testrepo", "HEAD", "b.md"),
        make_entry("testrepo", "HEAD", "sub/c.md"),
    ];

    // When batch_fetch is called with all entries
    let results = source.batch_fetch(&entries);

    // Then it should return content for all files
    assert_eq!(results.len(), 3);
    let contents: Vec<_> = results
        .into_iter()
        .map(|r| r.expect("should succeed").1)
        .collect();
    assert_eq!(contents, vec!["content a", "content b", "content c"]);
}

#[test]
fn test_batch_fetch_git_missing_file() {
    // Given a bare repo
    let temp = setup_bare_repo("testrepo", &[("exists.md", "content")]);
    let source = BareGitSource::new(
        temp.path(),
        GitRev::try_new("HEAD").unwrap(),
        IndexingFilter::default(),
    );
    let entry = make_entry("testrepo", "HEAD", "nonexistent.md");

    // When batch_fetch is called for a missing file
    let results = source.batch_fetch(&[entry]);

    // Then it should return a NotFound error
    assert_eq!(results.len(), 1);
    let err = results
        .into_iter()
        .next()
        .unwrap()
        .expect_err("should fail");
    assert!(format!("{}", err).contains("not found"));
}

#[test]
fn test_batch_fetch_git_empty_file() {
    // Given a bare repo with an empty file
    let temp = setup_bare_repo("testrepo", &[("empty.md", "")]);
    let source = BareGitSource::new(
        temp.path(),
        GitRev::try_new("HEAD").unwrap(),
        IndexingFilter::default(),
    );
    let entry = make_entry("testrepo", "HEAD", "empty.md");

    // When batch_fetch is called for the empty file
    let results = source.batch_fetch(&[entry]);

    // Then it should return empty content
    assert_eq!(results.len(), 1);
    let (_, content) = results.into_iter().next().unwrap().expect("should succeed");
    assert_eq!(content, "");
}

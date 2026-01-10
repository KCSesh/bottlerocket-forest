//! Bare git repository content source
//!
//! Provides [`BareGitSource`] for reading content from bare git repositories,
//! enabling indexing of forest documentation without working tree checkouts.

use std::path::{Path, PathBuf};
use std::process::Command;

use bon::Builder;
use nutype::nutype;
use snafu::{ResultExt, Snafu};

use crate::knowledge::domain::{FileType, IndexRelativePath, RepoName};
use crate::knowledge::indexing::filter::IndexingFilter;

use super::{ContentEntry, ContentSource};

/// Git revision reference (branch, tag, or commit SHA).
#[nutype(
    validate(not_empty),
    derive(Debug, Clone, Display, PartialEq, Eq, AsRef)
)]
pub struct GitRev(String);

/// Reference to a blob in a bare git repository.
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct GitBlobRef {
    pub repo_name: RepoName,
    pub rev: GitRev,
    pub path: IndexRelativePath,
}

/// Content source for bare git repositories.
#[derive(Debug)]
pub struct BareGitSource {
    bare_repos_dir: PathBuf,
    rev: GitRev,
    filter: IndexingFilter,
}

impl BareGitSource {
    pub fn new(bare_repos_dir: impl Into<PathBuf>, rev: GitRev, filter: IndexingFilter) -> Self {
        Self {
            bare_repos_dir: bare_repos_dir.into(),
            rev,
            filter,
        }
    }

    fn discover_repos(&self) -> Result<Vec<(RepoName, PathBuf)>, BareGitSourceError> {
        use bare_git_source_error::*;

        let entries = std::fs::read_dir(&self.bare_repos_dir).context(ReadDirSnafu {
            path: self.bare_repos_dir.clone(),
        })?;

        let mut repos = Vec::new();
        for entry in entries {
            let entry = entry.context(ReadDirSnafu {
                path: self.bare_repos_dir.clone(),
            })?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".git") {
                        let repo_name = name.trim_end_matches(".git");
                        if let Ok(rn) = RepoName::try_new(repo_name) {
                            repos.push((rn, path));
                        }
                    }
                }
            }
        }
        Ok(repos)
    }

    fn list_files(
        &self,
        git_dir: &Path,
        repo_name: &RepoName,
    ) -> Result<Vec<ContentEntry<GitBlobRef>>, BareGitSourceError> {
        use bare_git_source_error::*;

        let output = Command::new("git")
            .args(["--git-dir", &git_dir.to_string_lossy()])
            .args(["ls-tree", "-r", "--name-only", self.rev.as_ref()])
            .output()
            .context(GitCommandSnafu)?;

        if !output.status.success() {
            return Err(BareGitSourceError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();

        for line in stdout.lines() {
            let file_type = match Path::new(line).extension().and_then(|e| e.to_str()) {
                Some("md") => FileType::Markdown,
                Some("rs") => FileType::Rust,
                Some("go") => FileType::Go,
                _ => continue,
            };

            if !self.filter.should_index_file_type(file_type) {
                continue;
            }

            let Ok(rel_path) = IndexRelativePath::try_new(line) else {
                continue;
            };

            let blob_ref = GitBlobRef::builder()
                .repo_name(repo_name.clone())
                .rev(self.rev.clone())
                .path(rel_path.clone())
                .build();

            entries.push(
                ContentEntry::builder()
                    .id(blob_ref)
                    .relative_path(rel_path)
                    .repo_name(repo_name.clone())
                    .file_type(file_type)
                    .build(),
            );
        }

        Ok(entries)
    }
}

impl ContentSource for BareGitSource {
    type Error = BareGitSourceError;
    type EntryId = GitBlobRef;

    fn scan(&self) -> Result<Vec<ContentEntry<Self::EntryId>>, Self::Error> {
        let repos = self.discover_repos()?;
        let mut all_entries = Vec::new();

        for (repo_name, git_dir) in repos {
            match self.list_files(&git_dir, &repo_name) {
                Ok(entries) => all_entries.extend(entries),
                Err(_) => continue,
            }
        }

        Ok(all_entries)
    }

    fn fetch(&self, entry: &ContentEntry<Self::EntryId>) -> Result<String, Self::Error> {
        use bare_git_source_error::*;

        let git_dir = self
            .bare_repos_dir
            .join(format!("{}.git", entry.repo_name));

        let rev_path = format!("{}:{}", entry.id.rev, entry.id.path);
        let output = Command::new("git")
            .args(["--git-dir", &git_dir.to_string_lossy()])
            .args(["show", &rev_path])
            .output()
            .context(GitCommandSnafu)?;

        if !output.status.success() {
            return Err(BareGitSourceError::GitFailed {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum BareGitSourceError {
    #[snafu(display("Failed to read directory '{}'", path.display()))]
    ReadDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("Failed to execute git command"))]
    GitCommand { source: std::io::Error },

    #[snafu(display("Git command failed: {stderr}"))]
    GitFailed { stderr: String },
}

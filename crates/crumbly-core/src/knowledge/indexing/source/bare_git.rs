//! Bare git repository content source
//!
//! Provides [`BareGitSource`] for reading content from bare git repositories,
//! enabling indexing of forest documentation without working tree checkouts.

use crate::knowledge::domain::{FileType, IndexRelativePath, RepoName};
use crate::knowledge::indexing::filter::IndexingFilter;
use bon::Builder;
use nutype::nutype;
use snafu::{ResultExt, Snafu};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{ContentEntry, ContentSource, FetchError, FetchResult};

/// Indexed entries grouped by repository for batch processing.
type IndexedEntryGroup<'a> = (&'a RepoName, Vec<(usize, &'a ContentEntry<GitBlobRef>)>);

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
    /// Name of the repository.
    repo_name: RepoName,
    /// Git revision (branch, tag, or SHA).
    rev: GitRev,
    /// Path to the file within the repository.
    path: IndexRelativePath,
}

impl GitBlobRef {
    /// Returns the repository name.
    pub fn repo_name(&self) -> &RepoName {
        &self.repo_name
    }
    /// Returns the git revision.
    pub fn rev(&self) -> &GitRev {
        &self.rev
    }
    /// Returns the file path within the repository.
    pub fn path(&self) -> &IndexRelativePath {
        &self.path
    }
}

/// Content source for bare git repositories.
#[derive(Debug)]
pub struct BareGitSource {
    bare_repos_dir: PathBuf,
    rev: GitRev,
    filter: IndexingFilter,
}

impl BareGitSource {
    /// Creates a new bare git source.
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
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && let Some(repo_name) = name.strip_suffix(".git")
                && let Ok(rn) = RepoName::try_new(repo_name)
            {
                repos.push((rn, path));
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
            let Some(file_type) = FileType::from_path(Path::new(line)) else {
                continue;
            };

            if !self.filter.should_index_file_type(&file_type) {
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

    fn batch_fetch_for_repo(
        &self,
        git_dir: &Path,
        entries: &[&ContentEntry<GitBlobRef>],
    ) -> Vec<FetchResult<GitBlobRef>> {
        if entries.is_empty() {
            return Vec::new();
        }

        let mut child = match Command::new("git")
            .args(["--git-dir", &git_dir.to_string_lossy()])
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                return entries
                    .iter()
                    .map(|e| {
                        Err(FetchError::Io {
                            path: PathBuf::from(e.relative_path.to_string()),
                            source: std::io::Error::other("failed to spawn git cat-file"),
                        })
                    })
                    .collect();
            }
        };

        let Some(mut stdin) = child.stdin.take() else {
            let _ = child.kill();
            return entries
                .iter()
                .map(|e| {
                    Err(FetchError::Io {
                        path: PathBuf::from(e.relative_path.to_string()),
                        source: std::io::Error::other("stdin not available"),
                    })
                })
                .collect();
        };
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            return entries
                .iter()
                .map(|e| {
                    Err(FetchError::Io {
                        path: PathBuf::from(e.relative_path.to_string()),
                        source: std::io::Error::other("stdout not available"),
                    })
                })
                .collect();
        };

        // Write all object refs to stdin
        for entry in entries {
            let obj_ref = format!("{}:{}", entry.id.rev(), entry.id.path());
            let _ = writeln!(stdin, "{}", obj_ref);
        }
        drop(stdin);

        // Parse batch output
        let mut reader = BufReader::new(stdout);
        let mut results = Vec::with_capacity(entries.len());

        for entry in entries {
            let path = PathBuf::from(entry.relative_path.to_string());
            let result =
                parse_cat_file_entry(&mut reader, &path).map(|content| ((*entry).clone(), content));
            results.push(result);
        }

        let _ = child.wait();
        results
    }
}

/// Parses a single entry from git cat-file --batch output.
fn parse_cat_file_entry(
    reader: &mut BufReader<impl Read>,
    path: &Path,
) -> Result<String, FetchError> {
    let mut header = String::new();
    if reader.read_line(&mut header).is_err() || header.is_empty() {
        return Err(FetchError::NotFound { path: path.into() });
    }

    // Header format: "<sha> <type> <size>" or "<ref> missing"
    let header = header.trim();
    if header.ends_with("missing") {
        return Err(FetchError::NotFound { path: path.into() });
    }

    let size: usize = header
        .rsplit_once(' ')
        .and_then(|(_, s)| s.parse().ok())
        .ok_or_else(|| FetchError::NotFound { path: path.into() })?;

    // Read exactly `size` bytes of content
    let mut content = vec![0u8; size];
    if reader.read_exact(&mut content).is_err() {
        return Err(FetchError::NotFound { path: path.into() });
    }

    // Consume trailing newline
    let mut newline = [0u8; 1];
    let _ = reader.read_exact(&mut newline);

    String::from_utf8(content).map_err(|_| FetchError::InvalidUtf8 { path: path.into() })
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

        let git_dir = self.bare_repos_dir.join(format!("{}.git", entry.repo_name));

        let rev_path = format!("{}:{}", entry.id.rev(), entry.id.path());
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

    fn batch_fetch(
        &self,
        entries: &[ContentEntry<Self::EntryId>],
    ) -> Vec<FetchResult<Self::EntryId>> {
        // Group entries by repository, tracking original indices
        let mut groups: Vec<IndexedEntryGroup<'_>> = Vec::new();
        for (idx, entry) in entries.iter().enumerate() {
            if let Some((_, group)) = groups
                .iter_mut()
                .find(|(name, _)| *name == &entry.repo_name)
            {
                group.push((idx, entry));
            } else {
                groups.push((&entry.repo_name, vec![(idx, entry)]));
            }
        }

        // Process each repo and collect results with their original indices
        let mut indexed_results: Vec<(usize, FetchResult<GitBlobRef>)> =
            Vec::with_capacity(entries.len());
        for (repo_name, repo_entries) in groups {
            let git_dir = self.bare_repos_dir.join(format!("{}.git", repo_name));
            let just_entries: Vec<&ContentEntry<GitBlobRef>> =
                repo_entries.iter().map(|(_, e)| *e).collect();
            let results = self.batch_fetch_for_repo(&git_dir, &just_entries);
            for ((idx, _), result) in repo_entries.into_iter().zip(results) {
                indexed_results.push((idx, result));
            }
        }

        // Reorder results to match input order
        indexed_results.sort_by_key(|(idx, _)| *idx);
        indexed_results.into_iter().map(|(_, r)| r).collect()
    }
}

/// Errors from bare git repository operations.
#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum BareGitSourceError {
    /// Failed to read a directory.
    #[snafu(display("Failed to read directory '{}'", path.display()))]
    ReadDir {
        /// Directory path.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to execute a git command.
    #[snafu(display("Failed to execute git command"))]
    GitCommand {
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Git command returned non-zero exit status.
    #[snafu(display("Git command failed: {stderr}"))]
    GitFailed {
        /// Error output from git.
        stderr: String,
    },
}

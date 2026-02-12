//! Bare git repository operations.

use std::path::{Path, PathBuf};

use crate::git::command::{GitCommand, GitCommandError};

/// A bare git repository.
#[derive(Debug, Clone)]
pub struct BareRepository {
    path: PathBuf,
    name: String,
}

impl BareRepository {
    /// Creates a new BareRepository reference.
    pub fn new(path: impl Into<PathBuf>, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
        }
    }

    /// Clones a remote repository as a bare repository.
    pub fn clone_from(remote: &str, path: &Path, quiet: bool) -> Result<Self, GitCommandError> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo")
            .to_string();

        GitCommand::new("clone")
            .args(["--bare", remote])
            .arg(path.to_string_lossy())
            .quiet(quiet)
            .run()?;

        Ok(Self {
            path: path.to_path_buf(),
            name,
        })
    }

    /// Returns true if this bare repository exists.
    pub fn exists(&self) -> bool {
        self.path.join("HEAD").exists()
    }

    /// Returns the path to the bare repository.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the repository name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Clones this bare repository to a target directory.
    pub fn clone_to(&self, target: &Path, quiet: bool) -> Result<(), GitCommandError> {
        GitCommand::new("clone")
            .arg(self.path.to_string_lossy())
            .arg(target.to_string_lossy())
            .quiet(quiet)
            .run()
    }

    /// Checks out a branch in a cloned repository.
    pub fn checkout_branch(
        target: &Path,
        branch: &str,
        quiet: bool,
    ) -> Result<(), GitCommandError> {
        GitCommand::new("checkout")
            .arg(branch)
            .cwd(target)
            .quiet(quiet)
            .run()
    }

    /// Creates and checks out a new branch in a cloned repository.
    pub fn checkout_new_branch(
        target: &Path,
        new_branch: &str,
        start_point: &str,
        quiet: bool,
    ) -> Result<(), GitCommandError> {
        GitCommand::new("checkout")
            .args(["-b", new_branch, start_point])
            .cwd(target)
            .quiet(quiet)
            .run()
    }

    /// Fetches from a remote into this bare repository.
    pub fn fetch(&self, remote: &str, quiet: bool) -> Result<(), GitCommandError> {
        GitCommand::new("fetch")
            .args([remote, "+refs/heads/*:refs/heads/*", "--prune"])
            .cwd(&self.path)
            .quiet(quiet)
            .run()
    }
}

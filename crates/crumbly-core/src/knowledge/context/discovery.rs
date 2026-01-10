//! Workspace discovery for crumbly.
//!
//! Discovers the workspace root by walking up the directory tree
//! looking for `.crumbly/knowledge.db`.

use miette::Diagnostic;
use snafu::Snafu;
use std::path::{Path, PathBuf};

use crate::knowledge::constants::{KNOWLEDGE_DB, SEMBLY_DIR};

/// A crumbly workspace containing the knowledge database.
///
/// The workspace root is the directory containing `.crumbly/knowledge.db`.
/// All contexts within a workspace share configuration and embeddings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Creates a new workspace with the given root directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Returns the workspace root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the path to the knowledge database.
    pub fn database_path(&self) -> PathBuf {
        self.root.join(SEMBLY_DIR).join(KNOWLEDGE_DB)
    }
}

/// Discovers the workspace containing the given directory.
///
/// Walks up the directory tree from `cwd` looking for `.crumbly/knowledge.db`.
/// Returns the workspace root or an error if no workspace is found.
pub fn discover_workspace(cwd: &Path) -> Result<Workspace, DiscoveryError> {
    use discovery_error::*;

    for ancestor in cwd.ancestors() {
        let db_path = ancestor.join(SEMBLY_DIR).join(KNOWLEDGE_DB);
        if db_path.exists() {
            return Ok(Workspace::new(ancestor.to_path_buf()));
        }
    }
    WorkspaceNotFoundSnafu.fail()
}

/// Errors that can occur during workspace discovery.
#[derive(Debug, Snafu, Diagnostic)]
#[snafu(module)]
#[non_exhaustive]
pub enum DiscoveryError {
    /// No workspace found in the directory tree.
    #[snafu(display("No crumbly workspace found"))]
    #[diagnostic(
        code(crumbly::workspace::not_found),
        help(
            "Run `crumbly build` to create an index, or navigate to a directory within an existing workspace"
        )
    )]
    WorkspaceNotFound,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_workspace(root: &Path) {
        let crumbly_dir = root.join(".crumbly");
        fs::create_dir_all(&crumbly_dir).unwrap();
        fs::File::create(crumbly_dir.join("knowledge.db")).unwrap();
    }

    #[test]
    fn finds_workspace_when_db_exists() {
        // Given a directory with .crumbly/knowledge.db
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        // When discovering workspace from that directory
        let result = discover_workspace(temp.path());

        // Then it should find the workspace at that directory
        let workspace = result.unwrap();
        assert_eq!(workspace.root(), temp.path());
    }

    #[test]
    fn finds_workspace_from_nested_subdirectory() {
        // Given a workspace with nested subdirectories
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());
        let nested = temp.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        // When discovering workspace from a nested directory
        let result = discover_workspace(&nested);

        // Then it should find the workspace at the root
        let workspace = result.unwrap();
        assert_eq!(workspace.root(), temp.path());
    }

    #[test]
    fn returns_error_when_no_workspace_found() {
        // Given a directory without .crumbly
        let temp = TempDir::new().unwrap();

        // When discovering workspace from that directory
        let result = discover_workspace(temp.path());

        // Then it should return an error
        assert!(matches!(result, Err(DiscoveryError::WorkspaceNotFound)));
    }
}

//! Context management operations for the knowledge index.

use super::KnowledgeIndex;
use super::types::IndexError;
use snafu::ResultExt;
use std::path::Path;

use crate::knowledge::domain::{Context, ContextId};
use crate::knowledge::storage::ContextRepository;

impl KnowledgeIndex {
    pub fn resolve_context(&self, cwd: impl AsRef<Path>) -> Result<Context, IndexError> {
        use crate::knowledge::context::{Workspace, resolve_context};

        let workspace = Workspace::new(self.index_root.clone());
        let repository = self.repository()?;
        let context_repo = repository.context_repository();

        resolve_context(&workspace, cwd.as_ref(), &context_repo).map_err(|e| match e {
            crate::knowledge::context::ResolutionError::NoMatchingContext {
                available_contexts,
            } => IndexError::ContextNotFound { available_contexts },
            other => IndexError::ContextResolutionFailed { source: other },
        })
    }

    /// Build the index for the first time
    ///
    /// Creates a new index by scanning all files in the forest. Fails if an index already exists.
    pub fn list_contexts(&self) -> Result<Vec<Context>, IndexError> {
        use super::types::index_error::*;

        let repository = self.repository()?;
        let context_repo = repository.context_repository();
        context_repo
            .list_contexts()
            .context(ContextRegistrationFailedSnafu)
    }

    /// Remove a registered context from the workspace
    ///
    /// Removes the context's indexed file records and the context itself.
    /// The default context (`.`) cannot be removed.
    pub fn remove_context(&self, context_id: &ContextId) -> Result<(), IndexError> {
        use super::types::index_error::*;

        snafu::ensure!(context_id.as_str() != ".", CannotRemoveDefaultContextSnafu);

        let repository = self.repository()?;
        let context_repo = repository.context_repository();

        let context = context_repo
            .get_context(context_id)
            .context(ContextRegistrationFailedSnafu)?;

        snafu::ensure!(
            context.is_some(),
            ContextDoesNotExistSnafu {
                context_id: context_id.as_str().to_string()
            }
        );

        context_repo
            .remove_context(context_id)
            .context(ContextRegistrationFailedSnafu)?;

        Ok(())
    }
}

#[cfg(test)]
mod context_tests {
    use super::*;
    use crate::knowledge::EmbeddingModelConfig;
    use crate::knowledge::domain::{Context, ContextId};
    use crate::knowledge::storage::{ContextRepository, SqliteChunkRepository};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_workspace(root: &Path) {
        let crumbly_dir = root.join(".crumbly");
        fs::create_dir_all(&crumbly_dir).unwrap();
        // Create a minimal database with the contexts table
        let db_path = crumbly_dir.join("knowledge.db");
        let config = EmbeddingModelConfig::default();
        let repo = SqliteChunkRepository::open(&db_path, &config).unwrap();
        // Register the default context
        let default_context = Context::builder()
            .context_id(ContextId::from_path(".").unwrap())
            .build();
        repo.context_repository()
            .insert_context(&default_context)
            .unwrap();
    }

    #[test]
    fn discover_finds_workspace_from_root() {
        // Given a workspace with .crumbly/knowledge.db
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        // When discovering from the workspace root
        let result = KnowledgeIndex::discover(temp.path());

        // Then it should find the workspace
        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.index_root(), temp.path());
    }

    #[test]
    fn discover_finds_workspace_from_nested_directory() {
        // Given a workspace with nested subdirectories
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());
        let nested = temp.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        // When discovering from a nested directory
        let result = KnowledgeIndex::discover(&nested);

        // Then it should find the workspace at the root
        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.index_root(), temp.path());
    }

    #[test]
    fn discover_returns_error_when_no_workspace() {
        // Given a directory without .crumbly
        let temp = TempDir::new().unwrap();

        // When discovering from that directory
        let result = KnowledgeIndex::discover(temp.path());

        // Then it should return WorkspaceNotFound error
        assert!(matches!(result, Err(IndexError::WorkspaceNotFound { .. })));
    }

    #[test]
    fn resolve_context_finds_default_context() {
        // Given a workspace with the default "." context
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        // When resolving context from the workspace root
        let result = index.resolve_context(temp.path());

        // Then it should return the default context
        assert!(result.is_ok());
        let context = result.unwrap();
        assert_eq!(context.context_id.as_str(), ".");
    }

    #[test]
    fn resolve_context_returns_error_for_unregistered_subdirectory() {
        // Given a workspace with only the default "." context
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());
        let subdir = temp.path().join("subdir");
        fs::create_dir_all(&subdir).unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        // When resolving context from a subdirectory that's not a registered context
        let result = index.resolve_context(&subdir);

        // Then it should return ContextNotFound error (per MCI-ERR-2)
        // The "." context only matches the workspace root, not subdirectories
        assert!(matches!(result, Err(IndexError::ContextNotFound { .. })));
    }

    #[test]
    fn list_contexts_returns_registered_contexts() {
        // Given a workspace with the default context
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        // When listing contexts
        let result = index.list_contexts();

        // Then it should return the default context
        assert!(result.is_ok());
        let contexts = result.unwrap();
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].context_id.as_str(), ".");
    }

    #[test]
    fn resolve_context_finds_registered_context_from_subdirectory() {
        // Given a workspace with a registered "worktrees/feature-a" context
        let temp = TempDir::new().unwrap();
        let crumbly_dir = temp.path().join(".crumbly");
        fs::create_dir_all(&crumbly_dir).unwrap();
        let db_path = crumbly_dir.join("knowledge.db");
        let config = EmbeddingModelConfig::default();
        let repo = SqliteChunkRepository::open(&db_path, &config).unwrap();

        // Register the worktree context (not the default ".")
        let worktree_context = Context::builder()
            .context_id(ContextId::from_path("worktrees/feature-a").unwrap())
            .build();
        repo.context_repository()
            .insert_context(&worktree_context)
            .unwrap();

        // Create the directory structure
        let worktree_dir = temp.path().join("worktrees").join("feature-a");
        let nested_dir = worktree_dir.join("src").join("lib");
        fs::create_dir_all(&nested_dir).unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        // When resolving context from a nested subdirectory within the worktree
        let result = index.resolve_context(&nested_dir);

        // Then it should return the worktrees/feature-a context
        assert!(result.is_ok());
        let context = result.unwrap();
        assert_eq!(context.context_id.as_str(), "worktrees/feature-a");
    }

    #[test]
    fn remove_context_removes_existing_context() {
        // Given a workspace with a non-default context registered
        let temp = TempDir::new().unwrap();
        let crumbly_dir = temp.path().join(".crumbly");
        fs::create_dir_all(&crumbly_dir).unwrap();
        let db_path = crumbly_dir.join("knowledge.db");
        let config = EmbeddingModelConfig::default();
        let repo = SqliteChunkRepository::open(&db_path, &config).unwrap();

        let default_context = Context::builder()
            .context_id(ContextId::from_path(".").unwrap())
            .build();
        repo.context_repository()
            .insert_context(&default_context)
            .unwrap();

        let feature_context = Context::builder()
            .context_id(ContextId::from_path("worktrees/feature-a").unwrap())
            .build();
        repo.context_repository()
            .insert_context(&feature_context)
            .unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        let context_id = ContextId::from_path("worktrees/feature-a").unwrap();

        // When removing the non-default context
        let result = index.remove_context(&context_id);

        // Then it should succeed and the context should be gone
        assert!(result.is_ok());
        let contexts = index.list_contexts().unwrap();
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].context_id.as_str(), ".");
    }

    #[test]
    fn remove_context_fails_for_default_context() {
        // Given a workspace with the default context
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        let default_context_id = ContextId::from_path(".").unwrap();

        // When attempting to remove the default context
        let result = index.remove_context(&default_context_id);

        // Then it should fail with CannotRemoveDefaultContext error
        assert!(matches!(
            result,
            Err(IndexError::CannotRemoveDefaultContext)
        ));
    }

    #[test]
    fn remove_context_fails_for_nonexistent_context() {
        // Given a workspace with only the default context
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        let nonexistent_id = ContextId::from_path("does-not-exist").unwrap();

        // When attempting to remove a nonexistent context
        let result = index.remove_context(&nonexistent_id);

        // Then it should fail with ContextDoesNotExist error
        assert!(matches!(
            result,
            Err(IndexError::ContextDoesNotExist { .. })
        ));
    }
}

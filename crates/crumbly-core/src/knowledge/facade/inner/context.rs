//! Context management operations for the knowledge index.

use std::path::Path;

use snafu::ResultExt;

use super::KnowledgeIndex;
use crate::knowledge::domain::{Context, ContextId};
use crate::knowledge::facade::types::IndexError;
use crate::knowledge::storage::ContextRepository;

pub(in crate::knowledge::facade) fn resolve_context(
    index: &KnowledgeIndex,
    cwd: impl AsRef<Path>,
) -> Result<Context, IndexError> {
    use crate::knowledge::context::{Workspace, resolve_context as do_resolve};

    let workspace = Workspace::new(index.index_root.clone());
    let repository = super::repository(index)?;
    let context_repo = repository.context_repository();

    do_resolve(&workspace, cwd.as_ref(), &context_repo).map_err(|e| match e {
        crate::knowledge::context::ResolutionError::NoMatchingContext { available_contexts } => {
            IndexError::ContextNotFound { available_contexts }
        }
        other => IndexError::ContextResolutionFailed { source: other },
    })
}

pub(in crate::knowledge::facade) fn list_contexts(
    index: &KnowledgeIndex,
) -> Result<Vec<Context>, IndexError> {
    use crate::knowledge::facade::types::index_error::*;

    let repository = super::repository(index)?;
    let context_repo = repository.context_repository();
    context_repo
        .list_contexts()
        .context(ContextRegistrationFailedSnafu)
}

pub(in crate::knowledge::facade) fn remove_context(
    index: &KnowledgeIndex,
    context_id: &ContextId,
) -> Result<(), IndexError> {
    use crate::knowledge::facade::types::index_error::*;

    snafu::ensure!(context_id.as_str() != ".", CannotRemoveDefaultContextSnafu);

    let repository = super::repository(index)?;
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::EmbeddingModelConfig;
    use crate::knowledge::KnowledgeIndex;
    use crate::knowledge::storage::sqlite::SqliteChunkRepository;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_workspace(root: &Path) {
        let crumbly_dir = root.join(".crumbly");
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
    }

    #[test]
    fn discover_finds_workspace_from_root() {
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let result = KnowledgeIndex::discover(temp.path());

        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.index_root(), temp.path());
    }

    #[test]
    fn discover_finds_workspace_from_nested_directory() {
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());
        let nested = temp.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        let result = KnowledgeIndex::discover(&nested);

        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.index_root(), temp.path());
    }

    #[test]
    fn discover_returns_error_when_no_workspace() {
        let temp = TempDir::new().unwrap();

        let result = KnowledgeIndex::discover(temp.path());

        assert!(matches!(result, Err(IndexError::WorkspaceNotFound { .. })));
    }

    #[test]
    fn test_resolve_context_finds_default_context() {
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        let result = resolve_context(&index, temp.path());

        assert!(result.is_ok());
        let context = result.unwrap();
        assert_eq!(context.context_id.as_str(), ".");
    }

    #[test]
    fn test_resolve_context_matches_subdirectory_to_root_context() {
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());
        let subdir = temp.path().join("subdir");
        fs::create_dir_all(&subdir).unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        let result = resolve_context(&index, &subdir);

        assert!(result.is_ok());
        let context = result.unwrap();
        assert_eq!(context.context_id.as_str(), ".");
    }

    #[test]
    fn test_list_contexts_returns_registered_contexts() {
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        let result = list_contexts(&index);

        assert!(result.is_ok());
        let contexts = result.unwrap();
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].context_id.as_str(), ".");
    }

    #[test]
    fn test_resolve_context_finds_registered_context_from_subdirectory() {
        let temp = TempDir::new().unwrap();
        let crumbly_dir = temp.path().join(".crumbly");
        fs::create_dir_all(&crumbly_dir).unwrap();
        let db_path = crumbly_dir.join("knowledge.db");
        let config = EmbeddingModelConfig::default();
        let repo = SqliteChunkRepository::open(&db_path, &config).unwrap();

        let worktree_context = Context::builder()
            .context_id(ContextId::from_path("worktrees/feature-a").unwrap())
            .build();
        repo.context_repository()
            .insert_context(&worktree_context)
            .unwrap();

        let worktree_dir = temp.path().join("worktrees").join("feature-a");
        let nested_dir = worktree_dir.join("src").join("lib");
        fs::create_dir_all(&nested_dir).unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        let result = resolve_context(&index, &nested_dir);

        assert!(result.is_ok());
        let context = result.unwrap();
        assert_eq!(context.context_id.as_str(), "worktrees/feature-a");
    }

    #[test]
    fn test_remove_context_removes_existing_context() {
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

        let result = remove_context(&index, &context_id);

        assert!(result.is_ok());
        let contexts = list_contexts(&index).unwrap();
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].context_id.as_str(), ".");
    }

    #[test]
    fn test_remove_context_fails_for_default_context() {
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        let default_context_id = ContextId::from_path(".").unwrap();

        let result = remove_context(&index, &default_context_id);

        assert!(matches!(
            result,
            Err(IndexError::CannotRemoveDefaultContext)
        ));
    }

    #[test]
    fn test_remove_context_fails_for_nonexistent_context() {
        let temp = TempDir::new().unwrap();
        create_workspace(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        let nonexistent_id = ContextId::from_path("does-not-exist").unwrap();

        let result = remove_context(&index, &nonexistent_id);

        assert!(matches!(
            result,
            Err(IndexError::ContextDoesNotExist { .. })
        ));
    }
}

//! Garbage collection operations for the knowledge index.

use snafu::ResultExt;

use crate::knowledge::facade::KnowledgeIndex;
use crate::knowledge::facade::types::{GcStats, IndexError, index_error::*};
use crate::knowledge::storage::ChunkRepository;

/// Run garbage collection to remove orphaned chunks.
pub(in crate::knowledge::facade) fn gc(index: &KnowledgeIndex) -> Result<GcStats, IndexError> {
    let mut repository = super::repository(index)?;
    let chunks_deleted = repository
        .delete_orphaned_chunks()
        .context(DatabaseAccessFailedSnafu)?;

    Ok(GcStats::builder()
        .chunks_deleted(chunks_deleted as usize)
        .embeddings_deleted(chunks_deleted as usize)
        .build())
}

#[cfg(test)]
mod gc_tests {
    use crate::knowledge::EmbeddingModelConfig;
    use crate::knowledge::KnowledgeIndex;
    use crate::knowledge::domain::{Context, ContextId};

    use crate::knowledge::storage::{ContextRepository, SqliteChunkRepository};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_workspace_with_default_context(root: &Path) {
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
    fn gc_on_empty_database_returns_zero_deleted() {
        let temp = TempDir::new().unwrap();
        create_workspace_with_default_context(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        let result = index.gc();

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.chunks_deleted, 0);
        assert_eq!(stats.embeddings_deleted, 0);
    }

    #[test]
    fn gc_with_no_orphaned_chunks_returns_zero_deleted() {
        let temp = TempDir::new().unwrap();
        let docs_dir = temp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("test.md"), "# Test\n\nSome content here").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        index.build().call().unwrap();

        let result = index.gc();

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.chunks_deleted, 0);
    }

    #[test]
    fn gc_preserves_chunks_referenced_by_remaining_context() {
        let temp = TempDir::new().unwrap();
        let docs_dir = temp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("shared.md"), "# Shared\n\nShared content").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        index.build().call().unwrap();

        let ctx_b = ContextId::from_path("context-b").unwrap();
        index.update().context_id(ctx_b.clone()).call().unwrap();

        let status_before = index.status().unwrap();
        let chunks_before = status_before.chunk_count;

        index.remove_context(&ctx_b).unwrap();
        let gc_result = index.gc().unwrap();

        assert_eq!(gc_result.chunks_deleted, 0);

        let status_after = index.status().unwrap();
        assert_eq!(status_after.chunk_count, chunks_before);
    }

    #[test]
    fn gc_deletes_orphaned_chunks_after_context_removal() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let worktree_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&worktree_dir).unwrap();

        fs::write(main_dir.join("main.md"), "# Main\n\nMain content").unwrap();
        fs::write(
            worktree_dir.join("feature.md"),
            "# Feature\n\nFeature content",
        )
        .unwrap();

        fs::write(temp.path().join("crumbly.toml"), "targets = [\".\"]").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        let ctx_main = ContextId::from_path("main").unwrap();
        index.build().context_id(ctx_main).call().unwrap();

        let ctx_worktree = ContextId::from_path("worktree").unwrap();
        index
            .update()
            .context_id(ctx_worktree.clone())
            .call()
            .unwrap();

        index.remove_context(&ctx_worktree).unwrap();
        let gc_result = index.gc().unwrap();

        assert!(gc_result.chunks_deleted > 0);
    }

    #[test]
    fn gc_returns_zero_when_no_orphans() {
        let temp = TempDir::new().unwrap();
        let docs_dir = temp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("file1.md"), "# File 1\n\nContent one").unwrap();
        fs::write(docs_dir.join("file2.md"), "# File 2\n\nContent two").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        index.build().call().unwrap();

        let gc_result = index.gc().unwrap();

        assert_eq!(gc_result.chunks_deleted, 0);
    }

    #[test]
    fn clear_removes_orphaned_chunks() {
        let temp = TempDir::new().unwrap();
        let docs_dir = temp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("file1.md"), "# File 1\n\nContent one").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        index.build().call().unwrap();

        let status_before = index.status().unwrap();
        assert!(status_before.chunk_count > 0);

        let default_ctx = ContextId::from_path(".").unwrap();
        index.clear(&default_ctx).unwrap();

        let status_after = index.status().unwrap();
        assert_eq!(status_after.chunk_count, 0);
    }
}

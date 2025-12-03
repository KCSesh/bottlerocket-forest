//! Garbage collection operations for the knowledge index.

use super::KnowledgeIndex;
use super::types::{GcStats, IndexError};
use snafu::ResultExt;

use crate::knowledge::storage::ChunkRepository;

impl KnowledgeIndex {
    /// Run garbage collection to remove orphaned chunks
    ///
    /// Deletes chunks whose file_hash is not referenced by any context's indexed files.
    /// This reclaims space after contexts are removed.
    pub fn gc(&self) -> Result<GcStats, IndexError> {
        use super::types::index_error::*;

        let mut repository = self.repository()?;
        let chunks_deleted = repository
            .delete_orphaned_chunks()
            .context(DatabaseAccessFailedSnafu)?;

        // Each chunk has exactly one embedding (1:1 relationship), so the count is the same
        Ok(GcStats::builder()
            .chunks_deleted(chunks_deleted as usize)
            .embeddings_deleted(chunks_deleted as usize)
            .build())
    }
}

#[cfg(test)]
mod gc_tests {
    use super::*;
    use crate::knowledge::EmbeddingModelConfig;
    use crate::knowledge::domain::{Context, ContextId};
    use crate::knowledge::storage::{ContextRepository, SqliteChunkRepository};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_workspace_with_default_context(root: &Path) {
        let sembly_dir = root.join(".sembly");
        fs::create_dir_all(&sembly_dir).unwrap();
        let db_path = sembly_dir.join("knowledge.db");
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
        // Given a workspace with no indexed content
        let temp = TempDir::new().unwrap();
        create_workspace_with_default_context(temp.path());

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        // When running garbage collection
        let result = index.gc();

        // Then it should return zero deleted chunks
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.chunks_deleted, 0);
        assert_eq!(stats.embeddings_deleted, 0);
    }

    #[test]
    fn gc_with_no_orphaned_chunks_returns_zero_deleted() {
        // Given a workspace with indexed content (all chunks referenced)
        let temp = TempDir::new().unwrap();
        let docs_dir = temp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("test.md"), "# Test\n\nSome content here").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        index.build().call().unwrap();

        // When running garbage collection
        let result = index.gc();

        // Then it should return zero (no orphans)
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.chunks_deleted, 0);
    }

    #[test]
    fn gc_preserves_chunks_referenced_by_remaining_context() {
        // Given a workspace with two contexts sharing the same content
        let temp = TempDir::new().unwrap();
        let docs_dir = temp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("shared.md"), "# Shared\n\nShared content").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        // Build default context
        index.build().call().unwrap();

        // Build second context with same content
        let ctx_b = ContextId::from_path("context-b").unwrap();
        index.update().context_id(ctx_b.clone()).call().unwrap();

        let status_before = index.status().unwrap();
        let chunks_before = status_before.chunk_count;

        // When removing one context
        index.remove_context(&ctx_b).unwrap();

        // And running garbage collection
        let gc_result = index.gc().unwrap();

        // Then shared content should NOT be deleted (still referenced by default context)
        assert_eq!(gc_result.chunks_deleted, 0);

        let status_after = index.status().unwrap();
        assert_eq!(status_after.chunk_count, chunks_before);
    }

    #[test]
    fn gc_deletes_orphaned_chunks_after_context_removal() {
        // Given a workspace with two contexts having different content
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let worktree_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&worktree_dir).unwrap();

        // Different content in each directory
        fs::write(main_dir.join("main.md"), "# Main\n\nMain content").unwrap();
        fs::write(
            worktree_dir.join("feature.md"),
            "# Feature\n\nFeature content",
        )
        .unwrap();

        // Configure targets - "." means scan from context root
        fs::write(temp.path().join(".sembly.toml"), "targets = [\".\"]").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();

        // Build main context - scans "main/." which is the main directory
        let ctx_main = ContextId::from_path("main").unwrap();
        index.build().context_id(ctx_main).call().unwrap();

        // Build worktree context - scans "worktree/." which is the worktree directory
        let ctx_worktree = ContextId::from_path("worktree").unwrap();
        index
            .update()
            .context_id(ctx_worktree.clone())
            .call()
            .unwrap();

        // When removing the worktree context
        index.remove_context(&ctx_worktree).unwrap();

        // And running garbage collection
        let gc_result = index.gc().unwrap();

        // Then orphaned chunks from worktree should be deleted
        assert!(gc_result.chunks_deleted > 0);
    }

    #[test]
    fn gc_returns_zero_when_no_orphans() {
        // Given a workspace with content
        let temp = TempDir::new().unwrap();
        let docs_dir = temp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("file1.md"), "# File 1\n\nContent one").unwrap();
        fs::write(docs_dir.join("file2.md"), "# File 2\n\nContent two").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        index.build().call().unwrap();

        // When running gc on an index with no orphaned chunks
        let gc_result = index.gc().unwrap();

        // Then gc should report 0 chunks deleted (all chunks are referenced)
        assert_eq!(gc_result.chunks_deleted, 0);
    }

    #[test]
    fn clear_removes_orphaned_chunks() {
        // Given a workspace with content
        let temp = TempDir::new().unwrap();
        let docs_dir = temp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("file1.md"), "# File 1\n\nContent one").unwrap();

        let index = KnowledgeIndex::open(temp.path()).unwrap();
        index.build().call().unwrap();

        let status_before = index.status().unwrap();
        assert!(status_before.chunk_count > 0);

        // When clearing the context (which also runs gc)
        let default_ctx = ContextId::from_path(".").unwrap();
        index.clear(&default_ctx).unwrap();

        // Then chunks should be removed (gc ran as part of clear)
        let status_after = index.status().unwrap();
        assert_eq!(status_after.chunk_count, 0);
    }
}

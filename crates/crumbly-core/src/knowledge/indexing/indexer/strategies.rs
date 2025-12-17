use super::*;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

impl<R: ChunkRepository> Indexer<R> {
    /// Build index from scratch
    pub(super) fn build(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;

        // Configure thread pool to prevent CPU saturation
        rayon::ThreadPoolBuilder::new()
            .num_threads(crate::knowledge::constants::MAX_INDEXING_THREADS)
            .build_global()
            .ok(); // Ignore error if already initialized

        let start = Instant::now();

        let files = self.scanner.scan().context(ScanFailedSnafu)?;

        if let Some(progress) = &self.progress {
            progress.chunking_started(files.len());
        }

        let results: Vec<_> = files
            .par_iter()
            .map(|file| {
                let result = operations::chunk_file_gracefully(file, &self.dispatcher);
                if let (Ok(chunks), Some(progress)) = (&result, &self.progress) {
                    progress.file_chunked(Path::new(&file.absolute_path.to_string()), chunks.len());
                }
                (file, result)
            })
            .collect();

        // Count total chunks for progress reporting
        let mut total_chunks = 0;
        for (_, result) in &results {
            if let Ok(chunks) = result {
                total_chunks += chunks.len();
            }
        }

        if let Some(progress) = &self.progress {
            progress.chunking_completed(total_chunks);
            progress.embedding_started(total_chunks);
        }

        let (files_added, files_skipped, chunks_affected) =
            self.index_and_store_chunks(results.into_iter(), &[])?;

        if let Some(progress) = &self.progress {
            progress.embedding_completed();
            progress.indexing_completed();
        }

        Ok(IndexResult::builder()
            .files_processed(files_added)
            .files_added(files_added)
            .files_updated(0)
            .files_removed(0)
            .files_skipped(files_skipped)
            .chunks_affected(chunks_affected)
            .duration(start.elapsed())
            .build())
    }

    /// Clear existing index then build from scratch
    pub(super) fn rebuild(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;

        self.repository.clear().context(StorageFailedSnafu)?;
        self.build()
    }

    /// Update only changed files
    pub(super) fn incremental(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;

        rayon::ThreadPoolBuilder::new()
            .num_threads(crate::knowledge::constants::MAX_INDEXING_THREADS)
            .build_global()
            .ok();

        let start = Instant::now();
        let current_files = self.scanner.scan().context(ScanFailedSnafu)?;
        let indexed_files = self
            .repository
            .get_indexed_files(&self.context_id)
            .context(StorageFailedSnafu)?;
        let current_paths: HashSet<_> = current_files
            .iter()
            .map(|f| f.relative_path.clone())
            .collect();
        let indexed_paths: HashSet<_> = indexed_files.keys().cloned().collect();

        let added: Vec<_> = current_files
            .iter()
            .filter(|f| !indexed_paths.contains(&f.relative_path))
            .collect();
        let modified: Vec<_> = current_files
            .iter()
            .filter(|f| {
                indexed_files
                    .get(&f.relative_path)
                    .map(|&ts| f.last_modified > ts)
                    .unwrap_or(false)
            })
            .collect();
        let deleted: Vec<_> = indexed_paths.difference(&current_paths).cloned().collect();

        for path in &deleted {
            self.repository
                .remove_indexed_file_from_context(path, &self.context_id)
                .context(StorageFailedSnafu)?;
        }

        let files_to_process: Vec<_> = added.iter().chain(modified.iter()).copied().collect();
        if let Some(p) = &self.progress {
            p.chunking_started(files_to_process.len());
        }

        let results: Vec<_> = files_to_process
            .par_iter()
            .map(|file| {
                let result = operations::chunk_file_gracefully(file, &self.dispatcher);
                if let (Ok(chunks), Some(p)) = (&result, &self.progress) {
                    p.file_chunked(Path::new(&file.absolute_path.to_string()), chunks.len());
                }
                (*file, result)
            })
            .collect();

        let total_chunks: usize = results
            .iter()
            .filter_map(|(_, r)| r.as_ref().ok())
            .map(|c| c.len())
            .sum();
        if let Some(p) = &self.progress {
            p.chunking_completed(total_chunks);
            p.embedding_started(total_chunks);
        }

        let modified_paths: Vec<_> = modified.iter().map(|f| &f.relative_path).collect();
        let (files_processed, files_skipped, chunks_affected) =
            self.index_and_store_chunks(results.into_iter(), &modified_paths)?;

        if let Some(p) = &self.progress {
            p.embedding_completed();
            p.indexing_completed();
        }

        Ok(IndexResult::builder()
            .files_processed(files_processed)
            .files_added(added.len())
            .files_updated(modified.len())
            .files_removed(deleted.len())
            .files_skipped(files_skipped)
            .chunks_affected(chunks_affected)
            .duration(start.elapsed())
            .build())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::domain::{ContextId, Embedding, IndexRelativePath, Timestamp};
    use crate::knowledge::indexing::provider::MockIndexDataProvider;
    use crate::knowledge::storage::StorageError;
    use crate::knowledge::storage::repository::MockChunkRepository;
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_new_creates_indexer_with_valid_forest() {
        // Given A valid forest directory and mock provider
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();

        let mock_repo = MockChunkRepository::new();
        let config = EmbeddingModelConfig::default();
        let mock_provider = MockIndexDataProvider::new();

        // When Creating an Indexer
        let context_id = ContextId::from_path(".").unwrap();
        let result = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build();

        // Then It should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_fails_with_invalid_forest() {
        // Given A nonexistent forest directory
        let nonexistent = Path::new("/nonexistent/forest");
        let mock_repo = MockChunkRepository::new();
        let config = EmbeddingModelConfig::default();
        let mock_provider = MockIndexDataProvider::new();

        // When Creating an Indexer
        let context_id = ContextId::from_path(".").unwrap();
        let result = Indexer::builder()
            .index_root(nonexistent)
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build();

        // Then It should fail with ScanFailed error
        assert!(matches!(result, Err(IndexingError::ScanFailed { .. })));
    }

    #[test]
    fn test_build_indexes_markdown_files() {
        // Given A forest with markdown files
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("README.md"), "# Test\nContent here").unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_track_indexed_file()
            .times(1)
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().times(1).returning(|chunks| {
            assert!(!chunks.is_empty());
            Ok(())
        });

        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .returning(|_, _| Ok(vec![Embedding::try_new(vec![0.1; 384]).unwrap()]));

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Building the index
        let result = indexer.index(IndexStrategy::Build);

        // Then It should succeed and report indexed files
        assert!(result.is_ok());
        let index_result = result.unwrap();
        assert_eq!(index_result.files_added, 1);
        assert_eq!(index_result.files_processed, 1);
        assert!(index_result.chunks_affected > 0);
    }

    #[test]
    fn test_build_indexes_rust_files() {
        // Given A forest with rust files
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir_all(repo_dir.join("src")).unwrap();
        fs::write(
            repo_dir.join("src/lib.rs"),
            "/// Documentation\npub fn test() {}",
        )
        .unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_track_indexed_file()
            .times(1)
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().times(1).returning(|chunks| {
            assert!(!chunks.is_empty());
            Ok(())
        });

        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .returning(|_, _| Ok(vec![Embedding::try_new(vec![0.1; 384]).unwrap()]));

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Building the index
        let result = indexer.index(IndexStrategy::Build);

        // Then It should succeed and index the rust file
        assert!(result.is_ok());
        let index_result = result.unwrap();
        assert_eq!(index_result.files_added, 1);
        assert!(index_result.chunks_affected > 0);
    }

    #[test]
    fn test_build_calls_provider_generate() {
        // Given A forest with a markdown file
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("test.md"), "# Test\n\nContent").unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_track_indexed_file()
            .times(1)
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().times(1).returning(|_| Ok(()));

        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .times(1)
            .returning(|_, _| Ok(vec![Embedding::try_new(vec![0.1; 384]).unwrap()]));

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Building the index
        let result = indexer.index(IndexStrategy::Build);

        // Then Provider generate should be called
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_propagates_provider_errors() {
        // Given A provider that fails to generate
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("test.md"), "# Test\n\nContent").unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .returning(|_, _| {
                Err(
                    crate::knowledge::indexing::IndexDataError::EmbeddingFailed {
                        source: Box::new(std::io::Error::other("test error")),
                    },
                )
            });

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Building the index
        let result = indexer.index(IndexStrategy::Build);

        // Then It should fail with IndexDataGenerationFailed error
        assert!(matches!(
            result,
            Err(IndexingError::IndexDataGenerationFailed { .. })
        ));
    }

    #[test]
    fn test_build_propagates_storage_errors() {
        // Given A repository that fails to store
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("test.md"), "# Test\n\nContent").unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_track_indexed_file()
            .times(1)
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().returning(|_| {
            Err(StorageError::InvalidData {
                message: "test error".to_string(),
            })
        });

        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .returning(|_, _| Ok(vec![Embedding::try_new(vec![0.1; 384]).unwrap()]));

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Building the index
        let result = indexer.index(IndexStrategy::Build);

        // Then It should fail with StorageFailed error
        assert!(matches!(result, Err(IndexingError::StorageFailed { .. })));
    }

    #[test]
    fn test_build_handles_empty_forest() {
        // Given An empty forest directory
        let temp_dir = TempDir::new().unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo.expect_save_batch().times(0);

        let config = EmbeddingModelConfig::default();
        let mock_provider = MockIndexDataProvider::new();

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Building the index
        let result = indexer.index(IndexStrategy::Build).unwrap();

        // Then It should succeed with zero files indexed
        assert_eq!(result.files_added, 0);
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_affected, 0);
    }

    #[test]
    fn test_rebuild_clears_then_builds() {
        // Given A forest with files
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("test.md"), "# Test\n\nContent").unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo.expect_clear().times(1).returning(|| Ok(5));
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_track_indexed_file()
            .times(1)
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().times(1).returning(|_| Ok(()));

        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .returning(|_, _| Ok(vec![Embedding::try_new(vec![0.1; 384]).unwrap()]));

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Rebuilding the index
        let result = indexer.index(IndexStrategy::Rebuild);

        // Then It should clear then build
        assert!(result.is_ok());
    }

    #[test]
    fn test_incremental_adds_new_files() {
        // Given A forest with a new file and empty index
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("new.md"), "# New\n\nContent").unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_get_indexed_files()
            .returning(|_| Ok(HashMap::new()));
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_track_indexed_file()
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().times(1).returning(|_| Ok(()));

        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .returning(|_, _| Ok(vec![Embedding::try_new(vec![0.1; 384]).unwrap()]));

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Updating the index
        let result = indexer.index(IndexStrategy::Incremental).unwrap();

        // Then It should report one file added
        assert_eq!(result.files_added, 1);
        assert_eq!(result.files_updated, 0);
        assert_eq!(result.files_removed, 0);
        assert_eq!(result.files_processed, 1);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_incremental_modifies_changed_files() {
        // Given A forest with a modified file
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("modified.md"), "# Modified\n\nNew content").unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo.expect_get_indexed_files().returning(|_| {
            let mut map = HashMap::new();
            map.insert(
                IndexRelativePath::try_new("test-repo/modified.md").unwrap(),
                Timestamp::from_secs(1000),
            );
            Ok(map)
        });
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_remove_indexed_file_from_context()
            .times(1)
            .returning(|_, _| Ok(()));
        mock_repo
            .expect_track_indexed_file()
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().times(1).returning(|_| Ok(()));

        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .returning(|_, _| Ok(vec![Embedding::try_new(vec![0.1; 384]).unwrap()]));

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Updating the index
        let result = indexer.index(IndexStrategy::Incremental).unwrap();

        // Then It should report one file updated
        assert_eq!(result.files_added, 0);
        assert_eq!(result.files_updated, 1);
        assert_eq!(result.files_removed, 0);
        assert_eq!(result.files_processed, 1);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_incremental_removes_deleted_files() {
        // Given An index with a file that no longer exists
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo.expect_get_indexed_files().returning(|_| {
            let mut map = HashMap::new();
            map.insert(
                IndexRelativePath::try_new("test-repo/deleted.md").unwrap(),
                Timestamp::from_secs(1000),
            );
            Ok(map)
        });
        mock_repo
            .expect_remove_indexed_file_from_context()
            .times(1)
            .returning(|_, _| Ok(()));

        let config = EmbeddingModelConfig::default();
        let mock_provider = MockIndexDataProvider::new();

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Updating the index
        let result = indexer.index(IndexStrategy::Incremental).unwrap();

        // Then It should report one file removed
        // Note: chunks_affected is 0 because we only remove file references now,
        // actual chunk cleanup happens via GC
        assert_eq!(result.files_added, 0);
        assert_eq!(result.files_updated, 0);
        assert_eq!(result.files_removed, 1);
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_affected, 0);
    }

    #[test]
    fn test_incremental_handles_mixed_changes() {
        // Given A forest with added, modified, and deleted files
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("new.md"), "# New\n\nContent").unwrap();
        fs::write(repo_dir.join("modified.md"), "# Modified\n\nContent").unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo.expect_get_indexed_files().returning(|_| {
            let mut map = HashMap::new();
            map.insert(
                IndexRelativePath::try_new("test-repo/modified.md").unwrap(),
                Timestamp::from_secs(1000),
            );
            map.insert(
                IndexRelativePath::try_new("test-repo/deleted.md").unwrap(),
                Timestamp::from_secs(1000),
            );
            Ok(map)
        });
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_remove_indexed_file_from_context()
            .times(2)
            .returning(|_, _| Ok(()));
        mock_repo
            .expect_save_batch()
            .times(1..=2)
            .returning(|_| Ok(()));
        mock_repo
            .expect_track_indexed_file()
            .times(2)
            .returning(|_, _, _, _| Ok(()));

        let config = EmbeddingModelConfig::default();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .returning(|_, _| Ok(vec![Embedding::try_new(vec![0.1; 384]).unwrap()]));

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Updating the index
        let result = indexer.index(IndexStrategy::Incremental).unwrap();

        // Then It should report all changes
        assert_eq!(result.files_added, 1);
        assert_eq!(result.files_updated, 1);
        assert_eq!(result.files_removed, 1);
        assert_eq!(result.files_processed, 2);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_incremental_propagates_storage_errors() {
        // Given A repository that fails to get indexed files
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo.expect_get_indexed_files().returning(|_| {
            Err(StorageError::InvalidData {
                message: "test error".to_string(),
            })
        });

        let config = EmbeddingModelConfig::default();
        let mock_provider = MockIndexDataProvider::new();

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Updating the index
        let result = indexer.index(IndexStrategy::Incremental);

        // Then It should fail with StorageFailed error
        assert!(matches!(result, Err(IndexingError::StorageFailed { .. })));
    }

    #[test]
    fn test_incremental_handles_empty_forest() {
        // Given An empty forest with no indexed files
        let temp_dir = TempDir::new().unwrap();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_get_indexed_files()
            .returning(|_| Ok(HashMap::new()));

        let config = EmbeddingModelConfig::default();
        let mock_provider = MockIndexDataProvider::new();

        let context_id = ContextId::from_path(".").unwrap();
        let mut indexer = Indexer::builder()
            .index_root(temp_dir.path())
            .repository(mock_repo)
            .config(&config)
            .provider(Box::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap();

        // When Updating the index
        let result = indexer.index(IndexStrategy::Incremental).unwrap();

        // Then It should report no changes
        assert_eq!(result.files_added, 0);
        assert_eq!(result.files_updated, 0);
        assert_eq!(result.files_removed, 0);
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_affected, 0);
    }
}

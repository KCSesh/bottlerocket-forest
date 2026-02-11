use super::*;
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

impl<R: ChunkRepository> Indexer<R> {
    pub(super) fn build(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;

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

    pub(super) fn rebuild(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;
        self.repository.clear().context(StorageFailedSnafu)?;
        self.build()
    }

    pub(super) fn incremental(&mut self) -> Result<IndexResult, IndexingError> {
        use types::indexing_error::*;

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

use crate::knowledge::indexing::source::ContentSource;

/// Configuration for source-based indexing operations.
pub struct SourceIndexConfig<'a, R> {
    /// Chunking dispatcher for content processing.
    pub dispatcher: &'a ChunkingDispatcher,
    /// Repository for storing indexed chunks.
    pub repository: &'a mut R,
    /// Provider for generating embeddings.
    pub provider: &'a dyn IndexDataProvider,
    /// Context identifier for the index.
    pub context_id: &'a ContextId,
    /// Optional progress reporter.
    pub progress: Option<&'a dyn ProgressReporter>,
    /// Batch processing configuration.
    pub batch_config: &'a BatchConfig,
}

/// Build index from a ContentSource.
#[allow(dead_code)]
pub(crate) fn build_from_source<S, R>(
    _source: &S,
    _config: SourceIndexConfig<'_, R>,
) -> Result<IndexResult, IndexingError>
where
    S: ContentSource,
    R: ChunkRepository,
{
    todo!("build_from_source not yet implemented")
}

/// Incremental update from a ContentSource.
#[allow(dead_code)]
pub(crate) fn incremental_from_source<S, R>(
    _source: &S,
    _config: SourceIndexConfig<'_, R>,
) -> Result<IndexResult, IndexingError>
where
    S: ContentSource,
    R: ChunkRepository,
{
    todo!("incremental_from_source not yet implemented")
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

    fn test_embedding() -> Embedding {
        Embedding::try_new(vec![0.1; 384]).unwrap()
    }

    fn mock_provider_success() -> MockIndexDataProvider {
        let mut p = MockIndexDataProvider::new();
        p.expect_generate_batch_with_progress()
            .returning(|_, _| Ok(vec![test_embedding()]));
        p
    }

    fn setup_mock_repo_for_build() -> MockChunkRepository {
        let mut r = MockChunkRepository::new();
        r.expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        r.expect_track_indexed_file().returning(|_, _, _, _| Ok(()));
        r.expect_save_batch().returning(|_| Ok(()));
        r
    }

    fn setup_mock_repo_for_incremental(
        indexed: HashMap<IndexRelativePath, Timestamp>,
    ) -> MockChunkRepository {
        let mut r = MockChunkRepository::new();
        r.expect_get_indexed_files()
            .returning(move |_| Ok(indexed.clone()));
        r.expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        r.expect_remove_indexed_file_from_context()
            .returning(|_, _| Ok(()));
        r.expect_track_indexed_file().returning(|_, _, _, _| Ok(()));
        r.expect_save_batch().returning(|_| Ok(()));
        r
    }

    fn create_test_indexer(
        temp_dir: &TempDir,
        repo: MockChunkRepository,
        provider: MockIndexDataProvider,
    ) -> Indexer<MockChunkRepository> {
        let config = EmbeddingModelConfig::default();
        let context_id = ContextId::from_path(".").unwrap();
        Indexer::builder()
            .index_root(temp_dir.path())
            .repository(repo)
            .config(&config)
            .provider(Arc::new(provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build()
            .unwrap()
    }

    fn setup_test_file(base: &TempDir, rel_path: &str, content: &str) {
        let full_path = base.path().join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }

    fn setup_test_files(base: &TempDir, files: &[(&str, &str)]) {
        for (path, content) in files {
            setup_test_file(base, path, content);
        }
    }

    #[test]
    fn test_new_creates_indexer_with_valid_forest() {
        // Given A valid forest directory and mock provider
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();

        let mock_repo = MockChunkRepository::new();
        let mock_provider = MockIndexDataProvider::new();

        // When Creating an Indexer
        let _indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

        // Then It should succeed
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
            .provider(Arc::new(mock_provider))
            .scan_config(ScanConfig::default())
            .filter(IndexingFilter::default())
            .context_id(context_id)
            .build();

        // Then It should fail with ScanFailed error
        assert!(matches!(result, Err(IndexingError::ScanFailed { .. })));
    }

    #[test_case::test_case(&[("test-repo/README.md", "# Test\nContent here")]; "markdown")]
    #[test_case::test_case(&[("test-repo/src/lib.rs", "/// Documentation\npub fn test() {}")]; "rust")]
    fn test_build_indexes_files(files: &[(&str, &str)]) {
        // Given A forest with a file
        let temp_dir = TempDir::new().unwrap();
        setup_test_files(&temp_dir, files);

        let mock_repo = setup_mock_repo_for_build();
        let mock_provider = mock_provider_success();
        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

        // When Building the index
        let result = indexer.index(IndexStrategy::Build);

        // Then It should succeed and report indexed files
        assert!(result.is_ok());
        let index_result = result.unwrap();
        assert_eq!(index_result.files_added, files.len());
        assert_eq!(index_result.files_processed, files.len());
        assert!(index_result.chunks_affected > 0);
    }

    #[test]
    fn test_build_calls_provider_generate() {
        // Given A forest with a markdown file
        let temp_dir = TempDir::new().unwrap();
        setup_test_file(&temp_dir, "test-repo/test.md", "# Test\n\nContent");

        let mock_repo = setup_mock_repo_for_build();
        let mut mock_provider = MockIndexDataProvider::new();
        mock_provider
            .expect_generate_batch_with_progress()
            .times(1)
            .returning(|_, _| Ok(vec![test_embedding()]));

        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

        // When Building the index
        let result = indexer.index(IndexStrategy::Build);

        // Then Provider generate should be called
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_propagates_provider_error() {
        // Given A provider that fails
        let temp_dir = TempDir::new().unwrap();
        setup_test_file(&temp_dir, "test-repo/test.md", "# Test\n\nContent");

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

        let mock_repo = setup_mock_repo_for_build();
        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

        // When Building the index
        let result = indexer.index(IndexStrategy::Build);

        // Then It should fail with IndexDataGenerationFailed error
        assert!(matches!(
            result,
            Err(IndexingError::IndexDataGenerationFailed { .. })
        ));
    }

    #[test]
    fn test_build_propagates_storage_error() {
        // Given A repository that fails
        let temp_dir = TempDir::new().unwrap();
        setup_test_file(&temp_dir, "test-repo/test.md", "# Test\n\nContent");

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_track_indexed_file()
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().returning(|_| {
            Err(StorageError::InvalidData {
                message: "test error".to_string(),
            })
        });

        let mock_provider = mock_provider_success();
        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

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
        let mock_provider = MockIndexDataProvider::new();
        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

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
        setup_test_file(&temp_dir, "test-repo/test.md", "# Test\n\nContent");

        let mut mock_repo = MockChunkRepository::new();
        mock_repo.expect_clear().times(1).returning(|| Ok(5));
        mock_repo
            .expect_has_embedding_batch()
            .returning(|_| Ok(HashSet::new()));
        mock_repo
            .expect_track_indexed_file()
            .returning(|_, _, _, _| Ok(()));
        mock_repo.expect_save_batch().returning(|_| Ok(()));

        let mock_provider = mock_provider_success();
        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

        // When Rebuilding the index
        let result = indexer.index(IndexStrategy::Rebuild);

        // Then It should clear then build
        assert!(result.is_ok());
    }

    #[test_case::test_case(&[("test-repo/new.md", "# New\n\nContent")], &[], 1, 0, 0, 1; "adds_new_files")]
    #[test_case::test_case(&[("test-repo/modified.md", "# Modified\n\nNew content")], &["test-repo/modified.md"], 0, 1, 0, 1; "modifies_changed_files")]
    #[test_case::test_case(&[], &["test-repo/deleted.md"], 0, 0, 1, 0; "removes_deleted_files")]
    #[test_case::test_case(&[("test-repo/new.md", "# New\n\nContent"), ("test-repo/modified.md", "# Modified\n\nContent")], &["test-repo/modified.md", "test-repo/deleted.md"], 1, 1, 1, 2; "handles_mixed_changes")]
    fn test_incremental_operations(
        files: &[(&str, &str)],
        indexed_paths: &[&str],
        exp_added: usize,
        exp_updated: usize,
        exp_removed: usize,
        exp_processed: usize,
    ) {
        // Given A forest with specific file changes
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        setup_test_files(&temp_dir, files);

        let mut indexed = HashMap::new();
        for path in indexed_paths {
            indexed.insert(
                IndexRelativePath::try_new(*path).unwrap(),
                Timestamp::from_secs(1000),
            );
        }

        let mock_repo = setup_mock_repo_for_incremental(indexed);
        let mock_provider = mock_provider_success();
        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

        // When Updating the index
        let result = indexer.index(IndexStrategy::Incremental).unwrap();

        // Then It should report expected changes
        assert_eq!(result.files_added, exp_added);
        assert_eq!(result.files_updated, exp_updated);
        assert_eq!(result.files_removed, exp_removed);
        assert_eq!(result.files_processed, exp_processed);
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

        let mock_provider = MockIndexDataProvider::new();
        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

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

        let mock_provider = MockIndexDataProvider::new();
        let mut indexer = create_test_indexer(&temp_dir, mock_repo, mock_provider);

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

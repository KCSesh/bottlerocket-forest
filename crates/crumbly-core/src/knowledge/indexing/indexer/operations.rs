//! Core file processing operations for indexing
//!
//! Provides functions for processing individual files during indexing: reading
//! content, chunking via the dispatcher, generating embeddings, and handling
//! parse errors gracefully.

use snafu::ResultExt;

use super::super::{IndexDataProvider, IndexableFile, ProgressReporter};
use super::types::IndexingError;
use crate::knowledge::chunking::{ChunkingDispatcher, ChunkingError, ChunkingInput, DispatchError};
use crate::knowledge::domain::{
    Chunk, ChunkHash, ChunkSource, ChunkableContent, FileHash, IndexedChunk, Timestamp,
};
use crate::knowledge::storage::ChunkRepository;

fn is_skippable_error(error: &IndexingError) -> bool {
    match error {
        // Skip files with parse errors
        IndexingError::ChunkingFailed {
            source:
                DispatchError::ChunkingFailed {
                    source: ChunkingError::ParseError { .. },
                },
        } => true,
        // Skip files with no matching strategy (shouldn't happen due to scanner filtering)
        IndexingError::ChunkingFailed {
            source: DispatchError::NoStrategyFound,
        } => true,
        // Strategy init failures are fatal - indicate configuration problem
        IndexingError::ChunkingFailed {
            source: DispatchError::StrategyInitFailed { .. },
        } => false,
        // Other chunking errors are fatal
        IndexingError::ChunkingFailed { .. } => false,
        // Skip files with read errors (permission denied, file deleted, etc.)
        IndexingError::ScanFailed { .. } => true,
        // Embedding errors are fatal - indicate systemic problem
        IndexingError::IndexDataGenerationFailed { .. } => false,
        // Storage errors are fatal
        IndexingError::StorageFailed { .. } => false,
    }
}

/// Chunk a file without generating embeddings
///
/// Returns raw chunks that can later be indexed with embedding reuse.
pub(super) fn chunk_file(
    file: &IndexableFile,
    dispatcher: &ChunkingDispatcher,
) -> Result<Vec<Chunk>, IndexingError> {
    use super::types::indexing_error::*;

    let content = std::fs::read_to_string(file.absolute_path.to_string())
        .map_err(|e| super::super::ScanError::IoError {
            source: e,
            path: file.absolute_path.to_string(),
        })
        .context(ScanFailedSnafu)?;

    let file_hash = FileHash::from_reader(std::io::Cursor::new(content.as_bytes()))
        .map_err(|e| super::super::ScanError::IoError {
            source: e,
            path: file.absolute_path.to_string(),
        })
        .context(ScanFailedSnafu)?;

    let input = ChunkingInput {
        content: ChunkableContent::new(content),
        source: ChunkSource::builder()
            .file_path(file.relative_path.clone())
            .repo_name(file.repo_name.clone())
            .build(),
        file_hash,
    };

    match dispatcher.chunk_file(&input) {
        Some(result) => result.context(ChunkingFailedSnafu),
        None => Ok(vec![]),
    }
}

/// Chunk a file with graceful error handling for parse failures
///
/// Returns Ok with chunks on success, Err(Ok(())) if the file should be skipped
/// due to a parse error, or Err(Err(error)) for fatal errors.
pub(super) fn chunk_file_gracefully(
    file: &IndexableFile,
    dispatcher: &ChunkingDispatcher,
) -> Result<Vec<Chunk>, Result<(), IndexingError>> {
    match chunk_file(file, dispatcher) {
        Ok(chunks) => Ok(chunks),
        Err(e) => {
            if is_skippable_error(&e) {
                Err(Ok(()))
            } else {
                Err(Err(e))
            }
        }
    }
}

/// Generate embeddings for chunks and wrap them as IndexedChunks
fn index_chunks(
    chunks: Vec<Chunk>,
    provider: &dyn IndexDataProvider,
    progress: Option<&dyn ProgressReporter>,
) -> Result<Vec<IndexedChunk>, IndexingError> {
    use super::types::indexing_error::*;

    if chunks.is_empty() {
        return Ok(vec![]);
    }

    let texts: Vec<_> = chunks.iter().map(|c| c.content.text.as_ref()).collect();

    let index_data_list = provider
        .generate_batch_with_progress(&texts, progress)
        .context(IndexDataGenerationFailedSnafu)?;

    let timestamp = Timestamp::now();

    Ok(chunks
        .into_iter()
        .zip(index_data_list)
        .map(|(chunk, embedding)| {
            IndexedChunk::builder()
                .chunk(chunk)
                .embedding(embedding)
                .indexed_at(timestamp)
                .build()
        })
        .collect())
}

/// Generates embeddings for chunks, reusing existing embeddings when available.
///
/// Checks the repository for chunks that already have embeddings and skips
/// embedding generation for those chunks. Returns only newly indexed chunks
/// that need to be stored.
pub(super) fn index_chunks_with_reuse<R: ChunkRepository>(
    chunks: Vec<Chunk>,
    repository: &R,
    provider: &dyn IndexDataProvider,
    progress: Option<&dyn ProgressReporter>,
) -> Result<Vec<IndexedChunk>, IndexingError> {
    let (chunks_needing_embeddings, reused_count) =
        filter_chunks_needing_embeddings(chunks, repository)?;

    let _ = (reused_count, chunks_needing_embeddings.len());
    // Logging would show: "Reusing {} existing embeddings, generating {} new"
    // but tracing is not available in crumbly-core dependencies

    index_chunks(chunks_needing_embeddings, provider, progress)
}

/// Filters chunks to find those that need new embeddings.
///
/// Returns the chunks that don't already have embeddings in the repository.
fn filter_chunks_needing_embeddings<R: ChunkRepository>(
    chunks: Vec<Chunk>,
    repository: &R,
) -> Result<(Vec<Chunk>, usize), IndexingError> {
    use super::types::indexing_error::*;

    let chunk_hashes: Vec<ChunkHash> = chunks.iter().map(|c| c.chunk_hash).collect();

    let existing = repository
        .has_embedding_batch(&chunk_hashes)
        .context(StorageFailedSnafu)?;

    let reused_count = existing.len();

    let chunks_needing_embeddings: Vec<Chunk> = chunks
        .into_iter()
        .filter(|c| !existing.contains(&c.chunk_hash))
        .collect();

    Ok((chunks_needing_embeddings, reused_count))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::domain::{
        ChunkContent, ChunkContext, ChunkId, ChunkSource, Embedding, FileHash, IndexRelativePath,
        MarkdownContext, RepoName, TokenCount,
    };
    use crate::knowledge::indexing::ScanError;
    use crate::knowledge::indexing::provider::MockIndexDataProvider;
    use crate::knowledge::storage::StorageError;
    use crate::knowledge::storage::repository::MockChunkRepository;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use test_case::test_case;

    fn make_test_chunk(text: &str) -> Chunk {
        let chunk_hash = ChunkHash::from_text(text);
        let file_hash = FileHash::new([0u8; 32]);

        Chunk::builder()
            .id(ChunkId::new(uuid::Uuid::new_v4()))
            .chunk_hash(chunk_hash)
            .file_hash(file_hash)
            .source(
                ChunkSource::builder()
                    .file_path(IndexRelativePath::try_new("test.md").unwrap())
                    .repo_name(RepoName::try_new("test-repo").unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(text)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(ChunkContext::Markdown(
                MarkdownContext::builder().heading_hierarchy(vec![]).build(),
            ))
            .build()
    }

    fn make_mock_repo_with_hashes(existing: HashSet<ChunkHash>) -> MockChunkRepository {
        let mut mock = MockChunkRepository::new();
        mock.expect_has_embedding_batch().returning(move |hashes| {
            Ok(hashes
                .iter()
                .filter(|h| existing.contains(h))
                .copied()
                .collect())
        });
        mock
    }

    fn make_mock_provider_counting_calls(counter: Arc<AtomicUsize>) -> MockIndexDataProvider {
        let mut mock = MockIndexDataProvider::new();
        mock.expect_generate_batch_with_progress()
            .returning(move |texts, _| {
                counter.fetch_add(texts.len(), Ordering::SeqCst);
                Ok(texts.iter().map(|_| test_embedding()).collect())
            });
        mock
    }

    fn test_embedding() -> Embedding {
        Embedding::try_new(vec![0.1; 384]).unwrap()
    }

    #[test_case(
        IndexingError::ChunkingFailed {
            source: DispatchError::ChunkingFailed {
                source: ChunkingError::ParseError {
                    file_path: "test.rs".to_string(),
                    source: Box::new(std::io::Error::other("parse failed")),
                },
            },
        },
        true ; "parse error is skippable"
    )]
    #[test_case(
        IndexingError::ScanFailed {
            source: ScanError::IoError {
                path: "/test/file.md".to_string(),
                source: std::io::Error::other("permission denied"),
            },
        },
        true ; "scan error is skippable"
    )]
    #[test_case(
        IndexingError::IndexDataGenerationFailed {
            source: crate::knowledge::indexing::IndexDataError::EmbeddingFailed {
                source: Box::new(std::io::Error::other("model failed")),
            },
        },
        false ; "embedding error not skippable"
    )]
    #[test_case(
        IndexingError::StorageFailed {
            source: StorageError::InvalidData {
                message: "corrupted data".to_string(),
            },
        },
        false ; "storage error not skippable"
    )]
    fn test_is_skippable_error(error: IndexingError, expected: bool) {
        // Given An indexing error
        // When Checking if skippable
        let result = is_skippable_error(&error);
        // Then Result matches expectation
        assert_eq!(result, expected);
    }

    #[test_case(vec![], 0, 0 ; "no chunks returns empty")]
    #[test_case(vec!["new1", "new2", "new3"], 3, 0 ; "all new chunks")]
    #[test_case(vec!["existing1", "existing2"], 0, 2 ; "all existing chunks")]
    #[test_case(vec!["existing", "new1", "new2"], 2, 1 ; "mixed chunks")]
    fn test_filter_chunks_needing_embeddings(
        chunk_texts: Vec<&str>,
        expected_new: usize,
        expected_reused: usize,
    ) {
        // Given Chunks with some existing embeddings
        // When Filtering chunks
        // Then Counts match expectations
        let chunks: Vec<_> = chunk_texts.iter().map(|t| make_test_chunk(t)).collect();
        let existing: HashSet<_> = chunks
            .iter()
            .filter(|c| c.content.text.starts_with("existing"))
            .map(|c| c.chunk_hash)
            .collect();
        let mock_repo = make_mock_repo_with_hashes(existing);

        // When Filtering chunks
        let (needs_embedding, reused_count) =
            filter_chunks_needing_embeddings(chunks, &mock_repo).unwrap();

        // Then Counts match expectations
        assert_eq!(needs_embedding.len(), expected_new);
        assert_eq!(reused_count, expected_reused);
    }

    #[test]
    fn index_chunks_with_reuse_generates_embeddings_only_for_new_chunks() {
        // Given Chunks where some already have embeddings
        let new_chunk = make_test_chunk("new content needs embedding");
        let existing_chunk = make_test_chunk("existing content has embedding");
        let existing = [existing_chunk.chunk_hash].into_iter().collect();
        let chunks = vec![new_chunk.clone(), existing_chunk];
        let mock_repo = make_mock_repo_with_hashes(existing);
        let counter = Arc::new(AtomicUsize::new(0));
        let mock_provider = make_mock_provider_counting_calls(counter.clone());

        // When Indexing with reuse
        let result = index_chunks_with_reuse(chunks, &mock_repo, &mock_provider, None).unwrap();

        // Then Embeddings generated only for new chunks
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(!result.is_empty());
    }

    #[test]
    fn index_chunks_with_reuse_does_not_call_provider_when_all_exist() {
        // Given Chunks that all have existing embeddings
        let chunk1 = make_test_chunk("existing one");
        let chunk2 = make_test_chunk("existing two");
        let existing = [chunk1.chunk_hash, chunk2.chunk_hash].into_iter().collect();
        let chunks = vec![chunk1, chunk2];
        let mock_repo = make_mock_repo_with_hashes(existing);
        let counter = Arc::new(AtomicUsize::new(0));
        let mock_provider = make_mock_provider_counting_calls(counter.clone());

        // When Indexing with reuse
        let result = index_chunks_with_reuse(chunks, &mock_repo, &mock_provider, None).unwrap();

        // Then Provider not called
        assert_eq!(counter.load(Ordering::SeqCst), 0);
        assert!(result.is_empty());
    }

    #[test]
    fn index_chunks_with_reuse_returns_newly_indexed_chunks() {
        // Given All new chunks
        let chunk1 = make_test_chunk("brand new one");
        let chunk2 = make_test_chunk("brand new two");
        let chunks = vec![chunk1.clone(), chunk2.clone()];
        let mock_repo = make_mock_repo_with_hashes(HashSet::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let mock_provider = make_mock_provider_counting_calls(counter);

        // When Indexing with reuse
        let result = index_chunks_with_reuse(chunks, &mock_repo, &mock_provider, None).unwrap();

        // Then All chunks returned as newly indexed
        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .any(|ic| ic.chunk.chunk_hash == chunk1.chunk_hash)
        );
        assert!(
            result
                .iter()
                .any(|ic| ic.chunk.chunk_hash == chunk2.chunk_hash)
        );
    }
}

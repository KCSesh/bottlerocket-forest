//! Core file processing operations for indexing
//!
//! Provides functions for processing individual files during indexing: reading
//! content, chunking via the dispatcher, generating embeddings, and handling
//! parse errors gracefully.

use snafu::ResultExt;

use super::super::IndexableFile;
use super::types::IndexingError;
use crate::knowledge::chunking::{ChunkingDispatcher, ChunkingError, ChunkingInput, DispatchError};
use crate::knowledge::domain::{Chunk, ChunkSource, ChunkableContent, FileHash};

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

    match dispatcher.chunk_file(&input, &file.file_peek) {
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

/// Chunk content from a ContentEntry without reading from filesystem.

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::indexing::ScanError;
    use crate::knowledge::storage::StorageError;
    use test_case::test_case;

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
}

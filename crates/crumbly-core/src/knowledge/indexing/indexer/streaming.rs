//! Streaming pipeline helpers for chunk indexing
//!
//! Provides shared state and helper functions for streaming chunks through
//! the embedding pipeline without collecting intermediate results.

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use snafu::ResultExt;

use super::BatchConfig;
use super::operations;
use super::pipeline::{EmbeddingPipeline, PipelineHandle, WorkItem};
use super::types::IndexingError;
use crate::knowledge::chunking::ChunkingDispatcher;
use crate::knowledge::domain::{ChunkHash, ContextId, FileHash, IndexedChunk, Timestamp};
use crate::knowledge::indexing::{IndexDataProvider, IndexableFile, ProgressReporter};
use crate::knowledge::storage::ChunkRepository;

pub(super) type ChunkResult =
    Result<Vec<crate::knowledge::domain::Chunk>, Result<(), IndexingError>>;

/// Shared state for streaming pipeline processing
pub(super) struct StreamContext {
    pub(super) error_flag: AtomicBool,
    pub(super) first_error: Mutex<Option<IndexingError>>,
    pub(super) files_added: AtomicUsize,
    pub(super) files_skipped: AtomicUsize,
}

impl StreamContext {
    pub(super) fn new() -> Self {
        Self {
            error_flag: AtomicBool::new(false),
            first_error: Mutex::new(None),
            files_added: AtomicUsize::new(0),
            files_skipped: AtomicUsize::new(0),
        }
    }

    pub(super) fn set_error(&self, e: IndexingError) {
        let mut guard = self.first_error.lock().unwrap_or_else(|p| p.into_inner());
        if guard.is_none() {
            *guard = Some(e);
        }
        self.error_flag.store(true, Ordering::Relaxed);
    }

    pub(super) fn take_error(&self) -> Option<IndexingError> {
        self.first_error
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
    }

    pub(super) fn has_error(&self) -> bool {
        self.error_flag.load(Ordering::Relaxed)
    }
}

/// Send chunks to pipeline, returning true if an error occurred
fn send_chunks_to_pipeline<F>(
    chunks: Vec<crate::knowledge::domain::Chunk>,
    existing: &HashSet<ChunkHash>,
    handle: &PipelineHandle<F>,
    ctx: &StreamContext,
) -> bool {
    for chunk in chunks
        .into_iter()
        .filter(|c| !existing.contains(&c.chunk_hash))
    {
        if handle.sender().send(WorkItem { chunk }).is_err() {
            if let Some(e) = handle.check_error() {
                ctx.set_error(e);
            }
            return true;
        }
    }
    false
}

/// Stream chunks from files through the embedding pipeline
#[allow(clippy::too_many_arguments)]
pub(super) fn stream_index_files<R: ChunkRepository>(
    files: &[&IndexableFile],
    modified_paths: &[&crate::knowledge::domain::IndexRelativePath],
    dispatcher: &ChunkingDispatcher,
    repository: &mut R,
    provider: &Arc<dyn IndexDataProvider>,
    progress: &Option<Arc<dyn ProgressReporter>>,
    batch_config: &BatchConfig,
    context_id: &ContextId,
) -> Result<(usize, usize, usize), IndexingError> {
    use super::types::indexing_error::*;

    let worker_count = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
    let batch_size = batch_config.batch_size.into_inner();

    let storage_batch: Arc<Mutex<Vec<IndexedChunk>>> =
        Arc::new(Mutex::new(Vec::with_capacity(batch_size)));
    let storage_batch_clone = Arc::clone(&storage_batch);

    let pipeline = EmbeddingPipeline::builder()
        .provider(Arc::clone(provider))
        .worker_count(worker_count)
        .maybe_progress(progress.clone())
        .build();

    let handle = pipeline.start_with_sink(move |chunk: IndexedChunk| {
        if let Ok(mut batch) = storage_batch_clone.lock() {
            batch.push(chunk);
        }
    });

    let ctx = StreamContext::new();

    // Chunk files in parallel
    let chunked_results: Vec<_> = files
        .par_iter()
        .map(|file| {
            let result = operations::chunk_file_gracefully(file, dispatcher);
            if let (Ok(chunks), Some(p)) = (&result, progress) {
                p.file_chunked(Path::new(&file.absolute_path.to_string()), chunks.len());
            }
            (*file, result)
        })
        .collect();

    let chunk_count: usize = chunked_results
        .iter()
        .filter_map(|(_, r)| r.as_ref().ok())
        .map(|c| c.len())
        .sum();

    if let Some(p) = progress {
        p.chunking_completed(chunk_count);
        p.embedding_started(chunk_count);
    }

    // Process results sequentially and send to pipeline
    for (file, result) in chunked_results {
        if ctx.has_error() {
            break;
        }

        ctx.files_added.fetch_add(1, Ordering::Relaxed);

        let is_modified = modified_paths.iter().any(|p| **p == file.relative_path);
        let processed = process_file_result(
            file,
            result,
            is_modified,
            &handle,
            &ctx,
            repository,
            context_id,
        );

        if ctx.has_error() {
            break;
        }

        let Some((chunks, existing)) = processed else {
            continue;
        };

        if chunks.is_empty() {
            continue;
        }

        if send_chunks_to_pipeline(chunks, &existing, &handle, &ctx) {
            break;
        }
    }

    if let Err(e) = handle.join() {
        ctx.set_error(e);
    }

    if let Some(e) = ctx.take_error() {
        return Err(e);
    }

    let mut final_batch = storage_batch.lock().unwrap_or_else(|p| p.into_inner());
    let chunks_affected = final_batch.len();

    if !final_batch.is_empty() {
        for batch in final_batch.chunks(batch_size) {
            repository.save_batch(batch).context(StorageFailedSnafu)?;
        }
    }
    final_batch.clear();

    Ok((
        ctx.files_added.load(Ordering::Relaxed),
        ctx.files_skipped.load(Ordering::Relaxed),
        chunks_affected,
    ))
}

/// Process a single file result, returning chunks to embed or None if skipped/error
#[allow(clippy::too_many_arguments)]
fn process_file_result<F, R: ChunkRepository>(
    file: &IndexableFile,
    result: ChunkResult,
    is_modified: bool,
    handle: &PipelineHandle<F>,
    ctx: &StreamContext,
    repository: &mut R,
    context_id: &ContextId,
) -> Option<(Vec<crate::knowledge::domain::Chunk>, HashSet<ChunkHash>)> {
    use super::types::indexing_error::*;

    let chunks = match result {
        Ok(chunks) => chunks,
        Err(Ok(())) => {
            ctx.files_skipped.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        Err(Err(e)) => {
            ctx.set_error(e);
            return None;
        }
    };

    if is_modified {
        let result = repository
            .remove_indexed_file_from_context(&file.relative_path, context_id)
            .context(StorageFailedSnafu);
        if let Err(e) = result {
            ctx.set_error(e);
            return None;
        }
    }

    if chunks.is_empty() {
        return Some((Vec::new(), HashSet::new()));
    }

    let chunk_hashes: Vec<ChunkHash> = chunks.iter().map(|c| c.chunk_hash).collect();
    let existing = match repository
        .has_embedding_batch(&chunk_hashes)
        .context(StorageFailedSnafu)
    {
        Ok(e) => e,
        Err(e) => {
            ctx.set_error(e);
            return None;
        }
    };

    let file_hash = chunks
        .first()
        .map(|c| c.file_hash)
        .unwrap_or(FileHash::new([0u8; 32]));

    let result = repository
        .track_indexed_file(
            &file.relative_path,
            &file_hash,
            Timestamp::from_secs(file.last_modified.as_secs()),
            context_id,
        )
        .context(StorageFailedSnafu);
    if let Err(e) = result {
        ctx.set_error(e);
        return None;
    }

    if let Some(e) = handle.check_error() {
        ctx.set_error(e);
        return None;
    }

    Some((chunks, existing))
}

//! Streaming pipeline helpers for chunk indexing
//!
//! Provides shared state and helper functions for streaming chunks through
//! the embedding pipeline without collecting intermediate results.

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{Receiver, Sender, bounded};
use rayon::prelude::*;
use snafu::ResultExt;

use super::BatchConfig;
use super::file_tracker::{FileRegistration, FileTracker};
use super::operations;
use super::pipeline::{EmbeddingPipeline, PipelineHandle, WorkItem};
use super::types::IndexingError;
use super::writer::{BatchingSink, CollectorState, WriterMessage, run_writer_thread};
use crate::knowledge::chunking::ChunkingDispatcher;
use crate::knowledge::domain::{ChunkHash, ContextId, FileHash, IndexedChunk, Timestamp};
use crate::knowledge::indexing::{IndexDataProvider, IndexableFile, ProgressReporter};
use crate::knowledge::storage::ChunkRepository;

pub(super) type ChunkResult =
    Result<Vec<crate::knowledge::domain::Chunk>, Result<(), IndexingError>>;

/// Number of files to buffer before parallel has_embedding_batch queries
const CONSUMER_BATCH_SIZE: usize = 8;

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

/// Item sent from rayon workers to consumer
struct ChunkedFile {
    file_idx: usize,
    result: ChunkResult,
}

/// Producer state for parallel chunking
struct ProducerState<'a> {
    files: &'a [&'a IndexableFile],
    dispatcher: &'a ChunkingDispatcher,
    ctx: &'a StreamContext,
    chunk_count: &'a AtomicUsize,
    progress: &'a Option<Arc<dyn ProgressReporter>>,
}

impl ProducerState<'_> {
    fn run(self, tx: Sender<ChunkedFile>) {
        self.files
            .par_iter()
            .enumerate()
            .for_each_with(tx, |tx, (idx, file)| self.process_file(idx, file, tx));
    }

    fn process_file(&self, idx: usize, file: &IndexableFile, tx: &Sender<ChunkedFile>) {
        if self.ctx.has_error() {
            return;
        }

        let result = operations::chunk_file_gracefully(file, self.dispatcher);
        self.report_progress(file, &result);
        let _ = tx.send(ChunkedFile {
            file_idx: idx,
            result,
        });
    }

    fn report_progress(&self, file: &IndexableFile, result: &ChunkResult) {
        let Ok(chunks) = result else { return };
        self.chunk_count.fetch_add(chunks.len(), Ordering::Relaxed);
        if let Some(p) = self.progress {
            p.file_chunked(Path::new(&file.absolute_path.to_string()), chunks.len());
        }
    }
}

/// Prepared file data after chunking, ready for embedding lookup
struct PreparedFile {
    file_idx: usize,
    chunks: Vec<crate::knowledge::domain::Chunk>,
}

/// Consumer state for processing chunked files
struct ConsumerState<'a, F, R: ChunkRepository> {
    files: &'a [&'a IndexableFile],
    modified_paths: &'a [&'a crate::knowledge::domain::IndexRelativePath],
    handle: &'a PipelineHandle<F>,
    ctx: &'a StreamContext,
    repository: &'a mut R,
    context_id: &'a ContextId,
    reg_tx: &'a Sender<FileRegistration>,
}

impl<F, R: ChunkRepository + Send> ConsumerState<'_, F, R> {
    fn run(mut self, rx: Receiver<ChunkedFile>) {
        let mut batch: Vec<ChunkedFile> = Vec::with_capacity(CONSUMER_BATCH_SIZE);

        for chunked in rx {
            if self.ctx.has_error() {
                continue;
            }
            batch.push(chunked);
            if batch.len() >= CONSUMER_BATCH_SIZE {
                self.process_batch(&mut batch);
                batch.clear();
            }
        }

        // Process remaining files
        if !batch.is_empty() && !self.ctx.has_error() {
            self.process_batch(&mut batch);
        }
    }

    fn process_batch(&mut self, batch: &mut Vec<ChunkedFile>) {
        if self.ctx.has_error() {
            return;
        }

        // Phase 1: Prepare files and collect all chunk hashes
        let mut prepared: Vec<PreparedFile> = Vec::with_capacity(batch.len());
        let mut all_hashes: Vec<ChunkHash> = Vec::new();

        for chunked in batch.drain(..) {
            self.ctx.files_added.fetch_add(1, Ordering::Relaxed);
            let file = self.files[chunked.file_idx];
            let is_modified = self
                .modified_paths
                .iter()
                .any(|p| **p == file.relative_path);

            let chunks = match chunked.result {
                Ok(chunks) => chunks,
                Err(Ok(())) => {
                    self.ctx.files_skipped.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                Err(Err(e)) => {
                    self.ctx.set_error(e);
                    return;
                }
            };

            // Handle modified files
            if is_modified && let Err(e) = self.remove_modified_file(file) {
                self.ctx.set_error(e);
                return;
            }

            if chunks.is_empty() {
                // Empty file - track immediately (no chunks flow through pipeline)
                self.track_empty_file(file)
                    .unwrap_or_else(|e| self.ctx.set_error(e));
                continue;
            }

            all_hashes.extend(chunks.iter().map(|c| c.chunk_hash));
            prepared.push(PreparedFile {
                file_idx: chunked.file_idx,
                chunks,
            });
        }

        if prepared.is_empty() {
            return;
        }

        // Phase 2: Parallel has_embedding_batch queries
        let existing = self.query_existing_parallel(&all_hashes);
        let existing = match existing {
            Ok(e) => e,
            Err(e) => {
                self.ctx.set_error(e);
                return;
            }
        };

        // Phase 3: Process each file with pre-fetched existing set
        for prep in prepared {
            if self.ctx.has_error() {
                return;
            }
            self.finalize_file(prep, &existing);
        }
    }

    fn query_existing_parallel(
        &self,
        all_hashes: &[ChunkHash],
    ) -> Result<HashSet<ChunkHash>, IndexingError> {
        if all_hashes.is_empty() {
            return Ok(HashSet::new());
        }

        let num_connections = CONSUMER_BATCH_SIZE.min(4);
        let chunk_size = all_hashes.len().div_ceil(num_connections);
        let hash_chunks: Vec<&[ChunkHash]> = all_hashes.chunks(chunk_size).collect();

        let results = thread::scope(|s| {
            let handles: Vec<_> = hash_chunks
                .into_iter()
                .map(|hashes| {
                    let repo_result = self.repository.spawn();
                    s.spawn(move || query_hashes(repo_result, hashes))
                })
                .collect();
            handles.into_iter().map(|h| h.join()).collect::<Vec<_>>()
        });

        let mut combined = HashSet::new();
        for result in results {
            let inner = result.map_err(|_| IndexingError::ThreadPanic)?;
            combined.extend(inner?);
        }
        Ok(combined)
    }

    fn remove_modified_file(&mut self, file: &IndexableFile) -> Result<(), IndexingError> {
        use super::types::indexing_error::*;
        self.repository
            .remove_indexed_file_from_context(&file.relative_path, self.context_id)
            .context(StorageFailedSnafu)
    }

    fn track_empty_file(&mut self, file: &IndexableFile) -> Result<(), IndexingError> {
        use super::types::indexing_error::*;
        self.repository
            .track_indexed_file(
                &file.relative_path,
                &FileHash::new([0u8; 32]),
                Timestamp::from_secs(file.last_modified.as_secs()),
                self.context_id,
            )
            .context(StorageFailedSnafu)
    }

    fn finalize_file(&mut self, prep: PreparedFile, existing: &HashSet<ChunkHash>) {
        use super::types::indexing_error::*;

        let file = self.files[prep.file_idx];
        let file_hash = prep
            .chunks
            .first()
            .map(|c| c.file_hash)
            .unwrap_or(FileHash::new([0u8; 32]));

        // Count new chunks (not in existing)
        let new_chunks: Vec<_> = prep
            .chunks
            .into_iter()
            .filter(|c| !existing.contains(&c.chunk_hash))
            .collect();
        let new_chunk_count = new_chunks.len();

        let reg = FileRegistration {
            file_path: file.relative_path.clone(),
            file_hash,
            mtime: Timestamp::from_secs(file.last_modified.as_secs()),
            context_id: self.context_id.clone(),
            expected_chunks: new_chunk_count,
        };

        if new_chunk_count == 0 {
            // All chunks cached - track immediately (no chunks flow through pipeline)
            let result = self
                .repository
                .track_indexed_file(&reg.file_path, &reg.file_hash, reg.mtime, &reg.context_id)
                .context(StorageFailedSnafu);
            if let Err(e) = result {
                self.ctx.set_error(e);
            }
            return;
        }

        // Send registration to collector
        if self.reg_tx.send(reg).is_err() {
            if let Some(e) = self.handle.check_error() {
                self.ctx.set_error(e);
            }
            return;
        }

        if let Some(e) = self.handle.check_error() {
            self.ctx.set_error(e);
            return;
        }

        // Send new chunks to pipeline
        send_chunks(new_chunks, self.handle, self.ctx);
    }
}

fn send_chunks<F>(
    chunks: Vec<crate::knowledge::domain::Chunk>,
    handle: &PipelineHandle<F>,
    ctx: &StreamContext,
) {
    for chunk in chunks {
        if handle.sender().send(WorkItem { chunk }).is_err() {
            if let Some(e) = handle.check_error() {
                ctx.set_error(e);
            }
            return;
        }
    }
}

fn query_hashes<R: ChunkRepository>(
    repo_result: Result<R, crate::knowledge::storage::repository::StorageError>,
    hashes: &[ChunkHash],
) -> Result<HashSet<ChunkHash>, IndexingError> {
    use super::types::indexing_error::*;
    let repo = repo_result.context(StorageFailedSnafu)?;
    repo.has_embedding_batch(hashes).context(StorageFailedSnafu)
}

/// Stream chunks from files through the embedding pipeline
#[allow(clippy::too_many_arguments)]
pub(super) fn stream_index_files<R: ChunkRepository + Send + 'static>(
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

    // Spawn repository instances for writer thread
    let sink_repo = repository.spawn().context(StorageFailedSnafu)?;
    let tracker_repo = repository.spawn().context(StorageFailedSnafu)?;

    // Channel for messages from collector to writer
    let (writer_tx, writer_rx) = bounded::<WriterMessage>(worker_count.get() * 2);

    // Spawn writer thread BEFORE pipeline - owns BatchingSink and FileTracker directly
    let sink = BatchingSink::new(sink_repo, batch_size);
    let tracker = FileTracker::new(tracker_repo, batch_size);
    let writer_handle = thread::Builder::new()
        .name("writer".into())
        .spawn(move || run_writer_thread(writer_rx, sink, tracker))
        .map_err(|_| IndexingError::ThreadPanic)?;

    // Channel for file registrations from consumer to collector
    let (reg_tx, reg_rx) = bounded::<FileRegistration>(worker_count.get() * 2);

    let pipeline = EmbeddingPipeline::builder()
        .provider(Arc::clone(provider))
        .worker_count(worker_count)
        .maybe_progress(progress.clone())
        .build();

    // Collector state wrapped in Arc<Mutex<>> so we can access after pipeline.join()
    let collector_state = Arc::new(Mutex::new(CollectorState::new(
        writer_tx, reg_rx, batch_size,
    )));
    let collector_clone = Arc::clone(&collector_state);

    let handle = pipeline.start_with_sink(move |chunk: IndexedChunk| {
        if let Ok(mut state) = collector_clone.lock() {
            state.ingest(chunk);
        }
    });

    let ctx = Arc::new(StreamContext::new());
    let chunk_count = Arc::new(AtomicUsize::new(0));

    // Channel capacity: 2x rayon thread count for backpressure
    let channel_capacity = worker_count.get() * 2;
    let (tx, rx) = bounded::<ChunkedFile>(channel_capacity);

    // Use std::thread::scope to allow borrowing
    thread::scope(|s| {
        // Producer thread: parallel chunking via rayon
        let producer = ProducerState {
            files,
            dispatcher,
            ctx: &ctx,
            chunk_count: &chunk_count,
            progress,
        };
        s.spawn(|| producer.run(tx));

        // Consumer: main scope body (borrows repository)
        let consumer = ConsumerState {
            files,
            modified_paths,
            handle: &handle,
            ctx: &ctx,
            repository,
            context_id,
            reg_tx: &reg_tx,
        };
        consumer.run(rx);
    });

    // Report progress and join pipeline
    let total_chunks = chunk_count.load(Ordering::Relaxed);
    if let Some(p) = progress {
        p.chunking_completed(total_chunks);
        p.embedding_started(total_chunks);
    }

    // Join pipeline - collector thread exits, closure is returned
    let join_result = handle.join();
    let (chunks_affected, _) = match join_result {
        Ok((count, sink)) => (count, sink),
        Err(e) => return Err(e),
    };

    // Flush partial batch and send shutdown to writer
    {
        let mut state = collector_state.lock().unwrap_or_else(|p| p.into_inner());
        state.flush_and_shutdown();
    }

    // Join writer thread and propagate any errors
    let writer_result = writer_handle
        .join()
        .map_err(|_| IndexingError::ThreadPanic)?;
    if let Err(e) = writer_result {
        ctx.set_error(e);
    }

    if let Some(e) = ctx.take_error() {
        return Err(e);
    }

    Ok((
        ctx.files_added.load(Ordering::Relaxed),
        ctx.files_skipped.load(Ordering::Relaxed),
        chunks_affected,
    ))
}

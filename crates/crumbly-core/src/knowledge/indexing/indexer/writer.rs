//! Dedicated writer thread for decoupling collection from SQLite writes
//!
//! Provides a writer thread that owns BatchingSink and FileTracker directly,
//! receiving chunks via bounded channel to prevent SQLite I/O from blocking
//! embedding workers.

use std::collections::HashSet;

use crossbeam_channel::{Receiver, Sender};

use super::file_tracker::{FileRegistration, FileTracker};
use super::types::IndexingError;
use crate::knowledge::domain::{ChunkHash, IndexedChunk};
use crate::knowledge::storage::ChunkRepository;

/// Message sent from collector to writer thread
pub(super) enum WriterMessage {
    ChunkBatch(Vec<IndexedChunk>),
    Registration(FileRegistration),
    Shutdown,
}

/// Batched writer that flushes indexed chunks to storage periodically
pub(super) struct BatchingSink<R: ChunkRepository> {
    repository: R,
    buffer: Vec<IndexedChunk>,
    batch_size: usize,
    error: Option<IndexingError>,
    /// Tracks chunk hashes written in this indexing run for cross-batch deduplication
    written_hashes: HashSet<ChunkHash>,
}

impl<R: ChunkRepository> BatchingSink<R> {
    pub(super) fn new(repository: R, batch_size: usize) -> Self {
        Self {
            repository,
            buffer: Vec::with_capacity(batch_size),
            batch_size,
            error: None,
            written_hashes: HashSet::new(),
        }
    }

    /// Checks if a chunk hash is known (either written this run or exists in DB)
    /// Uses lazy population: queries DB on miss, caches result regardless
    fn is_known(&mut self, chunk_hash: &ChunkHash) -> bool {
        use super::types::indexing_error::*;
        use snafu::ResultExt;

        if self.written_hashes.contains(chunk_hash) {
            return true;
        }
        // Lazy DB lookup - query once per unique hash
        let exists = match self
            .repository
            .has_embedding(chunk_hash)
            .context(StorageFailedSnafu)
        {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(e);
                return true; // Skip chunk on error; error surfaces at flush()
            }
        };
        // Cache regardless of result to avoid repeated queries
        self.written_hashes.insert(*chunk_hash);
        exists
    }

    /// Records all chunk hashes in buffer as written
    fn record_written(&mut self) {
        for c in &self.buffer {
            self.written_hashes.insert(c.chunk.chunk_hash);
        }
    }

    pub(super) fn ingest(&mut self, chunk: IndexedChunk) {
        use super::types::indexing_error::*;
        use snafu::ResultExt;

        if self.error.is_some() {
            return;
        }

        // Skip chunks already written or known to exist in DB
        if self.is_known(&chunk.chunk.chunk_hash) {
            return;
        }

        self.buffer.push(chunk);
        if self.buffer.len() >= self.batch_size {
            let result = self
                .repository
                .save_batch(&self.buffer)
                .context(StorageFailedSnafu);
            match result {
                Ok(()) => self.record_written(),
                Err(e) => self.error = Some(e),
            }
            self.buffer.clear();
        }
    }

    pub(super) fn flush(&mut self) -> Result<(), IndexingError> {
        use super::types::indexing_error::*;
        use snafu::ResultExt;

        if let Some(e) = self.error.take() {
            return Err(e);
        }
        if !self.buffer.is_empty() {
            self.repository
                .save_batch(&self.buffer)
                .context(StorageFailedSnafu)?;
            self.record_written();
            self.buffer.clear();
        }
        Ok(())
    }
}

/// Collector sink state that batches chunks and sends to writer thread
pub(super) struct CollectorState {
    writer_tx: Sender<WriterMessage>,
    reg_rx: Receiver<FileRegistration>,
    buffer: Vec<IndexedChunk>,
    batch_size: usize,
}

impl CollectorState {
    pub(super) fn new(
        writer_tx: Sender<WriterMessage>,
        reg_rx: Receiver<FileRegistration>,
        batch_size: usize,
    ) -> Self {
        Self {
            writer_tx,
            reg_rx,
            buffer: Vec::with_capacity(batch_size),
            batch_size,
        }
    }

    pub(super) fn ingest(&mut self, chunk: IndexedChunk) {
        // Drain pending registrations and forward to writer
        while let Ok(reg) = self.reg_rx.try_recv() {
            let _ = self.writer_tx.send(WriterMessage::Registration(reg));
        }

        self.buffer.push(chunk);

        if self.buffer.len() >= self.batch_size {
            let batch = std::mem::replace(&mut self.buffer, Vec::with_capacity(self.batch_size));
            let _ = self.writer_tx.send(WriterMessage::ChunkBatch(batch));
        }
    }

    pub(super) fn flush_and_shutdown(&mut self) {
        // Drain any remaining registrations
        while let Ok(reg) = self.reg_rx.try_recv() {
            let _ = self.writer_tx.send(WriterMessage::Registration(reg));
        }

        // Send partial batch if any
        if !self.buffer.is_empty() {
            let batch = std::mem::take(&mut self.buffer);
            let _ = self.writer_tx.send(WriterMessage::ChunkBatch(batch));
        }

        // Send shutdown
        let _ = self.writer_tx.send(WriterMessage::Shutdown);
    }
}

/// Process a batch of chunks in the writer thread
fn process_chunk_batch<R: ChunkRepository>(
    chunks: Vec<IndexedChunk>,
    sink: &mut BatchingSink<R>,
    tracker: &mut FileTracker<R>,
) {
    for chunk in chunks {
        let file_path = chunk.chunk.source.file_path.clone();
        sink.ingest(chunk);
        tracker.record_chunk(&file_path);
    }
    tracker.maybe_flush();
}

/// Run the writer thread loop
pub(super) fn run_writer_thread<R: ChunkRepository>(
    writer_rx: Receiver<WriterMessage>,
    mut sink: BatchingSink<R>,
    mut tracker: FileTracker<R>,
) -> Result<(), IndexingError> {
    for msg in writer_rx {
        match msg {
            WriterMessage::Registration(reg) => tracker.expect_file(reg),
            WriterMessage::ChunkBatch(chunks) => {
                process_chunk_batch(chunks, &mut sink, &mut tracker)
            }
            WriterMessage::Shutdown => break,
        }
    }

    // Flush sink BEFORE tracker (crash-safety invariant)
    sink.flush()?;
    tracker.flush()
}

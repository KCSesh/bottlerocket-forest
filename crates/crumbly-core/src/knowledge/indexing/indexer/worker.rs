//! Embedding worker for parallel chunk processing
//!
//! Provides [`EmbeddingWorker`] which receives chunks from a channel, batches them,
//! generates embeddings, and sends indexed chunks to an output channel.

use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;

use super::types::IndexingError;
use crate::knowledge::domain::{IndexedChunk, Timestamp};
use crate::knowledge::indexing::{IndexDataProvider, ProgressReporter};

use super::pipeline::WorkItem;

/// Worker that processes chunks from a channel and generates embeddings
#[derive(bon::Builder)]
pub(super) struct EmbeddingWorker {
    work_rx: Receiver<WorkItem>,
    output_tx: Sender<IndexedChunk>,
    provider: Arc<dyn IndexDataProvider>,
    progress: Option<Arc<dyn ProgressReporter>>,
    batch_size: usize,
    thread_pool: Arc<rayon::ThreadPool>,
}

impl EmbeddingWorker {
    /// Run the worker loop until the work channel is closed
    pub(super) fn run(self) -> Result<(), IndexingError> {
        use super::types::indexing_error::*;
        use snafu::ResultExt;

        let mut batch: Vec<WorkItem> = Vec::with_capacity(self.batch_size);

        loop {
            // Try to fill batch
            if batch.is_empty() {
                // Block on first item
                match self.work_rx.recv() {
                    Ok(item) => batch.push(item),
                    Err(_) => break, // Channel closed
                }
            }

            // Non-blocking drain up to batch_size
            while batch.len() < self.batch_size {
                match self.work_rx.try_recv() {
                    Ok(item) => batch.push(item),
                    Err(_) => break,
                }
            }

            if batch.is_empty() {
                break;
            }

            // Generate embeddings for batch using dedicated thread pool
            let texts: Vec<&str> = batch
                .iter()
                .map(|w| w.chunk.content.text.as_ref())
                .collect();
            let progress_ref = self.progress.as_ref().map(|p| p.as_ref());
            let embeddings = self
                .thread_pool
                .install(|| {
                    self.provider
                        .generate_batch_with_progress(&texts, progress_ref)
                })
                .context(IndexDataGenerationFailedSnafu)?;

            let timestamp = Timestamp::now();

            // Send indexed chunks
            for (work_item, embedding) in batch.drain(..).zip(embeddings) {
                let indexed = IndexedChunk::builder()
                    .chunk(work_item.chunk)
                    .embedding(embedding)
                    .indexed_at(timestamp)
                    .build();

                // Ignore send errors - collector may have stopped
                let _ = self.output_tx.send(indexed);
            }
        }

        Ok(())
    }
}

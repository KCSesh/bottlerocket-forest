//! Producer-consumer embedding pipeline
//!
//! Provides [`EmbeddingPipeline`] which decouples chunk production from embedding
//! generation via bounded channels, eliminating file-boundary stalls while
//! maintaining consistent CPU utilization.

use crossbeam_channel::{Receiver, Sender, bounded};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::thread;

use super::types::IndexingError;
use super::worker::EmbeddingWorker;
use crate::knowledge::domain::{Chunk, IndexedChunk};
use crate::knowledge::indexing::{IndexDataProvider, ProgressReporter};

/// Work item sent through the pipeline
pub(crate) struct WorkItem {
    pub(super) chunk: Chunk,
}

/// Builder for configuring an embedding pipeline
#[derive(bon::Builder)]
#[non_exhaustive]
pub(crate) struct EmbeddingPipeline {
    provider: Arc<dyn IndexDataProvider>,
    worker_count: NonZeroUsize,
    progress: Option<Arc<dyn ProgressReporter>>,
    #[builder(default = 32)]
    batch_size: usize,
}

impl EmbeddingPipeline {
    /// Run the pipeline with the given chunks, returning indexed chunks
    pub(crate) fn run<I>(self, chunks: I) -> Result<Vec<IndexedChunk>, IndexingError>
    where
        I: Iterator<Item = Chunk>,
    {
        let worker_count = self.worker_count.get();
        let channel_capacity = worker_count * self.batch_size;

        let (work_tx, work_rx): (Sender<WorkItem>, Receiver<WorkItem>) = bounded(channel_capacity);
        let (output_tx, output_rx): (Sender<IndexedChunk>, Receiver<IndexedChunk>) =
            bounded(channel_capacity * 2);

        // Spawn workers
        let mut handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let worker = EmbeddingWorker::new(
                work_rx.clone(),
                output_tx.clone(),
                Arc::clone(&self.provider),
                self.progress.clone(),
                self.batch_size,
            );
            handles.push(thread::spawn(move || worker.run()));
        }

        // Drop our copies so workers see channel close
        drop(work_rx);
        drop(output_tx);

        // Collector: spawn thread to drain output concurrently with producer
        // This prevents deadlock when output channel fills before producer finishes
        let collector_handle = thread::spawn(move || output_rx.iter().collect::<Vec<_>>());

        // Producer: send all chunks
        for chunk in chunks {
            // Ignore send errors - workers may have failed
            if work_tx.send(WorkItem { chunk }).is_err() {
                break;
            }
        }
        drop(work_tx); // Signal completion to workers

        // Join collector
        let results = collector_handle.join().unwrap_or_default();

        // Join workers and propagate first error
        for handle in handles {
            if let Ok(Err(e)) = handle.join() {
                return Err(e);
            }
        }

        Ok(results)
    }
}

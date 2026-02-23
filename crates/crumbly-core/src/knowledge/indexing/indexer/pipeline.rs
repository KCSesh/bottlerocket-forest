//! Producer-consumer embedding pipeline
//!
//! Provides [`EmbeddingPipeline`] which decouples chunk production from embedding
//! generation via bounded channels, eliminating file-boundary stalls while
//! maintaining consistent CPU utilization.

use crossbeam_channel::{Receiver, Sender, bounded};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use super::types::IndexingError;
use super::worker::EmbeddingWorker;
use crate::knowledge::domain::{Chunk, IndexedChunk};
use crate::knowledge::indexing::{IndexDataProvider, ProgressReporter};

/// Work item sent through the pipeline
pub(crate) struct WorkItem {
    pub(crate) chunk: Chunk,
}

/// Handle to a running embedding pipeline
pub(crate) struct PipelineHandle<F> {
    work_tx: Sender<WorkItem>,
    error_rx: Receiver<IndexingError>,
    collector_handle: JoinHandle<usize>,
    worker_handles: Vec<JoinHandle<()>>,
    _sink: std::marker::PhantomData<F>,
}

impl<F> PipelineHandle<F> {
    /// Get a reference to the work sender for submitting chunks
    pub(crate) fn sender(&self) -> &Sender<WorkItem> {
        &self.work_tx
    }

    /// Check for pipeline errors without blocking
    pub(crate) fn check_error(&self) -> Option<IndexingError> {
        self.error_rx.try_recv().ok()
    }

    /// Signal completion and wait for pipeline to finish
    pub(crate) fn join(self) -> Result<usize, IndexingError> {
        drop(self.work_tx);
        let error = self.error_rx.try_recv().ok();
        let count = self.collector_handle.join().unwrap_or(0);
        for handle in self.worker_handles {
            let _ = handle.join();
        }
        if let Some(e) = error {
            return Err(e);
        }
        if let Ok(e) = self.error_rx.try_recv() {
            return Err(e);
        }
        Ok(count)
    }
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
    /// Start the pipeline with a sink callback for processing output chunks
    pub(crate) fn start_with_sink<F>(self, mut sink: F) -> PipelineHandle<F>
    where
        F: FnMut(IndexedChunk) + Send + 'static,
    {
        let worker_count = self.worker_count.get();
        let channel_capacity = worker_count * self.batch_size;

        let (work_tx, work_rx): (Sender<WorkItem>, Receiver<WorkItem>) = bounded(channel_capacity);
        let (output_tx, output_rx): (Sender<IndexedChunk>, Receiver<IndexedChunk>) =
            bounded(channel_capacity * 2);
        let (error_tx, error_rx): (Sender<IndexingError>, Receiver<IndexingError>) =
            bounded(worker_count);

        let mut worker_handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let worker = EmbeddingWorker::builder()
                .work_rx(work_rx.clone())
                .output_tx(output_tx.clone())
                .provider(Arc::clone(&self.provider))
                .maybe_progress(self.progress.clone())
                .batch_size(self.batch_size)
                .build();
            let err_tx = error_tx.clone();
            worker_handles.push(thread::spawn(move || match worker.run() {
                Ok(()) => {}
                Err(e) => {
                    let _ = err_tx.send(e);
                }
            }));
        }

        drop(work_rx);
        drop(output_tx);
        drop(error_tx);

        let collector_handle = thread::spawn(move || {
            let mut count = 0;
            for chunk in output_rx {
                sink(chunk);
                count += 1;
            }
            count
        });

        PipelineHandle {
            work_tx,
            error_rx,
            collector_handle,
            worker_handles,
            _sink: std::marker::PhantomData,
        }
    }

    /// Run the pipeline with the given chunks, returning indexed chunks
    #[allow(dead_code)]
    pub(crate) fn run<I>(self, chunks: I) -> Result<Vec<IndexedChunk>, IndexingError>
    where
        I: Iterator<Item = Chunk>,
    {
        use std::sync::Mutex;

        let results: Arc<Mutex<Vec<IndexedChunk>>> = Arc::new(Mutex::new(Vec::new()));
        let results_clone = Arc::clone(&results);

        let handle = self.start_with_sink(move |chunk| {
            if let Ok(mut guard) = results_clone.lock() {
                guard.push(chunk);
            }
        });

        for chunk in chunks {
            if handle.check_error().is_some() {
                break;
            }
            if handle.sender().send(WorkItem { chunk }).is_err() {
                break;
            }
        }

        handle.join()?;

        let mut guard = results.lock().unwrap_or_else(|e| e.into_inner());
        Ok(std::mem::take(&mut *guard))
    }
}

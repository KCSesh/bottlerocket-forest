//! Model pool for parallel embedding generation
//!
//! Provides a pool of embedding models that can be borrowed by parallel workers.
//! Uses crossbeam channels for lock-free model distribution with RAII guards
//! for automatic return on drop.

use crossbeam_channel::{Receiver, Sender};
use snafu::{ResultExt, Snafu};
use std::num::NonZeroUsize;
use std::path::PathBuf;

use super::EmbeddingError;
use super::model::{EmbeddingModel, EmbeddingProvider, LoadedEmbeddingModel, default_cache_dir};
use crate::knowledge::domain::Embedding;
use crate::knowledge::indexing::{IndexDataError, IndexDataProvider};

const MODEL_SIZE_ESTIMATE_MB: u64 = 100;
const ENV_POOL_SIZE: &str = "CRUMBLY_MODEL_POOL_SIZE";

/// Configuration for the embedding model pool
#[derive(Debug, Clone, bon::Builder)]
#[non_exhaustive]
pub struct PoolConfig {
    /// Number of model instances in the pool
    #[builder(default = compute_default_pool_size())]
    pool_size: NonZeroUsize,
    /// Cache directory for model downloads
    #[builder(default = default_cache_dir())]
    cache_dir: PathBuf,
}

fn compute_default_pool_size() -> NonZeroUsize {
    if let Ok(val) = std::env::var(ENV_POOL_SIZE)
        && let Ok(n) = val.parse::<usize>()
        && let Some(size) = NonZeroUsize::new(n)
    {
        return size;
    }

    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let sys = sysinfo::System::new_all();
    let total_mem_mb = sys.total_memory() / (1024 * 1024);
    let mem_limit = (total_mem_mb / 8 / MODEL_SIZE_ESTIMATE_MB) as usize;

    let size = cpus.min(mem_limit).max(1);
    // SAFETY: max(1) guarantees size >= 1
    NonZeroUsize::new(size).unwrap_or(NonZeroUsize::MIN)
}

/// Pool of embedding models for parallel processing
pub struct EmbeddingModelPool {
    sender: Sender<LoadedEmbeddingModel>,
    receiver: Receiver<LoadedEmbeddingModel>,
    pool_size: NonZeroUsize,
}

impl std::fmt::Debug for EmbeddingModelPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingModelPool")
            .field("pool_size", &self.pool_size)
            .finish_non_exhaustive()
    }
}

impl EmbeddingModelPool {
    /// Create a new model pool with the given configuration
    #[allow(clippy::result_large_err)]
    pub fn new(config: PoolConfig) -> Result<Self, CreatePoolError> {
        use create_pool_error::*;

        let (sender, receiver) = crossbeam_channel::bounded(config.pool_size.get());

        for _ in 0..config.pool_size.get() {
            let model = EmbeddingModel::builder()
                .cache_dir(config.cache_dir.clone())
                .build()
                .load()
                .context(ModelLoadSnafu)?;
            sender.send(model).context(ChannelSnafu)?;
        }

        Ok(Self {
            sender,
            receiver,
            pool_size: config.pool_size,
        })
    }

    /// Borrow a model from the pool
    ///
    /// Blocks until a model is available. The model is automatically returned
    /// to the pool when the guard is dropped.
    pub fn borrow(&self) -> Result<PooledModel<'_>, BorrowModelError> {
        let model = self
            .receiver
            .recv()
            .context(borrow_model_error::ReceiveSnafu)?;
        Ok(PooledModel {
            model: Some(model),
            sender: &self.sender,
        })
    }

    /// Get the pool size
    pub fn pool_size(&self) -> NonZeroUsize {
        self.pool_size
    }
}

/// RAII guard for a borrowed model
///
/// Returns the model to the pool on drop.
pub struct PooledModel<'a> {
    model: Option<LoadedEmbeddingModel>,
    sender: &'a Sender<LoadedEmbeddingModel>,
}

impl PooledModel<'_> {
    /// Get a reference to the underlying model
    ///
    /// # Panics
    ///
    /// Panics if called after the model has been taken (should never happen
    /// in normal use as this is only called before drop).
    pub fn model(&self) -> &LoadedEmbeddingModel {
        self.model
            .as_ref()
            .unwrap_or_else(|| unreachable!("model taken before drop"))
    }
}

impl Drop for PooledModel<'_> {
    fn drop(&mut self) {
        if let Some(model) = self.model.take() {
            // Ignore send errors - pool may be shutting down
            let _ = self.sender.send(model);
        }
    }
}

/// Embedding provider backed by a model pool
///
/// Each call to generate borrows a model from the pool, enabling parallel
/// embedding generation across multiple threads.
pub struct PooledEmbeddingProvider {
    pool: EmbeddingModelPool,
}

impl std::fmt::Debug for PooledEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledEmbeddingProvider")
            .field("pool", &self.pool)
            .finish()
    }
}

impl PooledEmbeddingProvider {
    /// Create a new pooled provider with default configuration
    #[allow(clippy::result_large_err)]
    pub fn new() -> Result<Self, CreatePoolError> {
        Self::with_config(PoolConfig::builder().build())
    }

    /// Create a new pooled provider with custom configuration
    #[allow(clippy::result_large_err)]
    pub fn with_config(config: PoolConfig) -> Result<Self, CreatePoolError> {
        Ok(Self {
            pool: EmbeddingModelPool::new(config)?,
        })
    }

    /// Get the pool size
    pub fn pool_size(&self) -> NonZeroUsize {
        self.pool.pool_size()
    }
}

impl IndexDataProvider for PooledEmbeddingProvider {
    fn generate(&self, text: &str) -> Result<Embedding, IndexDataError> {
        let guard = self
            .pool
            .borrow()
            .map_err(|e| IndexDataError::EmbeddingFailed {
                source: Box::new(e),
            })?;
        guard
            .model()
            .embed(text)
            .map_err(|e| IndexDataError::EmbeddingFailed {
                source: Box::new(e),
            })
    }

    #[allow(clippy::needless_lifetimes)]
    fn generate_batch<'a>(&self, texts: &[&'a str]) -> Result<Vec<Embedding>, IndexDataError> {
        let guard = self
            .pool
            .borrow()
            .map_err(|e| IndexDataError::EmbeddingFailed {
                source: Box::new(e),
            })?;
        let text_strings: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        guard
            .model()
            .embed_batch(text_strings)
            .map_err(|e| IndexDataError::EmbeddingFailed {
                source: Box::new(e),
            })
    }
}

/// Error creating the model pool
#[derive(Debug, Snafu)]
#[snafu(module)]
#[allow(clippy::large_enum_variant)]
pub enum CreatePoolError {
    /// Failed to load embedding model
    #[snafu(display("Failed to load embedding model"))]
    ModelLoad {
        /// Underlying embedding error.
        source: EmbeddingError,
    },

    /// Failed to initialize channel
    #[snafu(display("Failed to initialize pool channel"))]
    Channel {
        /// Underlying channel error.
        source: crossbeam_channel::SendError<LoadedEmbeddingModel>,
    },
}

/// Error borrowing a model from the pool
#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum BorrowModelError {
    /// Channel receive failed
    #[snafu(display("Failed to receive model from pool"))]
    Receive {
        /// Underlying channel error.
        source: crossbeam_channel::RecvError,
    },
}

//! Knowledge indexing for Bottlerocket documentation
//!
//! This module provides semantic search across forest documentation.

pub mod chunking;
pub mod constants;
pub mod domain;
pub mod error;
pub mod indexing;
pub mod scoring;
pub mod search;
pub mod storage;

// Modules to be added in subsequent commits:
// pub mod context;
// pub mod facade;

pub use chunking::{ChunkingError, ChunkingInput, ChunkingStrategy};
pub use constants::*;
pub use domain::*;
pub use error::*;
pub use indexing::{
    FileScanner, IndexResult, IndexStrategy, IndexableFile, Indexer, IndexingError, IndexingFilter,
    RustFilter, RustItemType, ScanError, SemblyConfig, SemblyConfigError, load_sembly_config,
};
pub use scoring::{BoostMultiplier, BoostPattern, BoostRule, ScoreBooster, default_boost_rules};
pub use search::{SearchEngine, SearchError, SemanticSearchEngine};
pub use storage::{ChunkRepository, StorageError};

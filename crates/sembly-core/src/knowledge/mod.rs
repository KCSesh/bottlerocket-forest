//! Knowledge indexing for Bottlerocket documentation
//!
//! This module provides semantic search across forest documentation.

pub mod constants;
pub mod domain;
pub mod error;
pub mod storage;
pub mod chunking;
pub mod indexing;
pub mod scoring;
pub mod search;

// Modules to be added in subsequent commits:
// pub mod context;
// pub mod facade;

pub use constants::*;
pub use domain::*;
pub use error::*;
pub use storage::{ChunkRepository, StorageError};
pub use chunking::{ChunkingError, ChunkingInput, ChunkingStrategy};
pub use indexing::{IndexingFilter, RustFilter, RustItemType};
pub use scoring::{BoostMultiplier, BoostPattern, BoostRule, ScoreBooster, default_boost_rules};
pub use search::{SearchEngine, SearchError, SemanticSearchEngine};

//! Knowledge indexing for Bottlerocket documentation
//!
//! This module provides semantic search across forest documentation.

pub mod chunking;
pub mod constants;
pub mod domain;
pub mod error;
pub mod indexing;
pub mod storage;

// Modules to be added in subsequent commits:
pub mod search;
pub mod scoring;
// pub mod context;
// pub mod facade;

pub use chunking::{ChunkingError, ChunkingInput, ChunkingStrategy};
pub use constants::*;
pub use domain::*;
pub use error::*;
pub use indexing::{IndexingFilter, RustFilter, RustItemType};
pub use storage::{ChunkRepository, StorageError};
pub use search::{SearchEngine, SearchError, SemanticSearchEngine};
pub use scoring::{BoostMultiplier, BoostPattern, BoostRule, ScoreBooster, default_boost_rules};

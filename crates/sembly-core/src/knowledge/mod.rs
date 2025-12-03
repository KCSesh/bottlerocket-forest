//! Knowledge indexing for Bottlerocket documentation
//!
//! This module provides semantic search across forest documentation.

pub mod constants;
pub mod domain;
pub mod error;

// Modules to be added in subsequent commits:
pub mod storage;
// pub mod chunking;
// pub mod search;
// pub mod scoring;
// pub mod indexing;
// pub mod context;
// pub mod facade;

pub use constants::*;
pub use domain::*;
pub use error::*;
pub use storage::{ChunkRepository, StorageError};

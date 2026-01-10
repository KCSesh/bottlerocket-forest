//! Centralizes grove detection and context for Bottlerocket development.
//!
//! Provides **GroveContext** as the primary abstraction for grove-aware operations,
//! walking up the directory tree to find the `.grove/` marker directory.

mod context;

pub use context::{GroveContext, GroveContextError};

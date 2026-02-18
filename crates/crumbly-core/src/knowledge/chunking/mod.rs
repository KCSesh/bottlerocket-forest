//! Splits documentation files into searchable chunks for semantic indexing.
//!
//! Provides strategies for chunking markdown, Rust, Go, C, and shell source files while preserving
//! structural context (heading hierarchies, item metadata). Uses token-aware splitting
//! with configurable overlap to respect embedding model constraints.

pub mod cdoc;
pub mod dispatcher;
pub mod godoc;
pub mod javadoc;
pub mod language;
pub mod markdown;
pub mod rustdoc;
pub mod shell;
pub mod strategy;

pub use dispatcher::{ChunkingDispatcher, DispatchError};
pub use language::{LanguageConfig, LanguageSupport};
pub use strategy::{ChunkingError, ChunkingInput, ChunkingStrategy};

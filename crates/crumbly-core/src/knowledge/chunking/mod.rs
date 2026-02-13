//! Splits documentation files into searchable chunks for semantic indexing.
//!
//! Provides strategies for chunking markdown, Rust, and Go source files while preserving
//! structural context (heading hierarchies, item metadata). Uses token-aware splitting
//! with configurable overlap to respect embedding model constraints.

pub mod dispatcher;
pub mod godoc;
pub mod javadoc;
pub mod language;
pub mod markdown;
pub mod rustdoc;
pub mod strategy;

pub use dispatcher::{ChunkingDispatcher, DispatchError};
pub use godoc::{
    GoConfig, GoDocChunker, GoDocContext, GoDocSupport, GoFilter, GoItemType, GoVisibility,
};
pub use javadoc::{JavaDocChunker, JavaDocContext, JavaFilter, JavaItemType, JavaVisibility};
pub use language::{LanguageConfig, LanguageSupport};
pub use markdown::{MarkdownChunker, MarkdownContext, MarkdownSupport};
pub use rustdoc::{RustDocChunker, RustDocContext, RustDocSupport, RustFilter, RustItemType};
pub use strategy::{ChunkingError, ChunkingInput, ChunkingStrategy};

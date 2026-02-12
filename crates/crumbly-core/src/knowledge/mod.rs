//! Knowledge indexing for Bottlerocket documentation
//!
//! This module provides semantic search across forest documentation,
//! enabling fast, targeted documentation lookup for AI agents and developers.
//!
//! # Architecture
//!
//! The knowledge index is organized into layers:
//!
//! * **Domain Layer** (`domain/`): Type-safe domain models
//!   Defines core concepts like `Chunk` and `SearchQuery`.
//!
//! * **Storage Layer** (`storage/`): Knowledgebase persistence
//!
//! * **Chunking Layer** (`chunking/`): Splits documentation into searchable chunks with
//!   context preservation (markdown headings, rustdoc items).
//!
//! * **Indexing Layer** (`indexing/`): File scanning and index building
//!
//! * **Search Layer** (`search/`): Semantic search engine using all-MiniLM-L6-v2 embeddings
//!
//! * **Facade Layer** (`facade/`): High-level API via [`KnowledgeIndex`]
//!
//! # Quick Start
//!
//! ```no_run
//! use crumbly_core::knowledge::KnowledgeIndex;
//! use crumbly_core::knowledge::domain::{QueryText, ResultLimit};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let index = KnowledgeIndex::open("/path/to/forest")?;
//! let result = index.build().call()?;
//! let query = QueryText::try_new("boot process")?;
//! let limit = ResultLimit::try_new(20).unwrap();
//! let results = index.search(query, limit)?;
//! # Ok(())
//! # }
//! ```

pub mod chunking;
pub mod constants;
pub mod context;
pub mod domain;
pub mod embeddings;
pub mod error;
pub mod facade;
pub mod indexing;
pub mod scoring;
pub mod search;
pub mod storage;

pub use chunking::{ChunkingError, ChunkingInput, ChunkingStrategy};
pub use context::{DiscoveryError, Workspace, discover_workspace};
pub use domain::{
    Chunk, ChunkContent, ChunkContext, ChunkId, ChunkSource, ChunkableContent, Embedding,
    EmbeddingModelConfig, FileType, IndexMetadata, IndexRelativePath, IndexedChunk,
    MarkdownContext, RepoName, RustDocContext, SearchQuery, SearchResult, SearchResults, Timestamp,
};
pub use facade::{CacheResult, CacheSource, IndexError, IndexStatus, KnowledgeIndex};
pub use indexing::{
    CrumblyConfig, CrumblyConfigError, FileScanner, IndexResult, IndexStrategy, IndexableFile,
    Indexer, IndexingError, ScanConfig, ScanError, load_crumbly_config,
};
pub use scoring::{BoostMultiplier, BoostPattern, BoostRule, ScoreBooster, default_boost_rules};
pub use search::{SearchEngine, SearchError, SemanticSearchEngine};
pub use storage::{ChunkRepository, StorageError};

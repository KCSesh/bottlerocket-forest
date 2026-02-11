# crumbly-core

Semantic search library for code and documentation.

## Module Organization

```
knowledge/
├── domain/          # Core types (Chunk, SearchQuery, Embedding, etc.)
├── embeddings/      # Embedding model and vector operations (shared by indexing & search)
├── chunking/        # Text splitting strategies (markdown, rustdoc, godoc, javadoc)
├── context/         # Workspace discovery and context resolution
├── storage/         # Persistence layer (SQLite repository)
├── search/          # Query execution and semantic search
├── indexing/        # Index building pipeline
│   ├── indexer/     # Orchestration of indexing workflow
│   ├── scanner/     # File discovery and ScanConfig
│   ├── source/      # Content source abstraction (filesystem, bare git)
│   ├── cacher/      # Chunk caching
│   ├── config/      # .crumbly.toml configuration
│   └── filter/      # Filtering logic for what gets indexed
├── scoring/         # Search result score boosting
└── facade/          # High-level KnowledgeIndex API
```

## Quick Start

```rust
use crumbly_core::knowledge::KnowledgeIndex;
use crumbly_core::knowledge::domain::{QueryText, ResultLimit};

let index = KnowledgeIndex::open("/path/to/project")?;
index.build().call()?;

let query = QueryText::try_new("boot process")?;
let limit = ResultLimit::try_new(20).unwrap();
let results = index.search(query, limit)?;
```

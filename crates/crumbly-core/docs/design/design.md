# Crumbly Core - Technical Design

## Overview

Crumbly Core implements semantic search for documentation using a layered architecture with type-safe domain models at its core. The design prioritizes testability, modularity, and performance while keeping the embedding model and vector database self-contained.

Design Philosophy: The architecture is organized into vertical layers, each with clear responsibilities. Domain types define the vocabulary of the system. Each layer builds on the types and capabilities of layers below it, with the facade providing a simple high-level API.

## Critical Constraints

Non-negotiable implementation requirements.

| ID | Constraint | Rationale | Anti-pattern to avoid |
|----|------------|-----------|----------------------|
| CC-1 | All domain types MUST be in the domain module with no external dependencies | Enables pure unit testing and prevents coupling | Domain types importing from storage or other layers |
| CC-2 | Vector similarity search MUST happen in the database, not application code | Performance - filtering 100K vectors in-memory is too slow | Loading all embeddings and computing similarity in Rust |
| CC-3 | Chunking strategies MUST preserve semantic coherence | Search quality depends on meaningful chunks | Fixed-size sliding windows that split mid-sentence |
| CC-4 | Embedding model MUST run locally without network calls | Reliability and privacy - no external API dependencies | Calling AWS Bedrock API or other cloud embedding APIs |
| CC-5 | Storage layer MUST support incremental updates | Performance - reindexing everything on each update is prohibitive | Deleting and recreating the entire index on updates |

## Architecture

The system is organized into vertical layers:

Facade Layer (KnowledgeIndex)
 ↓
Search Layer (SemanticSearchEngine)
 ↓
Scoring Layer (ScoreBooster)
 ↓
Indexing Layer (Indexer, FileScanner)
 ↓
Chunking Layer (ChunkingStrategy)
 ↓
Storage Layer (ChunkRepository)
 ↓
Domain Layer (Chunk, SearchQuery, Embedding, etc.)
 ↓
Context Layer (Workspace discovery, Multi-context support)


### Layer Responsibilities

Domain Layer (domain/) - Type-safe domain models that define the vocabulary of the system. Core types include Chunk, SearchQuery, SearchResult, Embedding, ChunkId, ChunkMetadata, ContextId, etc. No dependencies on external crates beyond standard library.

Storage Layer (storage/) - Knowledgebase persistence using SQLite + sqlite-vec. Implements ChunkRepository trait for storing and retrieving chunks with embeddings. Handles vector similarity search in the database. Supports multi-context storage with shared chunk deduplication.

Chunking Layer (chunking/) - Splits documentation into searchable chunks with context preservation. Different strategies for different content types: MarkdownChunker preserves heading hierarchy, RustDocChunker extracts doc comments and associates with code items.

Indexing Layer (indexing/) - File scanning and index building. FileScanner traverses directories respecting .gitignore and .crumblyignore. Indexer orchestrates the indexing workflow: scan → chunk → embed → store. Supports incremental updates via modification time tracking. Context-aware indexing for multi-context support.

Search Layer (search/) - Semantic search engine using all-MiniLM-L6-v2 embeddings. SemanticSearchEngine converts queries to embeddings and performs vector similarity search via the storage layer. Context-aware search.

Scoring Layer (scoring/) - Boost rules for result ranking. ScoreBooster applies configurable boost patterns to adjust relevance scores based on file paths, content patterns, or metadata.

Context Layer (context/) - Workspace discovery and multi-context management. discover_workspace() locates the forest root and identifies repository structure. Workspace provides context about the codebase being indexed. ContextManager handles multiple working directories sharing a single embedding database.

Facade Layer (facade/) - High-level API via KnowledgeIndex. Provides simple methods like build() and search() that hide internal complexity. Users interact with one entry point, not individual layers.

## Multi-Context Support

Contexts enable multiple working directories (like git worktrees) to share a single embedding database while maintaining isolated file mappings. This is critical for AI agents working with git worktrees - they can have separate indexes for different branches while sharing the embedding database.

### Key Concepts

ContextId
- Unique identifier for a context
- Represents a canonicalized relative path from the database root
- Default context is "." (workspace root)
- Worktrees get their own context (e.g., "worktrees/feature-a")
- Newtype wrapper for type safety

Context Isolation
- Each context maintains its own file-to-chunk mappings
- Chunks are shared and deduplicated across contexts
- Same file content in different contexts references the same chunk
- Deleting a context doesn't delete shared chunks still referenced by other contexts

### Operations

Context Registration
- Contexts are automatically registered when indexing with --context flag
- crumbly index --context worktrees/feature-a creates and indexes that context
- Context metadata stored in database (context_id, root_path, created_at, last_updated)

Context Listing
- crumbly context list shows all registered contexts
- Displays context_id, root_path, chunk_count, last_updated
- Indicates which context is currently active

Context Removal
- crumbly context remove <id> removes a context
- Deletes file mappings and metadata for that context
- Does NOT delete chunks (they may be referenced by other contexts)
- Use crumbly gc afterward to clean up orphaned chunks

Garbage Collection
- crumbly gc removes orphaned chunks not referenced by any context
- Scans all contexts to build reference set
- Deletes chunks with zero references
- Returns statistics: chunks_removed, space_reclaimed

Context-Aware Search
- crumbly search "query" --context worktrees/feature-a searches within that context
- Default searches the "." context
- Results only include chunks mapped to the specified context

### Storage Schema

contexts table
- context_id (TEXT PRIMARY KEY)
- root_path (TEXT)
- created_at (INTEGER)
- last_updated (INTEGER)

context_chunks table (many-to-many mapping)
- context_id (TEXT)
- chunk_id (TEXT)
- source_path (TEXT) - path relative to context root
- PRIMARY KEY (context_id, chunk_id)
- FOREIGN KEY (context_id) REFERENCES contexts(context_id)
- FOREIGN KEY (chunk_id) REFERENCES chunks(chunk_id)

chunks table (shared across contexts)
- chunk_id (TEXT PRIMARY KEY)
- content (TEXT)
- chunk_context (TEXT) - structural context (heading path, etc.)

embeddings table (shared across contexts)
- chunk_id (TEXT PRIMARY KEY)
- embedding (BLOB)
- FOREIGN KEY (chunk_id) REFERENCES chunks(chunk_id)

files table (per-context modification tracking)
- context_id (TEXT)
- file_path (TEXT)
- last_modified (INTEGER)
- PRIMARY KEY (context_id, file_path)

### Implementation Notes

Chunk Deduplication
- Chunk content hash determines chunk_id
- Same content in different contexts maps to same chunk_id
- Embedding is computed once and shared

Reference Counting
- Implicit via context_chunks table
- Chunk is orphaned when no rows reference it in context_chunks
- GC uses LEFT JOIN to find chunks with zero references

Context Discovery
- Default context "." is created automatically
- Worktree contexts detected via git worktree list
- Custom contexts specified via --context flag

## Domain Model

### Core Types

Chunk
- Represents a semantically coherent piece of documentation
- Properties: id (ChunkId), content (ChunkContent), source (ChunkSource), context (ChunkContext)
- Validation: content must be non-empty, source must be valid

ChunkId
- Unique identifier for chunks
- Internally a hash of content for deduplication
- Immutable once created

ChunkContent
- The actual text content of a chunk
- Newtype wrapper around String for type safety
- Validation: must be non-empty

ChunkSource
- Where a chunk came from
- Properties: file_path (ForestRelativePath), file_type (FileType), byte_range
- Enables incremental updates and result presentation

ChunkContext
- Structural context about a chunk
- Variants: Markdown (heading path), RustDoc (item path), None
- Preserves document structure for better search results

ContextId
- Unique identifier for a context
- Newtype wrapper around canonicalized relative path
- Default is "." for workspace root
- Validation: must be valid relative path

SearchQuery
- User's search request
- Properties: query_text (String), limit (usize), filters (optional), context_id (ContextId)
- Validation: query_text must be non-empty, limit must be positive

SearchResult
- A chunk matching a search query
- Properties: chunk (Chunk), similarity_score (f32), boosted_score (f32)
- Ordered by boosted_score descending

Embedding
- Vector representation of text
- Properties: vector (Vec<f32>)
- Dimensions determined by embedding model (384 for MiniLM)

EmbeddingModelConfig
- Configuration for the embedding model
- Properties: model_name, dimensions, normalization settings
- Stored in index metadata to ensure consistency

IndexMetadata
- Metadata about the index
- Properties: created_at, updated_at, embedding_config, chunk_count, context_count
- Persisted in storage for version tracking

IndexedChunk
- A chunk with its embedding
- Properties: chunk (Chunk), embedding (Embedding)
- Used during indexing workflow

ForestRelativePath
- Path relative to forest root
- Newtype wrapper for type safety
- Enables portable index files

FileType
- Enum: Markdown, Rust, PlainText
- Determines which chunking strategy to use

Timestamp
- Newtype wrapper around SystemTime
- Used for modification time tracking

### Domain Operations

index_files(paths: Vec<PathBuf>, context_id: ContextId) -> Result<IndexResult, IndexError>
- Scans files, chunks content, generates embeddings, stores in database
- Associates chunks with specified context
- Returns statistics: files processed, chunks created, errors encountered
- **Invariants**: Only modified files are reprocessed; old chunks are removed before adding new ones; chunks are deduplicated across contexts

search(query: SearchQuery) -> Result<SearchResults, SearchError>
- Converts query to embedding, performs vector similarity search, applies boost rules, returns ranked results
- Filters results to specified context
- **Invariants**: Results are always ordered by boosted score descending; limit is enforced in database query, not application

update_index(paths: Vec<PathBuf>, context_id: ContextId) -> Result<IndexResult, IndexError>
- Incremental update: checks modification times, only reprocesses changed files
- Removes chunks for deleted files from specified context
- **Invariants**: Unchanged files are never reprocessed; file modification tracking is persisted per-context

list_contexts() -> Result<Vec<ContextInfo>, StorageError>
- Returns all registered contexts with metadata
- ContextInfo includes: context_id, root_path, chunk_count, last_updated

remove_context(context_id: ContextId) -> Result<(), StorageError>
- Removes context and its file mappings
- Does not delete shared chunks

garbage_collect() -> Result<GcResult, StorageError>
- Removes orphaned chunks not referenced by any context
- Returns statistics: chunks_removed, space_reclaimed

### Error Types

IndexError
- Variants: StorageError, ChunkingError, ScanError, ConfigError, ContextError
- Wraps lower-level errors from different layers
- Provides context about which file or operation failed

SearchError
- Variants: InvalidQuery, StorageError, EmbeddingError, ContextNotFound
- Distinguishes user errors (invalid query) from system errors

StorageError
- Variants: ConnectionFailed, QueryFailed, CorruptedData, MigrationFailed
- Storage layer errors

ChunkingError
- Variants: InvalidContent, UnsupportedFileType, ParseError
- Chunking layer errors

ScanError
- Variants: IoError, InvalidPath, PermissionDenied
- File scanning errors

ContextError
- Variants: InvalidContextId, ContextNotFound, ContextAlreadyExists
- Context management errors

## Layer Details

### Domain Layer (domain/)

Exports all core types:
- Chunk, ChunkContent, ChunkContext, ChunkId, ChunkSource
- SearchQuery, SearchResult, SearchResults
- Embedding, EmbeddingModelConfig
- IndexMetadata, IndexedChunk
- ForestRelativePath, FileType, Timestamp
- ContextId, ContextInfo, GcResult
- MarkdownContext, RustDocContext
- RepoName, ScanConfig
- ChunkableContent trait

All types are pure data structures with no external dependencies. Validation logic is implemented as methods on the types themselves.

### Storage Layer (storage/)

ChunkRepository trait
- store_chunks(&mut self, chunks: Vec<IndexedChunk>, context_id: ContextId) -> Result<(), StorageError>
- search_similar(&self, query_embedding: Embedding, limit: usize, context_id: ContextId) -> Result<Vec<(Chunk, f32)>, StorageError>
- remove_chunks(&mut self, source_path: &ForestRelativePath, context_id: ContextId) -> Result<(), StorageError>
- get_indexed_files(&self, context_id: ContextId) -> Result<Vec<(ForestRelativePath, Timestamp)>, StorageError>
- get_metadata(&self) -> Result<IndexMetadata, StorageError>
- list_contexts(&self) -> Result<Vec<ContextInfo>, StorageError>
- remove_context(&mut self, context_id: ContextId) -> Result<(), StorageError>
- garbage_collect(&mut self) -> Result<GcResult, StorageError>

Implementation: SqliteVecStorage
- Uses SQLite with sqlite-vec extension
- Schema: contexts, context_chunks (many-to-many), chunks (shared), embeddings (shared), files (per-context), metadata
- Vector similarity search uses sqlite-vec's cosine distance function with context filtering
- Transactions ensure atomic updates
- Chunk deduplication via content hash

### Chunking Layer (chunking/)

ChunkingStrategy trait
- chunk(&self, input: ChunkingInput) -> Result<Vec<Chunk>, ChunkingError>
- supported_file_types(&self) -> &[FileType]

ChunkingInput
- Properties: content (String), source (ChunkSource)
- Passed to chunking strategies

Implementations:
- MarkdownChunker: Splits on heading boundaries (##, ###, etc.), preserves heading hierarchy in ChunkContext::Markdown
- RustDocChunker: Extracts doc comments, associates with code items (functions, structs, etc.), stores in ChunkContext::RustDoc
- PlainTextChunker: Fallback for unsupported types, splits on paragraph boundaries

### Indexing Layer (indexing/)

FileScanner
- scan(&self, config: ScanConfig) -> Result<Vec<IndexableFile>, ScanError>
- Traverses directories respecting ignore files
- Returns IndexableFile with path, file type, and metadata

Indexer
- Orchestrates indexing workflow
- index(&mut self, files: Vec<IndexableFile>, context_id: ContextId) -> Result<IndexResult, IndexError>
- Steps: chunk → embed → store
- Handles incremental updates via modification time comparison per-context

CrumblyConfig
- Configuration file (.crumbly.toml) for customizing indexing
- Properties: include/exclude patterns, boost rules, chunking settings
- Loaded via load_crumbly_config()

IndexableFile
- Properties: path, file_type, last_modified
- Represents a file ready for indexing

IndexResult
- Statistics about indexing operation
- Properties: files_processed, chunks_created, errors

### Search Layer (search/)

SearchEngine trait
- search(&self, query: SearchQuery) -> Result<SearchResults, SearchError>

SemanticSearchEngine
- Implements SearchEngine using embeddings
- embed_query() converts text to vector
- Delegates to storage layer for similarity search with context filtering
- Integrates with scoring layer for boost rules

EmbeddingModel (internal)
- Uses all-MiniLM-L6-v2 via ONNX runtime
- embed(&self, text: &str) -> Result<Embedding, EmbeddingError>
- embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError>
- Batch processing for efficiency (32-64 chunks per batch)

### Scoring Layer (scoring/)

ScoreBooster
- Applies boost rules to search results
- boost(&self, results: Vec<SearchResult>) -> Vec<SearchResult>
- Rules are configurable via BoostRule

BoostRule
- Properties: pattern (BoostPattern), multiplier (BoostMultiplier)
- Patterns can match file paths, content, or metadata

BoostPattern
- Enum: PathGlob, ContentRegex, FileType, etc.
- Flexible matching for different boost scenarios

BoostMultiplier
- Newtype wrapper around f32
- Validation: must be positive

default_boost_rules()
- Provides sensible defaults (e.g., boost README files, boost exact matches)

### Context Layer (context/)

Workspace
- Properties: root_path, repositories, config
- Represents the forest being indexed

discover_workspace()
- Locates forest root by searching for .git or Cargo.toml
- Identifies repository structure
- Returns Result<Workspace, DiscoveryError>

ContextManager
- Manages multiple contexts
- discover_contexts() -> Result<Vec<ContextId>, ContextError>
- Detects git worktrees automatically
- Handles context registration and lifecycle

### Facade Layer (facade/)

KnowledgeIndex
- High-level API for indexing and search
- open(path: impl AsRef<Path>) -> Result<Self, IndexError>
- build() -> IndexBuilder - fluent API for indexing
- search(query: &str, limit: usize) -> Result<SearchResults, SearchError>
- status() -> Result<IndexStatus, IndexError>
- list_contexts() -> Result<Vec<ContextInfo>, IndexError>
- remove_context(context_id: ContextId) -> Result<(), IndexError>
- gc() -> Result<GcResult, IndexError>

IndexBuilder
- Fluent API for configuring indexing
- with_config(config: CrumblyConfig) -> Self
- with_context(context_id: ContextId) -> Self
- incremental(bool) -> Self
- call() -> Result<IndexResult, IndexError>

IndexStatus
- Properties: chunk_count, context_count, last_updated, embedding_config
- Provides information about the current index

## Module Structure

src/knowledge/
├── mod.rs                    # Module documentation, public exports
├── domain/                   # Type-safe domain models
│   ├── mod.rs
│   ├── chunk.rs              # Chunk, ChunkId, ChunkContent, ChunkSource, ChunkContext
│   ├── search.rs             # SearchQuery, SearchResult, SearchResults
│   ├── embedding.rs          # Embedding, EmbeddingModelConfig
│   ├── metadata.rs           # IndexMetadata, FileType, Timestamp
│   ├── context.rs            # ContextId, ContextInfo, GcResult
│   └── path.rs               # ForestRelativePath, RepoName
├── storage/                  # Knowledgebase persistence
│   ├── mod.rs
│   ├── repository.rs         # ChunkRepository trait
│   ├── sqlite.rs             # SqliteVecStorage implementation
│   └── schema.sql            # Database schema
├── chunking/                 # Content splitting
│   ├── mod.rs
│   ├── strategy.rs           # ChunkingStrategy trait, ChunkingInput
│   ├── markdown.rs           # MarkdownChunker
│   ├── rustdoc.rs            # RustDocChunker
│   └── plaintext.rs          # PlainTextChunker
├── indexing/                 # File scanning and index building
│   ├── mod.rs
│   ├── scanner.rs            # FileScanner
│   ├── indexer.rs            # Indexer
│   ├── config.rs             # CrumblyConfig, load_crumbly_config()
│   └── file.rs               # IndexableFile
├── search/                   # Semantic search
│   ├── mod.rs
│   ├── engine.rs             # SearchEngine trait, SemanticSearchEngine
│   └── embedding.rs          # EmbeddingModel (internal)
├── scoring/                  # Boost rules
│   ├── mod.rs
│   ├── booster.rs            # ScoreBooster
│   └── rules.rs              # BoostRule, BoostPattern, BoostMultiplier
├── context/                  # Workspace discovery and multi-context
│   ├── mod.rs
│   ├── workspace.rs          # Workspace
│   ├── discovery.rs          # discover_workspace()
│   └── manager.rs            # ContextManager
├── facade/                   # High-level API
│   ├── mod.rs
│   └── knowledge_index.rs    # KnowledgeIndex, IndexBuilder, IndexStatus
├── error.rs                  # Error types
└── constants.rs              # Constants (chunk size limits, etc.)


## Design Patterns

Layered Architecture
- Each layer has clear responsibilities and builds on layers below
- Dependencies flow downward (facade → search → storage → domain)
- Domain layer has no dependencies on other layers

Newtype Pattern
- Domain types use newtype wrappers for type safety (ChunkId, ForestRelativePath, BoostMultiplier, ContextId)
- Prevents mixing up semantically different values of the same underlying type

Strategy Pattern
- ChunkingStrategy trait allows different chunking algorithms per content type
- Selected at runtime based on file type

Repository Pattern
- ChunkRepository trait abstracts persistence details
- Domain code doesn't know about SQLite or vector databases

Facade Pattern
- KnowledgeIndex provides a simplified API hiding internal complexity
- Users interact with one entry point, not individual layers

Builder Pattern
- IndexBuilder provides fluent API for configuring indexing
- Makes optional parameters explicit and readable

## Implementation Guidance

Chunking Strategy Selection
- Use file extension to determine FileType
- Markdown (.md) → MarkdownChunker
- Rust (.rs) → RustDocChunker
- Others → PlainTextChunker
- Strategies are stateless and can be shared

Embedding Batch Processing
- Collect chunks into batches of 32-64 before calling embed_batch()
- Reduces model invocation overhead
- Balance batch size with memory usage

Database Schema
- Use sqlite-vec's vec0 virtual table for vector storage
- Separate tables: contexts, context_chunks (many-to-many), chunks (shared), embeddings (shared), files (per-context), metadata
- Index on (context_id, source_path) for efficient incremental updates
- Index on chunk_id in context_chunks for efficient GC
- Store embedding config in metadata table to detect incompatible changes
- Chunk deduplication via content hash as chunk_id

Error Propagation
- Use Result types throughout
- Convert lower-level errors to higher-level errors at layer boundaries
- Preserve error context (which file, which operation, which context)

Incremental Updates
- Store file modification times per-context in database
- On update, query for files with newer mtime in specified context
- Remove old chunks from context before inserting new ones (transaction)
- Handle file deletions by comparing database state with filesystem per-context

Type Safety
- Use domain types everywhere, not primitives
- ForestRelativePath instead of String for paths
- ChunkId instead of String for identifiers
- ContextId instead of String for contexts
- Validation happens at type construction time

Multi-Context Operations
- Always specify context_id in storage operations
- Default to "." context when not specified
- Use transactions for context removal to ensure consistency
- GC should be run periodically or after context removal

## Testing Strategy

Unit tests: Domain types and individual layer components
- Test Chunk creation and validation
- Test SearchQuery validation
- Test chunking strategies with sample content
- Test boost rule matching
- Test ContextId validation and canonicalization

Integration tests: Cross-layer interactions
- Test end-to-end indexing workflow
- Test search with real embeddings and database
- Test incremental updates
- Test workspace discovery
- Test multi-context indexing and search
- Test context removal and garbage collection
- Test chunk deduplication across contexts

Edge cases to cover:
- Empty files
- Files with no extractable content
- Very large files (chunking behavior)
- Concurrent access to database
- Database corruption recovery
- Missing or invalid embedding model
- Modification time edge cases (clock skew, etc.)
- Context with no chunks
- Removing non-existent context
- GC with no orphaned chunks
- Same file content in multiple contexts

## Design Decisions

### DD-1: SQLite + sqlite-vec vs. Dedicated Vector Database

Decision: Use SQLite with sqlite-vec extension

Alternatives considered:
1. Qdrant (dedicated vector database)
2. Milvus (distributed vector database)
3. SQLite + sqlite-vec (chosen)

Rationale: 
- Single-file database simplifies deployment (no separate server)
- sqlite-vec provides sufficient performance for up to 1M vectors
- Reduces operational complexity and dependencies
- Enables easy backup and portability
- Performance is adequate for local/single-machine use case

Implications: 
- Not suitable for distributed deployments
- Performance may degrade beyond 1M chunks (acceptable for target use case)
- Must bundle sqlite-vec extension with library

### DD-2: Embedding Model Selection

Decision: Use all-MiniLM-L6-v2 via ONNX runtime

Alternatives considered:
1. AWS Bedrock API (cloud-based)
2. Sentence-BERT (larger model)
3. all-MiniLM-L6-v2 (chosen)

Rationale:
- Runs locally without network calls (privacy, reliability)
- Small model size (~80MB) balances quality and resource usage
- Fast inference (~10ms per embedding on CPU)
- Good semantic understanding for documentation search
- ONNX format enables cross-platform deployment

Implications:
- Must bundle model weights with library or download on first use
- Embedding quality is fixed (can't easily upgrade without breaking index)
- Model dimensions (384) determine storage requirements

### DD-3: Chunking Strategy - Heading-Based vs. Fixed-Size

Decision: Use content-aware chunking (heading boundaries for Markdown, doc comments for Rust)

Alternatives considered:
1. Fixed-size sliding windows (e.g., 512 tokens)
2. Sentence-based chunking
3. Content-aware chunking (chosen)

Rationale:
- Preserves semantic coherence (complete sections)
- Maintains document structure context
- Avoids splitting mid-thought
- Heading hierarchy provides useful metadata for result presentation
- Better search quality than arbitrary splits

Implications:
- Chunk sizes vary (must handle in UI)
- Requires content-type-specific strategies
- More complex than fixed-size windows

### DD-4: Incremental Updates vs. Full Reindex

Decision: Support incremental updates with modification time tracking

Alternatives considered:
1. Full reindex on every update
2. Incremental updates with mtime tracking (chosen)
3. Watch-based updates (filesystem events)

Rationale:
- Full reindex is too slow for large codebases
- mtime tracking is simple and reliable
- Avoids complexity of filesystem watchers
- Enables efficient CI/CD integration (only reindex changed files)

Implications:
- Must persist file modification times in database per-context
- Must handle file deletions (remove orphaned chunks)
- Requires transaction support for atomic updates

### DD-6: Multi-Context Architecture - Shared Chunks vs. Isolated Databases

Decision: Share chunks and embeddings across contexts with many-to-many mapping

Alternatives considered:
1. Separate database per context (full isolation)
2. Shared chunks with many-to-many mapping (chosen)
3. Copy-on-write chunks

Rationale:
- Deduplicates identical content across contexts (saves storage and compute)
- Single database simplifies management and backup
- Enables efficient garbage collection
- Critical for git worktrees where most content is identical
- Many-to-many mapping provides clean isolation while sharing data

Implications:
- Must track chunk references across contexts
- GC required to clean up orphaned chunks
- Context removal doesn't immediately free storage
- Slightly more complex schema than isolated databases

## Notes

- All text is UTF-8 encoded
- Embedding dimensions are fixed at index creation time (stored in metadata)
- Cosine similarity is computed in SQLite using sqlite-vec functions
- Database schema version must be tracked for migrations
- ForestRelativePath enables portable index files (can move forest root)
- Boost rules are applied after similarity search, not during
- Chunk deduplication uses content hash to generate deterministic chunk_id
- Context isolation is logical (via context_chunks table), not physical
- GC should be run periodically to reclaim storage from removed contexts
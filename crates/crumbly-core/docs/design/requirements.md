# Crumbly Core - Requirements Specification

## Overview

This specification defines the functional and non-functional requirements for Crumbly Core, a semantic search library for documentation in large codebases.

## Functional Requirements

### SC-1: Knowledge Index Initialization

WHEN a user creates a KnowledgeIndex with a database path and content directories  
THEN the system SHALL initialize a SQLite database with vector search capabilities

WHERE the database does not exist  
THEN the system SHALL create the database schema with tables for chunks, embeddings, file metadata, and contexts

### SC-2: File Discovery

WHEN the indexing process starts  
THEN the system SHALL recursively scan configured directories for supported file types

WHERE a file has a supported extension (.md, .rs)  
THEN the system SHALL add the file to the processing queue

WHERE a file is in an ignored directory (target/, .git/, node_modules/)  
THEN the system SHALL skip the file

WHERE a .gitignore file exists in a directory  
THEN the system SHALL respect .gitignore patterns when discovering files

WHERE a .crumblyignore file exists in a directory  
THEN the system SHALL respect .crumblyignore patterns when discovering files

### SC-3: Content Chunking

WHEN processing a Markdown file  
THEN the system SHALL split content at heading boundaries (# headers)

WHERE a section exceeds maximum chunk size  
THEN the system SHALL further split on paragraph boundaries

WHEN processing a Rust source file  
THEN the system SHALL extract doc comments (///, //!, /** /)

WHERE doc comments are associated with code elements  
THEN the system SHALL preserve the association in chunk metadata

### SC-4: Embedding Generation

WHEN a chunk is created  
THEN the system SHALL generate a vector embedding using the configured model

WHERE embedding generation fails  
THEN the system SHALL log the error and continue processing other chunks

### SC-5: Vector Storage

WHEN an embedding is generated  
THEN the system SHALL store the vector in the database with associated metadata

WHERE metadata includes: source file path, chunk text, byte offset, and content type  
THEN the system SHALL persist all metadata fields

### SC-6: Semantic Search

WHEN a user submits a search query  
THEN the system SHALL convert the query to a vector embedding

WHEN the query embedding is generated  
THEN the system SHALL perform vector similarity search against stored embeddings

WHERE a limit parameter is provided  
THEN the system SHALL return at most that many results

WHERE no limit is specified  
THEN the system SHALL return a default number of results (10)

### SC-7: Search Results

WHEN returning search results  
THEN the system SHALL include: chunk text, source file path, similarity score, and chunk metadata

WHERE results are ordered by similarity score  
THEN the system SHALL return highest similarity first (descending order)

### SC-8: Incremental Updates

WHEN the index is updated  
THEN the system SHALL check file modification timestamps

WHERE a file has not been modified since last indexing  
THEN the system SHALL skip reprocessing that file

WHERE a file has been modified  
THEN the system SHALL remove old chunks and reprocess the file

WHERE a previously indexed file no longer exists  
THEN the system SHALL remove its chunks from the database

### SC-9: Batch Processing

WHEN indexing multiple files  
THEN the system SHALL process files in batches to manage memory usage

WHERE embedding generation supports batching  
THEN the system SHALL batch multiple chunks for efficient processing

### SC-10: Content-Addressed Storage

WHEN indexing a file  
THEN the system SHALL track files by content hash rather than path

WHERE multiple files have identical content  
THEN the system SHALL deduplicate chunks across those files

WHEN updating the index  
THEN the system SHALL only re-index files whose content hash has changed

### SC-11: Context Registration

WHEN a directory is indexed for the first time  
THEN the system SHALL register a context with a unique identifier

WHERE the context identifier is derived from a canonicalized relative path  
THEN the system SHALL store the context in the database

WHERE no context is specified  
THEN the system SHALL use the default context "." (workspace root)

### SC-12: Context Isolation

WHEN indexing files within a context  
THEN the system SHALL associate all file mappings with that context

WHERE multiple contexts exist  
THEN the system SHALL maintain separate file-to-chunk mappings for each context

WHEN searching within a context  
THEN the system SHALL only return results from files associated with that context

### SC-13: Chunk Sharing

WHEN a chunk is created from file content  
THEN the system SHALL deduplicate chunks with identical content across all contexts

WHERE multiple contexts reference the same chunk  
THEN the system SHALL store the chunk once and maintain multiple context references

### SC-14: Context Listing

WHEN a user requests a list of contexts  
THEN the system SHALL return all registered context identifiers

WHERE each context has associated metadata  
THEN the system SHALL include file count and last indexed timestamp

### SC-15: Context Removal

WHEN a user removes a context  
THEN the system SHALL delete all file mappings associated with that context

WHERE chunks are no longer referenced by any context  
THEN the system SHALL mark those chunks as orphaned

### SC-16: Garbage Collection

WHEN a user initiates garbage collection  
THEN the system SHALL identify chunks not referenced by any context

WHERE orphaned chunks are found  
THEN the system SHALL remove those chunks and their embeddings from the database

### SC-17: Context Command Support

WHEN executing indexing or search commands  
THEN the system SHALL accept a --context flag to specify the target context

WHERE no context flag is provided  
THEN the system SHALL operate on the default context "."

## Non-Functional Requirements

### SC-NFR-1: Performance - Search Latency

WHILE performing semantic search on an index with up to 100,000 chunks  
THEN the system SHALL return results within 500 milliseconds

### SC-NFR-3: Resource Usage

WHILE the embedding model is loaded  
THEN the system SHALL use no more than 500MB of RAM

### SC-NFR-4: Storage Efficiency

WHILE storing embeddings and metadata  
THEN the system SHALL use no more than 2KB per chunk on average

### SC-NFR-5: Portability

WHILE the library is deployed  
THEN the system SHALL run on Linux, macOS, and Windows without external dependencies

WHERE vector search is required  
THEN the system SHALL use only SQLite with sqlite-vec extension (no separate vector database)

### SC-NFR-6: Testability

WHILE developing and testing  
THEN the system SHALL support dependency injection for all external integrations

WHERE tests require controlled behavior  
THEN the system SHALL provide trait-based abstractions for filesystem, embedding models, and storage

## Error Handling

### SC-ERR-1: Database Initialization Failure

WHILE initializing the knowledge index  
WHERE database creation fails  
THEN the system SHALL return an error describing the failure cause

### SC-ERR-2: File Read Errors

WHILE processing files  
WHERE a file cannot be read due to permissions or I/O errors  
THEN the system SHALL log the error and continue processing remaining files

### SC-ERR-3: Embedding Model Loading

WHILE initializing the embedding model  
WHERE model files are missing or corrupted  
THEN the system SHALL return an error indicating the model cannot be loaded

### SC-ERR-4: Invalid Query

WHILE processing a search query  
WHERE the query is empty or exceeds maximum length  
THEN the system SHALL return an error indicating invalid query parameters

### SC-ERR-5: Database Corruption

WHILE performing database operations  
WHERE the database file is corrupted  
THEN the system SHALL return an error indicating database corruption and suggest recovery options

### SC-ERR-6: Invalid Context

WHILE operating on a specified context  
WHERE the context does not exist  
THEN the system SHALL return an error indicating the context is not registered

## Appendix A: Chunk Metadata Schema

rust
struct ChunkMetadata {
    id: ChunkId,              // Unique identifier
    source_path: PathBuf,     // Source file path
    content_type: ContentType, // Markdown, RustDoc, etc.
    byte_offset: usize,       // Position in source file
    heading_path: Vec<String>, // Hierarchical heading context (for Markdown)
    item_path: Option<String>, // Rust item path (for RustDoc)
}


## Appendix B: Search Result Schema

rust
struct SearchResult {
    chunk_id: ChunkId,
    text: String,
    source_path: PathBuf,
    similarity_score: f32,    // 0.0 to 1.0
    metadata: ChunkMetadata,
}


## Appendix C: Context Schema

rust
struct Context {
    id: ContextId,            // Unique identifier (canonicalized relative path)
    created_at: DateTime,     // Context registration timestamp
    last_indexed: DateTime,   // Last indexing operation timestamp
    file_count: usize,        // Number of files tracked in this context
}


## Notes

- Requirements use "SC" prefix (Crumbly Core)
- Vector similarity uses cosine similarity metric
- Embedding model must be deterministic for reproducible results
- Database schema must support efficient similarity search with sqlite-vec
- Contexts enable multiple working directories (like git worktrees) to share a single embedding database while maintaining isolated file mappings
- Default context is "." representing the workspace root
- Chunks are content-addressed and deduplicated across all contexts
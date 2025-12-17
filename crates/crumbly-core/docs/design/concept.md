━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━


feature: 0002-crumbly-core
status: proposed
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━



# Crumbly Core - Semantic Search Library

## Problem

Large codebases accumulate documentation across multiple locations—README files, inline comments, API documentation, design documents, and architectural decision records. Developers waste significant time searching for information because traditional keyword-based search fails when they don't know the exact terminology used in the documentation. A developer searching for "configuration management" might miss relevant documentation that uses terms like "settings," "preferences," or "parameters."

This problem compounds as codebases grow. The knowledge required to effectively develop Bottlerocket is vast, and documentation is sparse and organized ad-hoc. Developers usually must consult tenured developers when they should be able to find answers in documentation or source code. Critical architectural decisions remain hidden in scattered documents. The knowledge exists, but it's effectively invisible without the right search terms.

AI agents miss information or find just enough to be partially correct when answering complex Bottlerocket questions. Semantic search helps them find relevant source documents.

## Solution

Crumbly Core will provide a semantic search library that understands the meaning of queries, not just keyword matches. By leveraging machine learning embeddings, the system will find relevant documentation even when queries use different terminology than the source material.

The library will handle all components required in semantically searching documentation: scanning documentation files, intelligently chunking content to preserve context, generating vector embeddings, storing them efficiently, and providing fast semantic search. Developers will query using natural language, and the system will return the most semantically relevant documentation chunks ranked by similarity.

## How It Works

A developer initializes a knowledge index pointing to their documentation directories. The system scans for supported file types (Markdown, Rust source files with doc comments) and processes each file using content-aware chunking strategies. Markdown files are split at heading boundaries to preserve document structure. Rust files extract doc comments while maintaining their association with code elements.

Each chunk is converted to a vector embedding using a lightweight ML model that runs locally—no external API calls required. These embeddings capture the semantic meaning of the text in high-dimensional space. The vectors are stored in a SQLite database with the sqlite-vec extension for efficient similarity search.

When a developer searches, their query is converted to the same embedding space. The system performs a vector similarity search to find chunks whose meaning is closest to the query, regardless of exact word matches. Results include the original text, source file location, and relevance scores. The index supports incremental updates, so only modified files are reprocessed.

The system supports multiple contexts, enabling git worktrees to share a single embedding database while maintaining isolated file mappings. Each context has a unique identifier representing its working directory (e.g., "." for the workspace root or "worktrees/feature-a" for a worktree). Chunks are deduplicated across contexts—identical content is stored once but can be referenced by multiple contexts. This is critical for AI agents working with git worktrees, allowing them to maintain separate indexes for different branches without duplicating the entire embedding database. Developers can use the --context flag to operate on specific contexts, list registered contexts, or remove unused ones. Garbage collection removes orphaned chunks not referenced by any context.

## Benefits

Developers will find relevant documentation faster, even when they don't know the exact terminology. A search for "error handling patterns" will surface documentation about "failure recovery strategies" or "exception management" because the semantic meaning aligns.

New team members will onboard more quickly by discovering architectural context and design rationale through natural language queries. The barrier to finding information drops significantly when you don't need to guess the right keywords.

Documentation becomes more valuable because it's actually discoverable. Teams will be incentivized to write better documentation knowing it will be found when needed.

## Technical Notes

Embedding generation will use a small, efficient model (like all-MiniLM-L6-v2) that runs locally without external dependencies. The model must be fast enough for interactive search while providing sufficient semantic understanding.

Storage will use SQLite with the sqlite-vec extension for vector similarity search. This provides a single-file database with no separate server process, simplifying deployment and reducing operational complexity.

Chunking strategies will be pluggable to support different content types. The system must preserve semantic coherence—chunks should represent complete thoughts or concepts, not arbitrary text windows.
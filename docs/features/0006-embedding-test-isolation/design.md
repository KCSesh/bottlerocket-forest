# Embedding Test Isolation - Technical Design

## Overview

This design adds dependency injection for embedding providers to KnowledgeIndex, enabling unit tests to run without network access or model downloads.
The approach adds an optional `EmbeddingFactory` field that tests can populate via a `#[cfg(test)]` constructor, while production code continues using the existing `open()` API unchanged.
Internal methods `create_search_engine()` and `create_provider()` check for an injected factory before falling back to real model loading.

## Critical Constraints

| ID | Constraint | Rationale | Anti-pattern |
|----|------------|-----------|--------------|
| CC-1 | Existing `open()`/`open_with_config()` API unchanged | Backward compatibility for all callers | Adding required parameters to public constructors |
| CC-2 | `tests/embedding_integration.rs` must use real embeddings | Integration tests validate actual model behavior | Mocking in integration tests |
| CC-3 | Mock dimension must match `EmbeddingModelConfig.embedding_dim` (384) | Dimension mismatch causes runtime errors | Hardcoding different dimension in mock |
| CC-4 | Mock must implement `embed_batch()` and `model_name()` | Full trait coverage required for all code paths | Partial mock that panics on unused methods |

## Module Structure

```
crates/crumbly-core/src/knowledge/facade/
├── mod.rs                 # KnowledgeIndex with new embedding_factory field
├── inner/
│   └── mod.rs             # Updated create_search_engine/create_provider to use factory
└── test_helpers.rs        # create_mock_provider() helper for tests
```

**Changes by file:**

- `facade/mod.rs`: Add `embedding_factory: Option<EmbeddingFactory>` field, add `#[cfg(test)] open_with_mock()` constructor
- `facade/inner/mod.rs`: Update `create_search_engine()` and `create_provider()` to check `index.embedding_factory` before loading real model
- `facade/test_helpers.rs`: Add `create_mock_provider()` returning configured `MockEmbeddingProvider`
- `Makefile`: Add `HF_HOME` cache directory to `integ` target environment

## Implementation Guidance

- Reference CC-1: Do not modify signatures of `open()` or `open_with_config()`
- Reference CC-3: Mock provider must return vectors of length 384
- Use `mockall::automock` attribute already present on `EmbeddingProvider` trait
- Factory type: `Box<dyn Fn() -> Box<dyn EmbeddingProvider> + Send + Sync>`

# Git Batch Fetch - Technical Design

## Overview

This design introduces a `ContentSource` abstraction that decouples the indexer from filesystem access, enabling efficient batch content retrieval from bare git repositories via `git cat-file --batch`. The indexer strategies are generalized to work with any content source while maintaining backward compatibility with existing filesystem-based indexing and incremental update support.

## Critical Constraints

| ID | Constraint | Rationale | Anti-pattern |
|----|------------|-----------|-------------|
| CC-1 | Backward compatible - existing filesystem indexing must continue working | Users depend on current behavior | Breaking FilesystemSource or removing it |
| CC-2 | Incremental indexing must still function | Performance requirement for large repos | Removing last_modified tracking or always re-indexing |
| CC-3 | Partial batch failures should not fail entire operation | Resilience - one bad file shouldn't block indexing | Using `?` to propagate single-file errors in batch operations |
| CC-4 | Must handle missing files, empty files, UTF-8 errors gracefully | Real repos have edge cases | Panicking or failing batch on individual file issues |

## Module Structure

```
crates/crumbly-core/src/
├── source/
│   ├── mod.rs              # ContentSource trait, ContentEntry, FetchResult types
│   ├── filesystem.rs       # FilesystemSource impl (existing behavior)
│   └── bare_git.rs         # BareGitSource impl with batch_fetch
└── indexer/
    ├── mod.rs              # Public API, re-exports
    ├── operations.rs       # chunk_content() (renamed from chunk_file)
    └── strategies.rs       # Generalized to use ContentSource
```

### Key Type Changes

**source/mod.rs**
- `ContentEntry`: Add `last_modified: Option<SystemTime>` field
- `ContentSource` trait: Add `batch_fetch(&self, entries: &[ContentEntry]) -> Vec<FetchResult>`
- `FetchResult`: New type wrapping `Result<(ContentEntry, String), FetchError>` per-file

**source/bare_git.rs**
- `BareGitSource::batch_fetch()`: Spawn single `git cat-file --batch` process, write all blob IDs, parse streamed output

**indexer/strategies.rs**
- Accept `&dyn ContentSource` instead of `&FileScanner`
- Use `source.scan()` then `source.batch_fetch()` for content retrieval
- Handle `FetchResult` errors individually (log and skip, don't fail batch)

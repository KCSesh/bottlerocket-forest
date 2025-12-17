# Crumbly

Crumbly is a semantic search tool for exploring documentation using ML embeddings. It provides fast, targeted documentation lookup across large codebases.

## Purpose

Crumbly enables semantic search across documentation, making it easy to find relevant information even when you don't know the exact keywords. It uses machine learning embeddings to understand the meaning of your queries and match them with relevant documentation.

## Installation

Build from source:

```bash
cd bottlerocket-forest
cargo build --release -p crumbly-cli
```

The binary will be at `target/release/crumbly`.

## Usage

Build the index (first time or after major changes):

```bash
crumbly build
```

Search the documentation:

```bash
crumbly search "how to build a kit"
crumbly search "boot process" --limit 5
crumbly search "systemd configuration" --show-chunks
```

Check index status:

```bash
crumbly status
```

Update incrementally (faster than full rebuild):

```bash
crumbly update
```

Rebuild from scratch:

```bash
crumbly rebuild
```

Clear the index:

```bash
crumbly clear
```

## How It Works

Crumbly uses the `sentence-transformers/all-MiniLM-L6-v2` model for embeddings and stores data in `.crumbly/knowledge/` at the index root. It automatically discovers documentation from all repositories and supports markdown files, Rust source files, and other text formats.

The index downloads ML models automatically on first use (~90MB for the embedding model).

## Configuration

Crumbly looks for a `crumbly.toml` configuration file in the index root to determine which repositories and paths to index. If not found, it uses sensible defaults.

## Library Usage

Crumbly is built on `crumbly-core`, a reusable library for semantic search. You can use it in your own projects:

```toml
[dependencies]
crumbly-core = "0.1.0"
```

See `crates/crumbly-core/` for library documentation.

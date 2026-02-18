# Chunking

Documents are often too large to search effectively as a single unit.
A 500-line README might discuss installation, configuration, troubleshooting, and API reference—all very different topics.
If you search for "installation", you don't want the entire file; you want the relevant section.

Chunking solves this by splitting documents into smaller, focused pieces.
Each chunk becomes independently searchable, so queries match the most relevant portions of your content.

Chunk sizes are constrained by what the embedding model supports.
The model has a maximum input length, so chunks must stay within that limit to be embedded properly.

## How Chunking Works

Crumbly uses different chunking strategies depending on file type:

```
     .md files     ──▶   MarkdownChunker    ──▶   Heading sections
     .rs files     ──▶   RustDocChunker     ──▶   Doc comments
     .go files     ──▶   GoDocChunker       ──▶   Doc comments
     .java files   ──▶   JavaDocChunker     ──▶   Doc comments
     .c/.h files   ──▶   CDocChunker        ──▶   Doc comments
     .sh files     ──▶   ShellChunker       ──▶   Doc comments
     .js files     ──▶   JsDocChunker       ──▶   Doc comments
```

## Markdown Chunking

Crumbly splits markdown files by heading structure.
Each heading starts a new chunk that includes all content until the next heading of equal or higher level.

Consider this document:

```markdown
# Getting Started

Welcome to the project.

## Installation

Run `cargo install myapp`.

## Configuration

Create a config file...

### Advanced Options

For power users...
```

This produces three chunks:

1. **"Getting Started"** — The intro paragraph
2. **"Installation"** — The install instructions
3. **"Configuration > Advanced Options"** — Config content plus the nested advanced section

Each chunk preserves its heading hierarchy, so search results show context like "Configuration > Advanced Options" rather than just "Advanced Options".

## Rust Doc Comments

For Rust files, crumbly extracts documentation comments rather than code.
This focuses search on the explanatory content that developers write to describe their APIs.

```rust
/// Validates user input against the schema.
///
/// Returns `Ok(())` if valid, or an error describing
/// what failed validation.
///
/// # Examples
///
/// ```
/// let result = validate(&input, &schema);
/// assert!(result.is_ok());
/// ```
pub fn validate(input: &str, schema: &Schema) -> Result<()> {
    // ...
}
```

Crumbly extracts the doc comment as a searchable chunk, tagged with the item name (`validate`) and type (`function`).
The function body is not indexed—only the documentation.

### Filtering Rust Chunks

Not all documented items are equally useful for search.
You might want to focus on public APIs and skip internal helpers, or filter out trivial one-liner comments that add noise to results.

Crumbly lets you control what gets indexed:

```toml
[file-types.rust]
visibility = ["public"]
items = ["function", "struct", "trait", "enum", "module"]
min-doc-lines = 2
```

**visibility** — Which items to index based on visibility: `public`, `private`, `crate`, `restricted`.
Default: all.

**items** — Which item types to index: `function`, `struct`, `trait`, `enum`, `module`, `const`, `static`, `type`.
Default: all.

**min-doc-lines** — Skip items with fewer than N lines of documentation.
Filters out trivial one-liner comments.
Default: `1`.

## Go Doc Comments

Go uses a similar doc comment convention, so crumbly applies the same extraction approach.
Doc comments preceding functions, types, and packages become searchable chunks:

```go
// ParseConfig reads a configuration file and returns
// the parsed settings. It supports TOML and YAML formats.
//
// If the file doesn't exist, it returns default settings
// rather than an error.
func ParseConfig(path string) (*Config, error) {
    // ...
}
```

This doc comment becomes a chunk associated with the `ParseConfig` function.

### Filtering Go Chunks

```toml
[file-types.go]
visibility = ["exported"]
items = ["function", "type", "package"]
min-doc-lines = 2
```

**visibility** — `exported` (capitalized names) or `unexported`.
Default: all.

**items** — `function`, `type`, `package`, `const`, `var`.
Default: all.

**min-doc-lines** — Same as Rust.
Default: `1`.

## Enabling File Types

By default, crumbly only indexes markdown files.
Enable additional file types in your `crumbly.toml`:

```toml
enabled-file-types = ["markdown", "rust", "go", "java", "c", "shell", "javascript"]
```

## Chunk Size Controls

Two settings control how large chunks can be:

```toml
max-tokens = 256
```

**max-tokens** sets the maximum size of a chunk in tokens.
This limit exists because the embedding model has a maximum input length—chunks exceeding it cannot be embedded.
If a markdown section exceeds this limit, crumbly splits it into multiple chunks.

The default (256) is tuned for the built-in embedding model.
You typically don't need to change it unless you're using a different model with a different context window.

## What's Next

Chunking produces bite-sized pieces of your documentation.
The next step is [Embedding](embedding.md), where crumbly converts those chunks into vectors that capture their semantic meaning.

# Contexts

A context is a named scope for indexing and searching.
When you build an index, you specify which directory to index as a context.
When you search, results come from that context.

## Index Location

Crumbly stores its index in a `.crumbly` directory.
For contexts to work properly, this index must be in a parent directory of all the contexts you want to index.

A typical setup:

```
my-workspace/
├── .crumbly/          # Index lives here
├── main/              # Main worktree (a context)
├── feature-a/         # Feature worktree (a context)
└── feature-b/         # Another worktree (a context)
```

All three worktrees share the same index because `.crumbly` is in their common parent.

## Content-Addressable Storage

Crumbly uses content-addressable storage internally.
Identical content is stored only once, regardless of how many contexts contain it.

This design shines when you work with git worktrees.
Most files are identical across worktrees, so adding a new worktree as a context indexes almost instantly—only the files that differ need new embeddings computed.

Many developers have found that AI agents work well with worktrees.
Each task gets its own working directory without branch switching overhead.

## Creating a Context

Build a context:

```bash
crumbly build --context ./main
```

The context name is derived from the path.

Update an existing context after files change:

```bash
crumbly update --context ./main
```

This re-indexes only files that have changed since the last build.

## Worktree Workflow

A typical workflow with worktrees:

```bash
# From the workspace root (where .crumbly lives)
crumbly build --context ./main

# Create a worktree for a feature
cd main
git worktree add ../feature-x -b feature-x
cd ..

# Index it — fast, since most content is shared
crumbly build --context ./feature-x
```

The second build completes quickly because crumbly recognizes that most chunks already exist.

## Searching

Crumbly automatically discovers which context you're in by walking up the directory tree to find a `.crumbly` index.
It then determines if your current working directory falls within any indexed context.

```bash
cd feature-x
crumbly search "authentication"
```

This finds the index in `../`, recognizes you're in the `feature-x` context, and searches within it.

You can also specify a context explicitly:

```bash
crumbly search --context ./main "authentication"
```

## Listing Contexts

See what contexts exist:

```bash
crumbly context list
```

This shows each context with its chunk count and last update time.

## What's Next

The [Pipeline Overview](pipeline-overview.md) explains how crumbly transforms your documents into searchable chunks.

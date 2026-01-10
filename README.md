# Bottlerocket Forest

A meta-repository for Bottlerocket development.

## 🤖 For AI Agents

**Read [AGENTS.md](./AGENTS.md) first** - it contains the mandatory workflow and detailed guidance.

## Purpose

The forest provides:
- Organized access to all Bottlerocket component repositories
- Skills for AI agents to perform common development workflows
- Tooling to simplify local development and testing
- High-level documentation for understanding the Bottlerocket ecosystem

## Layout

```
bottlerocket-forest/
├── crates/                    # Rust workspace
│   ├── forester/              # Generic forest management CLI
│   └── brdev/                 # Bottlerocket-specific dev tooling
├── worktrees/                 # Forest worktrees for doing work on bottlerocket
├── skills/                    # AI agent skills for common workflows
├── docs/                      # High-level Bottlerocket documentation
└── planning/                  # Scratch space for notes and planning (gitignored)
```

## Crumbly

Semantic search tool for exploring Bottlerocket documentation. **Must run from forest root directory.**

Install via:
```bash
cargo install crumbly
```

Usage:
```bash
crumbly build                    # Build search index
crumbly search "boot process"    # Search documentation
crumbly status                   # Check index status
crumbly update                   # Update index incrementally
crumbly rebuild                  # Rebuild from scratch
```

Crumbly is a standalone open-source tool that can be applied to any codebase.

## Forester

Generic forest management tool. **Must run from forest root directory.**

Install via:
```bash
cargo install forester
```

Usage:
```bash
forester seed                        # Clone all member repos and set up forest
forester worktree create feature-x   # Create worktree for all member repos
forester worktree list               # List existing worktrees
forester worktree remove feature-x   # Remove a worktree
```

Forester is a standalone open-source tool that can be applied to any codebase. See `crates/forester/README.md` for complete documentation.

## brdev

Bottlerocket-specific development tooling. **Must run from forest root directory.**

```bash
brdev registry start    # Start local registry
brdev registry status   # Check registry status
brdev registry list     # List published images
```

See `crates/brdev/README.md` for complete documentation.

## Development

The forest uses a Cargo workspace for tools. Build all tools:

```bash
make build          # Build all binaries
make check          # Run unit tests, plus common checks (fmt, clippy, deny)
make integ          # Run full test suite (fmt, clippy, deny, unit tests, integ tests)
make release-build  # Build optimized binaries
```

Binaries are output to `./target/release/forester` and `./target/release/brdev`.

## Documentation Guidelines

**Add Keywords for Search:**

Include a keywords line near the top of documentation files:

```markdown
**Keywords:** primary-topic, related-term, technical-concept, component-name
```

Include 5-15 terms: technical concepts, component names, use cases, related features.
Use lowercase, comma-separated. Improves `crumbly search` discoverability.

**Example:**
```markdown
# Bottlerocket Boot Process

**Keywords:** boot, systemd, targets, preconfigured, configured, multi-user, 
fipscheck, services, dependencies, API system, bootstrap containers, settings
```

**One Sentence Per Line:**

Write each sentence on its own line in markdown files.
This makes diffs easier to review—changes to one sentence don't affect adjacent lines.

```bash
# Format a file
python3 scripts/sentence-split.py path/to/file.md --in-place
```

## Skills

The `skills/` directory contains modular workflows for common Bottlerocket development tasks. Skills are mandatory when they exist for a task. See `skills/README.md` for the complete protocol.

## Component Dependencies

When developing features:
1. Changes to kits (core-kit, kernel-kit) require building and publishing to OCI registry
2. Variants in `bottlerocket/` consume kits from OCI registries
3. Building a variant requires specifying kit versions
4. Testing requires deploying the built variant image

The forest tools help orchestrate these dependencies.

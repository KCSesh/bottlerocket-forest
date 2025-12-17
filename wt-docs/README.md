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
├── bottlerocket/              # Main Bottlerocket OS repository (variants, build configs)
├── kits/
│   ├── bottlerocket-core-kit/     # Core OS packages and dependencies
│   └── bottlerocket-kernel-kit/   # Kernel packages
├── sdk/
│   └── bottlerocket-sdk/          # Build SDK and toolchain
├── host-containers/
│   ├── bottlerocket-admin-container/   # Admin container for system access
│   └── bottlerocket-control-container/ # Control container for orchestration
├── twoliter/                  # Bottlerocket build tool
├── bottlerocket-settings-sdk/ # SDK for settings plugins
├── skills/                    # AI agent skills for common workflows
├── docs/                      # High-level Bottlerocket documentation
└── planning/                  # Scratch space for notes and planning (gitignored)
```

## Getting Started
## Sembly

Semantic search tool for exploring Bottlerocket documentation.

```bash
crumbly search "boot process"    # Search documentation
crumbly status                   # Check index status
crumbly update                   # Update index incrementally
```
## brdev

Bottlerocket-specific development tooling.

```bash
brdev registry start    # Start local registry
brdev registry status   # Check registry status
brdev registry list     # List published images
```

See `crates/brdev/README.md` for complete documentation.

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
You can also fix documents after the fact with this script:

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

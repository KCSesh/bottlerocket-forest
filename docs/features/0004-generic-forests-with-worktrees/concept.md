# Generic Forests with Worktree Support

**Status:** Concept

## Problem

The Bottlerocket forest contains multiple independent git repositories (bottlerocket, core-kit, kernel-kit, twoliter, etc.) that are intentionally NOT submodules.
This creates two problems:

1. **No coordinated worktrees** - You can't `git worktree` the forest itself, making parallel feature development awkward.

1. **Bottlerocket-specific tooling** - The current forester and seed-forest.sh are hardcoded for Bottlerocket. The forest concept is useful for any multi-repo project.

## Solution

Make forests a generic, portable concept with first-class worktree support.

### Forest Definition

A forest is defined by two files:

**`forester.toml`** - Forest members:

```toml
[forest]
name = "bottlerocket"

[[member]]
name = "bottlerocket"
remote = "git@github.com:bottlerocket-os/bottlerocket.git"
path = "bottlerocket"
default_branch = "develop"

[[member]]
name = "core-kit"
remote = "git@github.com:bottlerocket-os/bottlerocket-core-kit.git"
path = "kits/bottlerocket-core-kit"
default_branch = "develop"
```

**`crumbly.toml`** - What to index for semantic search (promoted from `.crumbly.toml`).

### Directory Structure

```
my-forest/
  forester.toml
  crumbly.toml
  .forest/
    bare/                     # Bare clones of all member repos
      bottlerocket.git/
      bottlerocket-core-kit.git/
  
  # "Main" worktree (default)
  bottlerocket/               # Worktree from .forest/bare/bottlerocket.git
  kits/bottlerocket-core-kit/
  .crumbly/                    # Sembly context for main

  worktrees/
    feature-x/
      bottlerocket/           # Worktree on feature-x branch
      kits/bottlerocket-core-kit/
      .crumbly/                # Sembly context for feature-x
```

### Commands

**`forester seed`**

1. Clone repos as bare to `.forest/bare/`
1. Create main worktree for each repo in expected locations
1. Build crumbly index

**`forester worktree create <name>`**

1. Create new worktree for each member repo
1. Assemble into `worktrees/<name>/`
1. Initialize crumbly context

**`forester worktree list`** - Show existing worktrees

**`forester worktree remove <name>`** - Clean up worktree

**`forester new <name>`** - Create new forest with templates:

- `forester.toml` template
- `crumbly.toml` template
- `skills/` with generic skills
- `AGENTS.md` template

### Tool Separation

**This repo becomes `forester` (tools monorepo):**

```
forester/
  crates/
    forester/       # Generic forest management
    crumbly-core/
    crumbly-cli/
    brdev/          # Bottlerocket-specific (registry, etc.)
```

Installed via `cargo install forester crumbly brdev`

**New `bottlerocket-forest` repo:**

```
bottlerocket-forest/
  forester.toml
  crumbly.toml
  skills/           # Bottlerocket-specific skills only
  docs/
  AGENTS.md
```

No Rust code.
Just config and documentation.

### Skill Distribution

**Built into forester (placed on `forester new`):**

- `fact-find`
- `deep-research`

**In forester repo (for tool development):**

- `idea-honing`
- `brownfield-research`
- `propose-feature-concept`
- `propose-feature-requirements`
- `propose-feature-design`
- `propose-implementation-plan`

**In bottlerocket-forest:**

- `local-registry`
- `build-kit-locally`
- `build-variant-from-local-kits`
- `test-local-twoliter`
- `update-twoliter`

## Migration Path

1. `git mv crates/forester crates/brdev`
1. Create fresh `crates/forester` for generic forest logic
1. Update Cargo workspace
1. Implement `forester seed` to replace `seed-forest.sh`
1. Implement worktree commands
1. Create `bottlerocket-forest` repo with configs
1. Move Bottlerocket-specific skills there

## Open Questions

- Repo name: keep as `forester` or rename to `forest-tools`?
- Should crumbly eventually move to its own repo?

# Forester

Generic forest management for multi-repo projects.

A forest is a collection of related git repositories that are developed together but maintained as separate repos (not submodules). Forester provides:

- Forest configuration via `forester.toml`
- Coordinated worktree management across all member repos
- Integration with sembly for semantic search

## Installation

```bash
cargo install forester
```

## Usage

### Seed a Forest

Clone all member repositories and set up the forest:

```bash
forester seed
forester seed --verbose
```

### Manage Worktrees

Create a new forest worktree (creates worktrees for all member repos):

```bash
forester worktree create feature-x
forester worktree create feature-x --branch my-branch
```

List existing worktrees:

```bash
forester worktree list
```

Remove a worktree:

```bash
forester worktree remove feature-x
forester worktree remove feature-x --force
```

## Configuration

### forester.toml

Defines the forest members:

```toml
[forest]
name = "my-project"

[[member]]
name = "main-repo"
remote = "git@github.com:org/main-repo.git"
path = "main-repo"
default_branch = "main"

[[member]]
name = "lib-repo"
remote = "git@github.com:org/lib-repo.git"
path = "libs/lib-repo"
default_branch = "develop"
```

### sembly.toml

Defines what to index for semantic search. See [sembly documentation](../sembly-cli/README.md).

## Directory Structure

After seeding:

```
my-forest/
  forester.toml
  sembly.toml
  .forest/
    bare/                     # Bare clones of all member repos
      main-repo.git/
      lib-repo.git/
  
  # Main worktree
  main-repo/
  libs/lib-repo/
  .sembly/                    # Sembly index

  worktrees/
    feature-x/
      main-repo/
      libs/lib-repo/
      .sembly/
```

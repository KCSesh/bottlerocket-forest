# Forester

Generic forest management for multi-repo projects.

A forest is a collection of related git repositories that are developed together but maintained as separate repos (not submodules). Forester provides:

- Forest configuration via `forester.toml`
- Coordinated grove management across all member repos
- Integration with crumbly for semantic search

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

### Manage Groves

Create a new forest grove (creates git worktrees for all member repos):

```bash
forester grove create feature-x
forester grove create feature-x --branch my-branch
```

List existing groves:

```bash
forester grove list
```

Remove a grove:

```bash
forester grove remove feature-x
forester grove remove feature-x --force
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

### crumbly.toml

Defines what to index for semantic search. See [crumbly documentation](../crumbly-cli/README.md).

## Directory Structure

After seeding:

```
my-forest/
  forester.toml
  crumbly.toml
  .forest/
    bare/                     # Bare clones of all member repos
      main-repo.git/
      lib-repo.git/
  
  # Main grove
  main-repo/
  libs/lib-repo/
  .crumbly/                    # Sembly index

  groves/
    feature-x/
      main-repo/
      libs/lib-repo/
      .crumbly/
```

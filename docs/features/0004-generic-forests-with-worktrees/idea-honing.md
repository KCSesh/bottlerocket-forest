# Idea Honing: Generic Forests with Worktree Support

**Date:** 2024-12-04

## Initial Idea

Two competing ideas that merged into one:

1. **Worktree Support** - AI agents work well with git worktrees. Want to develop multiple features simultaneously. But sub-repos aren't submodules (by design), so can't worktree the forest itself. Need coordinated worktrees of each sub-repo.

2. **Generic Forests** - Make forests a generic concept. `forester.toml` describes members. `forester seed` replaces `./seed-forest.sh`. Take forester and sembly anywhere.

## Key Insight

These are the same feature. Worktree support requires a forest manifest to know what repos exist. Generic forests are the foundation; worktrees are a feature on top.

## Honing Q&A

**Q: Should sembly remain hidden as `.sembly.toml`?**
A: No. Sembly is core to what makes a forest useful. Promote to `sembly.toml` (no dot). Seeding the forest builds the index by default.

**Q: Should each worktree have its own sembly index or share one?**
A: Each worktree is its own sembly context. Multi-context support already exists for this. Indexes are per-worktree (branch-specific docs), contexts share the embedding model cache.

**Q: What goes in `forester.toml`?**
A: Minimal - name, remote, path, default_branch for each member. No groups or subsets for now.

**Q: Where does registry support go?**
A: Rename current forester to `brdev` (Bottlerocket-specific dev tooling). Create new generic `forester`. Registry stays in brdev.

**Q: Should `forester.toml` specify tools to install?**
A: No, overcomplicating. Each forest's README can say what to install.

**Q: What's the repo structure going forward?**
A: This repo becomes a tools monorepo (forester, sembly, brdev). New `bottlerocket-forest` repo has just configs, docs, and Bottlerocket-specific skills.

**Q: Where do skills live?**
A: Three homes:
- Built into forester (generic, auto-placed on `forester new`): `fact-find`, `research-document`
- In forester repo (for developing the tools): `idea-honing`, `brownfield-research`, `propose-feature-*`
- In bottlerocket-forest (Bottlerocket-specific): `local-registry`, `build-kit-locally`, etc.

## Resolved Design

See CONCEPT.md

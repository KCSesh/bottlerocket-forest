markdown
---
feature: 0003-crumbly-cli
status: proposed
---

# Crumbly CLI

## Problem

While `crumbly-core` provides powerful semantic search capabilities, users need a friendly way to interact with the knowledge index from their terminal. Building and querying an index shouldn't require writing Rust code or understanding the library's internals. Developers working in Bottlerocket repositories need a tool that feels natural in their workflow—something that fits alongside `git`, `cargo`, and other command-line tools they use daily.

Without a CLI, the semantic search functionality remains locked away in library code, inaccessible to the people who would benefit most from quickly finding relevant documentation across large codebases.

## Solution

The Crumbly CLI will provide a complete command-line interface for semantic search and knowledge management. It will expose all core functionality through intuitive commands that follow familiar CLI patterns, while adding a touch of personality that makes the tool pleasant to use.

The CLI will handle index lifecycle operations (build, rebuild, update, clear), semantic search queries, status reporting, context management, and maintenance tasks. It will provide both human-friendly output with color and formatting, and machine-readable JSON output for scripting and integration.

The interface will be friendly and approachable, with thoughtful defaults that minimize required flags and arguments. A hint of whimsy in the user experience will make routine operations feel less mechanical—because even developer tools can have personality.

## How It Works

When you first clone a Bottlerocket repository, you run `crumbly build` to create the knowledge index. The CLI scans your documentation, chunks it intelligently, generates embeddings, and stores everything in `.crumbly/knowledge/` at your forest root. Progress indicators show what's happening, and the whole process completes in seconds for typical repositories.

Once the index is built, searching is simple: `crumbly search "how to build a kit"` returns relevant documentation ranked by semantic similarity. Results show file paths, relevance scores, and context snippets. If you want more detail, `--show-chunks` reveals the specific text segments that matched. Need JSON for a script? Add `--format json`.

As you work, `crumbly update` keeps the index current by processing only changed files—much faster than a full rebuild. When you want to see what's indexed, `crumbly status` shows statistics about chunks, contexts, and index health.

For repositories using git worktrees or multiple working directories, contexts let you share a single embedding database across all of them. `crumbly context list` shows registered contexts, and `crumbly context remove` cleans up contexts you no longer need. The `gc` command removes orphaned chunks that aren't referenced by any context.

Most commands accept `--forest-root` to specify where your forest lives, and `--context` to operate on a specific working directory. Sensible defaults mean you rarely need these flags—the CLI figures out what you want based on your current directory.

## Benefits

The CLI makes semantic search accessible to everyone working in Bottlerocket repositories, not just those comfortable with Rust libraries. You can find relevant documentation in seconds without remembering exact keywords or knowing which repository contains the information you need.

The tool fits naturally into existing workflows. It follows CLI conventions developers already know, with familiar flag patterns and output styles that match tools like `cargo` and `git`. The human-readable output is pleasant to read, while JSON output enables scripting and integration with other tools.

Context management enables efficient multi-worktree workflows without duplicating expensive embedding data. Update operations are fast enough to run frequently, keeping search results current as documentation evolves.

The friendly interface with subtle personality makes the tool approachable rather than intimidating. Finding documentation becomes less of a chore and more of a natural part of development.

## Technical Notes

The CLI uses `clap` for argument parsing with `clap-cargo` styling for consistent help output. Error handling uses `miette` for beautiful, actionable error messages. The `theme` module provides centralized color styling using `owo_colors`, ensuring visual consistency across all commands.

Progress reporting during indexing operations uses `indicatif` with custom templates that match the overall aesthetic. All commands support both TTY-aware colored output and plain output for non-interactive use.

The CLI is a thin layer over `crumbly-core`, delegating all semantic search logic to the library. This keeps the CLI focused on user interaction while maintaining a clean separation between interface and implementation.
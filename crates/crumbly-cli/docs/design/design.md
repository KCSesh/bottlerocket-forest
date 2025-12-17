# Feature 0003: Crumbly CLI - Technical Design

## Overview

The Crumbly CLI provides a command-line interface for semantic search and knowledge indexing. It wraps crumbly-core functionality with user-friendly commands, progress reporting, and formatted output.

Design Philosophy: The CLI is a thin presentation layer over crumbly-core. Commands map directly to domain operations with minimal transformation. The CLI handles argument parsing, user feedback, and output formatting—domain logic lives in the core library.

## Critical Constraints

| ID | Constraint | Rationale | Anti-pattern to avoid |
|----|------------|-----------|----------------------|
| CC-1 | CLI must not contain domain logic | Keeps core library reusable; CLI is just a presentation layer | Implementing indexing logic in command handlers |
| CC-2 | All commands must support --forest-root | Enables operation from any directory | Assuming current directory is always forest root |
| CC-3 | Progress reporting must be non-blocking | Large indexes take time; users need feedback | Silent operations that appear hung |
| CC-4 | Output must be machine-parseable when requested | Enables scripting and integration | Human-only output formats |

## Architecture

main.rs (clap argument parsing)
  ↓
Command handlers (index/, context/, gc.rs)
  ↓
crumbly-core (KnowledgeIndex facade)
  ↓
Domain operations


### Layer Responsibilities

CLI Layer - Argument parsing, user interaction, output formatting

Handler Layer - Command orchestration, progress reporting, error translation

Core Library - All domain logic, indexing, search (see Feature 0001 and 0002)

## Domain Model

The CLI operates on these core concepts from crumbly-core:

KnowledgeIndex
- Entry point for all operations
- Opened via KnowledgeIndex::open(forest_root)
- Provides methods for build, search, status, etc.

Context
- Represents a working directory within the forest
- Identified by relative path from forest root
- Default context is "." (forest root itself)

SearchResult
- Contains matched files and chunks
- Includes relevance scores
- Formatted for human or JSON output

## CLI Commands

### Index Management

build - Initial index creation
- Discovers repositories and documentation
- Creates .crumbly/knowledge/ directory
- Downloads ML models on first run

rebuild - Full index recreation
- Clears existing data
- Rebuilds from scratch
- Use when index is corrupted or schema changes

update - Incremental update
- Only processes changed files
- Faster than rebuild for routine updates
- Detects file modifications via timestamps

clear - Remove all indexed data
- Requires confirmation unless --yes flag
- Preserves ML models
- Useful for testing or starting fresh

### Search

search <query> - Semantic search
- Required: query string
- Optional: --limit (1-100, default 10)
- Optional: --format (human|json, default human)
- Optional: --show-chunks (show individual chunk matches)
- Optional: --context (limit search to specific context)

### Status

status - Show index statistics
- Total chunks indexed
- Number of contexts
- Index size on disk
- Last update time

### Context Management

context list - Show registered contexts
- Lists all contexts in workspace
- Marks default context

context remove <context_id> - Unregister context
- Removes context registration
- Does not delete chunks (use gc for cleanup)

### Garbage Collection

gc - Remove orphaned chunks
- Finds chunks not referenced by any context
- Reclaims disk space
- Safe to run anytime

## Module Structure

src/
├── main.rs              # Argument parsing, command dispatch
├── index/
│   ├── mod.rs          # Argument structs (BuildArgs, SearchArgs, etc.)
│   ├── handlers.rs     # Command implementations
│   ├── progress.rs     # Progress bar configuration
│   ├── formatting.rs   # Output formatting (human, JSON)
│   └── errors.rs       # CLI error types
├── context/
│   └── mod.rs          # Context list/remove handlers
├── gc.rs               # Garbage collection handler
└── theme.rs            # Color scheme and styling


## Command Handler Pattern

All handlers follow this structure:

rust
pub fn handle_command(args: CommandArgs) -> Result<(), CommandError> {
    // 1. Resolve forest root (from args or current directory)
    let forest_root = args.forest_root.unwrap_or_else(|| current_dir());
    
    // 2. Open knowledge index
    let index = KnowledgeIndex::open(&forest_root)?;
    
    // 3. Execute domain operation with progress reporting
    let result = index.operation_with_progress(|progress| {
        // Update progress bar
    })?;
    
    // 4. Format and display output
    print_formatted_output(result);
    
    Ok(())
}


Invariants:
- Handlers never implement domain logic
- All filesystem paths resolved before calling core
- Errors from core are wrapped in CLI error types with user-friendly messages
- Progress callbacks are non-blocking

## Output Formatting

### Human Format

- Uses theme.rs for consistent colors
- Relevance scores color-coded (green > 0.8, yellow > 0.6, orange > 0.4, red < 0.4)
- File paths relative to forest root
- Chunk previews with context

### JSON Format

- Machine-parseable structure
- No color codes
- Absolute paths
- All metadata included

## Progress Reporting

Uses indicatif for progress bars with three phases:

Scanning - File discovery
- Spinner animation
- File count updates
- Template: SCAN from theme::progress

Chunking - Document processing
- Progress bar with file count
- Template: CHUNK from theme::progress

Embedding - ML model inference
- Spinner with chunk count
- Template: EMBED from theme::progress

## Error Handling

CLI errors wrap core errors with additional context:

IndexError - Wraps crumbly_core::knowledge::facade::IndexError
- Adds user-friendly messages
- Suggests remediation steps
- Uses miette for diagnostic output

ContextError - Context operation failures
- Context not found
- Invalid context path
- Permission issues

All errors implement miette::Diagnostic for rich error reporting.

## Design Decisions

### DD-1: Clap for argument parsing

Decision: Use clap with derive macros

Alternatives considered:
1. Manual argument parsing
2. structopt (predecessor to clap derive)
3. clap with derive macros (chosen)

Rationale: Clap provides excellent help generation, validation, and integrates with clap-cargo for consistent styling. Derive macros reduce boilerplate while maintaining type safety.

Implications: All command arguments defined as structs with #[derive(Parser)]

### DD-2: Separate theme module

Decision: Centralize all styling in theme.rs

Alternatives considered:
1. Inline color codes in handlers
2. Per-module styling
3. Centralized theme module (chosen)

Rationale: Consistent visual identity across all commands. Easy to adjust colors globally. Matches clap-cargo aesthetic.

Implications: All output formatting uses theme::* functions

### DD-3: Progress reporting in handlers, not core

Decision: CLI layer manages progress bars

Alternatives considered:
1. Core library emits progress events
2. CLI polls core for progress
3. CLI manages progress via callbacks (chosen)

Rationale: Keeps core library UI-agnostic. Different UIs (CLI, GUI, web) can implement progress differently. Callbacks provide fine-grained control.

Implications: Core operations accept optional progress callbacks

### DD-4: Context as optional flag, not subcommand

Decision: --context flag on relevant commands

Alternatives considered:
1. crumbly context <name> search <query> (context as parent command)
2. crumbly search <query> --context <name> (context as flag, chosen)
3. Environment variable for active context

Rationale: Most operations work on default context. Flag syntax is more ergonomic for occasional use. Matches git's --git-dir pattern.

Implications: All command argument structs include context: Option<PathBuf>

## Implementation Guidance

- Keep handlers thin—delegate to crumbly-core immediately
- Use miette::Result for error propagation with rich diagnostics
- Test handlers with mock KnowledgeIndex (requires trait extraction in core)
- Progress bars should gracefully degrade when output is not a TTY
- Validate user input (limits, paths) before calling core
- Use std::env::current_dir() as fallback for --forest-root

## Testing Strategy

- Unit tests: argument parsing, output formatting, error messages
- Integration tests: end-to-end command execution with temporary forest
- Edge cases: missing forest root, invalid context paths, empty index, large result sets
- Progress reporting: verify non-blocking behavior, TTY detection

## Notes

- CLI version should match crumbly-core version
- Future enhancement: animated coffee cup spinner, flavor text during indexing
- Consider --quiet flag for scripting use cases
- JSON output format should be stable across versions
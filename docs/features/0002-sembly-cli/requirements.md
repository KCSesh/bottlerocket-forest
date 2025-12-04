# Feature 0003: Sembly CLI - Requirements Specification

## Overview

This specification defines the command-line interface for Sembly, a semantic search tool for exploring documentation using ML embeddings. The CLI provides commands for building and managing knowledge indexes, performing semantic searches, and managing contexts within a workspace.

## Functional Requirements

### CLI-1: Build Command

WHEN the user invokes sembly build  
THEN the system SHALL create a new knowledge index from discovered documentation files

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

WHERE the --context flag is provided  
THEN the system SHALL build the index for the specified context path

WHERE the --context flag is omitted  
THEN the system SHALL build the index for the workspace root context

### CLI-2: Rebuild Command

WHEN the user invokes sembly rebuild  
THEN the system SHALL clear the existing knowledge index and rebuild it from scratch

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

WHERE the --context flag is provided  
THEN the system SHALL rebuild the index for the specified context path

WHERE the --context flag is omitted  
THEN the system SHALL rebuild the index for the workspace root context

### CLI-3: Update Command

WHEN the user invokes sembly update  
THEN the system SHALL incrementally update the knowledge index with changes since the last build

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

WHERE the --context flag is provided  
THEN the system SHALL update the index for the specified context path

WHERE the --context flag is omitted  
THEN the system SHALL update the index for the workspace root context

### CLI-4: Clear Command

WHEN the user invokes sembly clear  
THEN the system SHALL prompt for confirmation before clearing all chunks from the index

WHERE the --yes or -y flag is provided  
THEN the system SHALL skip the confirmation prompt and clear the index immediately

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

WHERE the --context flag is provided  
THEN the system SHALL clear the index for the specified context path

WHERE the --context flag is omitted  
THEN the system SHALL clear the index for the current context

### CLI-5: Search Command

WHEN the user invokes sembly search <query>  
THEN the system SHALL perform a semantic search using the provided query string

WHERE the --limit or -n flag is provided with a value between 1 and 100  
THEN the system SHALL return at most the specified number of results

WHERE the --limit flag is omitted  
THEN the system SHALL return at most 10 results

WHERE the --format or -f flag is set to "json"  
THEN the system SHALL output search results in JSON format

WHERE the --format flag is set to "human" or omitted  
THEN the system SHALL output search results in human-readable format

WHERE the --show-chunks flag is provided  
THEN the system SHALL display individual chunk matches under each file in the results

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

WHERE the --context flag is provided  
THEN the system SHALL search within the specified context path

WHERE the --context flag is omitted  
THEN the system SHALL search across all contexts

### CLI-6: Status Command

WHEN the user invokes sembly status  
THEN the system SHALL display index status and statistics

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

### CLI-7: Context List Command

WHEN the user invokes sembly context list  
THEN the system SHALL display all registered contexts in the workspace

WHERE a context has the identifier "."  
THEN the system SHALL mark it as "(default)" in the output

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

WHERE no contexts are registered  
THEN the system SHALL display no output

### CLI-8: Context Remove Command

WHEN the user invokes sembly context remove <context_id>  
THEN the system SHALL remove the specified context from the workspace

WHERE the context identifier is valid  
THEN the system SHALL display a confirmation message with the removed context identifier

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

### CLI-9: Garbage Collection Command

WHEN the user invokes sembly gc  
THEN the system SHALL remove orphaned chunks not referenced by any context

WHERE the --forest-root flag is provided  
THEN the system SHALL use the specified path as the forest root

WHERE the --forest-root flag is omitted  
THEN the system SHALL use the current working directory as the forest root

### CLI-10: Version Information

WHEN the user invokes sembly --version  
THEN the system SHALL display the version number of the CLI

### CLI-11: Help Information

WHEN the user invokes sembly --help or sembly help  
THEN the system SHALL display usage information for all available commands

WHEN the user invokes sembly <command> --help  
THEN the system SHALL display detailed usage information for the specified command

## Non-Functional Requirements

### CLI-NFR-1: Output Styling

WHILE the CLI is producing output  
THEN the system SHALL use the centralized theme module for consistent color styling

WHILE displaying success indicators  
THEN the system SHALL use bright green bold text

WHILE displaying error states  
THEN the system SHALL use bright red bold text

WHILE displaying values and data  
THEN the system SHALL use cyan text

WHILE displaying labels  
THEN the system SHALL use bright white text

WHILE displaying secondary information  
THEN the system SHALL use dimmed text

WHILE displaying relevance scores  
THEN the system SHALL colorize based on thresholds: green (≥0.8), yellow (≥0.6), orange (≥0.4), red (<0.4)

### CLI-NFR-2: Index Storage

WHILE the CLI is operating  
THEN the system SHALL store the knowledge index in .sembly/knowledge/ at the forest root

### CLI-NFR-3: Error Reporting

WHILE the CLI encounters an error  
THEN the system SHALL use miette for diagnostic error reporting with helpful guidance

### CLI-NFR-4: Help Styling

WHILE displaying help information  
THEN the system SHALL use clap-cargo styling for consistent appearance with cargo-style CLIs

## Error Handling

### CLI-ERR-1: Invalid Forest Root

WHILE executing any command  
WHERE the specified or default forest root does not exist or is not accessible  
THEN the system SHALL display an error message and exit with a non-zero status code

### CLI-ERR-2: Invalid Context Identifier

WHILE executing sembly context remove  
WHERE the provided context identifier is invalid or does not exist  
THEN the system SHALL display an error message indicating the context does not exist

### CLI-ERR-3: Invalid Search Limit

WHILE executing sembly search  
WHERE the --limit value is outside the range 1-100  
THEN the system SHALL display an error message indicating the valid range

### CLI-ERR-4: Invalid Output Format

WHILE executing sembly search  
WHERE the --format value is neither "human" nor "json"  
THEN the system SHALL display an error message indicating the valid format options

### CLI-ERR-5: Knowledge Index Operation Failure

WHILE executing any command that interacts with the knowledge index  
WHERE the operation fails  
THEN the system SHALL display a diagnostic error with specific guidance and exit with a non-zero status code

## Appendix A: Search Output Format (Human)

Searching for: "how to build a kit"

Results:

  docs/building.md (score: 0.892)
    Building Kits
    This guide explains how to build Bottlerocket kits...

  docs/development.md (score: 0.745)
    Development Guide
    Learn how to develop and build components...


## Appendix B: Search Output Format (JSON)

json
{
  "query": "how to build a kit",
  "results": [
    {
      "file": "docs/building.md",
      "score": 0.892,
      "title": "Building Kits",
      "preview": "This guide explains how to build Bottlerocket kits..."
    },
    {
      "file": "docs/development.md",
      "score": 0.745,
      "title": "Development Guide",
      "preview": "Learn how to develop and build components..."
    }
  ]
}


## Appendix C: Context List Output Format

Registered contexts:
  . (default)
  worktrees/feature-a
  worktrees/feature-b


## Notes

- Context identifiers are paths relative to the forest root
- The default context is represented by "."
- All color output automatically disables when output is not a TTY
- The CLI uses clap for argument parsing with cargo-style help formatting
- Progress reporting uses indicatif with themed templates
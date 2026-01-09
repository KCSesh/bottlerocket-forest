# Agent Workflow Guide

This document contains the mandatory workflow for AI agents working in the Bottlerocket Forest.

## Important Reference Docs

If you do not have these files in context, you should probably read them:

- [ ] Read `./docs/ARCHITECTURE.md`
- [ ] Read `./docs/build-system.md`
- [ ] Read `./skills/README.md` (contains skill protocol and skill index)

### Read Skills Documentation

**REQUIRED: Read the entire skills/README.md file.** It contains:
- The skill announcement protocol (mandatory format)
- Complete index of available skills with descriptions
- When and how to use each skill

**This file contains the REQUIRED protocol you must follow when a skill applies.**

**If you skip reading skills/README.md:**
- ❌ Won't know the correct announcement format
- ❌ Won't know which skills are available
- ❌ Will reinvent tested procedures incorrectly
- ❌ Will miss validation steps

### Identify Applicable Skill

**Before responding to a user message, check if a skill exists for your task.**

Using the index in skills/README.md:
1. Determine if a skill applies to the user's request
2. If yes: You MUST announce it to the user before executing (see protocol in skills/README.md)
3. If no: Proceed with the appropriate approach (e.g., `crumbly search` for research)

**If you skip this:**
- ❌ Will reinvent tested procedures
- ❌ Will miss validation steps
- ❌ May produce inconsistent results

**⚠️ STOP: If a skill applies, announce it to the user before executing.**

Tell the user which skill you're using with `USING SKILL "skill-name"` before proceeding.
This is a user-facing checkpoint, not an internal process step.

## Multi-Step Workflows

For complex tasks with multiple steps:

1. **Use todolist functionality** if available
2. Create a task list with clear, actionable items
3. Update status as you progress
4. Mark tasks complete when verified

This helps track progress, prevents skipped steps, and provides clear status updates.

### Creating a Bottlerocket Grove

```bash
# From forest root, create a new grove
forester grove create my-feature

# This creates: ./groves/my-feature/
# With all repos checked out and ready to use
```

### Working Directory Structure

```
bottlerocket-forest/           # $FOREST_ROOT
├── groves/
│   └── my-feature/            # $GROVE_ROOT - Your working directory
│       ├── bottlerocket/
│       ├── kits/
│       │   ├── bottlerocket-core-kit/
│       │   └── bottlerocket-kernel-kit/
│       └── ...
├── docs/                      # Shared (not in groves)
├── skills/                    # Shared (not in groves)
├── planning/                  # Shared (not in groves)
└── .crumbly/                   # Shared search index
```

## Documentation Research

**ANY question about how Bottlerocket works requires research.**

Process:
1. **Read `skills/README.md`** to see the skill index
2. Check if a skill exists for your task (e.g., `fact-find` or `deep-research`)
3. If yes: Follow the protocol from skills/README.md
4. If no: Use `crumbly search` to find relevant documentation
5. Always cite sources in your response

Never guess or rely on training data for Bottlerocket-specific questions.

### Building and Testing

1. Understand the dependency chain (kit → registry → variant)
2. Use Makefile targets, not direct twoliter commands
3. Verify each step before proceeding
4. Check build artifacts exist

## Skills Protocol

**IMPORTANT: Read `skills/README.md` for the complete protocol and skill index.**

Skills are tested procedures for common tasks.

**When a skill exists for your task, you MUST use it.**

The skills/README.md file contains:
- The exact announcement format (mandatory)
- Complete three-step protocol
- Index of available skills with descriptions
- When to use each skill

**Use the index in skills/README.md to determine which skill applies to your task.**

**Do not skip reading skills/README.md** - it contains critical information not duplicated here.


# Agent Workflow Guide

This document contains the mandatory workflow for AI agents working in the Bottlerocket Forest.

## 🚨 MANDATORY WORKFLOW - START HERE

**Complete these steps IN ORDER before ANY response:**

### Step 1: Seed the Forest

```bash
./seed-forest.sh
```

**Run this every session.** It ensures:
- Sembly, forester, and brdev tools are built and available
- Knowledge index is current
- All repositories are present

**If you skip this:**
- ❌ Sembly, forester, and brdev commands will fail
- ❌ Documentation search won't work
- ❌ You'll reference outdated code

### Step 2: Read Core Documentation

```bash
cat ./docs/ARCHITECTURE.md
cat ./docs/build-system.md
```

**Always read these files.** They explain:
- How kits and variants relate
- The build system workflow
- Component dependencies
- Common development patterns

Takes 30 seconds, prevents hours of mistakes.

**If you skip this:**
- ❌ Will guess instead of citing facts
- ❌ Will misunderstand component relationships
- ❌ Will give outdated or incorrect guidance

### Step 3: Read Skills Documentation

```bash
cat skills/README.md
```

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

### Step 4: Identify Applicable Skill

**Before responding, check if a skill exists for your task.**

Using the index in skills/README.md:
1. Determine if a skill applies to the user's request
2. If yes: You MUST announce it to the user before executing (see protocol in skills/README.md)
3. If no: Proceed with the appropriate approach (e.g., `sembly search` for research)

**If you skip this:**
- ❌ Will reinvent tested procedures
- ❌ Will miss validation steps
- ❌ May produce inconsistent results

### Verification Checklist

Before answering, confirm you completed:
- [ ] Ran `./seed-forest.sh` and verified output
- [ ] Read `./docs/ARCHITECTURE.md`
- [ ] Read `./docs/build-system.md`
- [ ] Read `./skills/README.md` (contains protocol and skill index)
- [ ] Identified applicable skill (or confirmed none exists)

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

## Worktree Workflow

**Agents work in forest worktrees, not the forest root.**

A worktree is an isolated working directory containing all forest repositories. Each worktree has its own branch state, allowing parallel development without conflicts.

### Creating a Worktree

```bash
# From forest root, create a new worktree
forester worktree create my-feature

# This creates: ./worktrees/my-feature/
# With all repos checked out and ready to use
```

### Working Directory Structure

```
bottlerocket-forest/           # $FOREST_ROOT
├── worktrees/
│   └── my-feature/            # $WORKTREE_ROOT - Your working directory
│       ├── bottlerocket/
│       ├── kits/
│       │   ├── bottlerocket-core-kit/
│       │   └── bottlerocket-kernel-kit/
│       └── ...
├── docs/                      # Shared (not in worktrees)
├── skills/                    # Shared (not in worktrees)
├── planning/                  # Shared (not in worktrees)
└── .sembly/                   # Shared search index
```

### The $FOREST_ROOT Variable

`$FOREST_ROOT` should be set to the forest root directory. **Lead agents must set this variable** when spawning subagents (it is not automatically set by `seed-forest.sh`).

```bash
# Set FOREST_ROOT (lead agents do this when spawning subagents)
export FOREST_ROOT="/path/to/bottlerocket-forest"
```

Use it to access:
- Shared resources: `$FOREST_ROOT/docs/`, `$FOREST_ROOT/skills/`, `$FOREST_ROOT/planning/`

### The $WORKTREE_ROOT Variable

`$WORKTREE_ROOT` should be set to the worktree root directory. **Lead agents must set this variable** when spawning subagents (it is not automatically set by `seed-forest.sh`).

**Note:** `sembly search` must run from the **worktree root** (where the index was built), not the forest root. See Sembly Usage section.

### Accessing Shared Resources

From within a worktree, use `$FOREST_ROOT` for shared directories:

```bash
# Read documentation
cat $FOREST_ROOT/docs/ARCHITECTURE.md

# Read skills
cat $FOREST_ROOT/skills/README.md

# Create planning files
mkdir -p $FOREST_ROOT/planning/my-feature
```

### Listing and Removing Worktrees

```bash
# List existing worktrees
forester worktree list

# Remove a worktree when done
forester worktree remove my-feature
```

## Documentation Research

**ANY question about how Bottlerocket works requires research.**

Process:
1. **Read `skills/README.md`** to see the skill index
2. Check if a skill exists for your task (e.g., `research-with-citations`)
3. If yes: Follow the protocol from skills/README.md
4. If no: Use `sembly search` to find relevant documentation
5. Always cite sources in your response

Never guess or rely on training data for Bottlerocket-specific questions.

## Reading Code

**Use a tool that displays line numbers when you need accurate citations.**

This is essential when:
- Citing code in documentation or responses
- Researching existing code (brownfield development)
- Referencing specific functions or types
- Creating implementation plans with file:line references

```bash
# View file with line numbers
cat -n path/to/file.rs | head -100

# View specific line range (lines 50-100)
sed -n '50,100p' path/to/file.rs | cat -n

# Search for pattern with line numbers
grep -n "function_name" path/to/file.rs
```

**When citing code**, use the `file.rs:45-60` format and verify line numbers.


## Searching Code (ripgrep/grep)

You can run `rg` from the forest root to search across all repositories:

```bash
# Search all repos from forest root
rg "pattern" --type rust

# Search specific directory
rg "pattern" bottlerocket/sources/
```

The forest uses `.gitignore` and `.ignore` together:
- `.gitignore` excludes component repos from git (keeps `git status` clean)
- `.ignore` un-ignores them for ripgrep (enables cross-repo search)
- Each repo's own `.gitignore` excludes `target/`, `vendor/`, etc.

This gives you fast, focused searches without build artifacts.
## Sembly Usage

Sembly provides semantic search for Bottlerocket documentation.

**⚠️ CRITICAL: Run from the worktree root (where the index was built)**

The sembly index is context-specific. If built from `worktrees/develop`, searches must run from there.

```bash
# From the worktree where index was built:
cd /path/to/bottlerocket-forest/worktrees/develop
sembly search "boot process"

# Index management (also from worktree root)
sembly build      # Build search index
sembly status     # Check index status
sembly update     # Update incrementally
sembly rebuild    # Rebuild from scratch
```

## Forester Usage

Forester manages worktrees and the local OCI registry.

**⚠️ CRITICAL: Run from forest root (use subshell from worktrees)**

```bash
# Worktree management (from forest root)
forester worktree create my-feature
forester worktree list
forester worktree remove my-feature

# Registry management (from forest root or via subshell)
(cd $FOREST_ROOT && brdev registry start)
(cd $FOREST_ROOT && brdev registry status)
(cd $FOREST_ROOT && brdev registry list)
```

## Common Patterns

### Answering "How does X work?" Questions

1. Check for `research-with-citations` skill
2. Use `sembly search` to find relevant docs
3. Read the source files
4. Cite specific files and line numbers in your answer

### Making Code Changes

1. Check for applicable skills (e.g., `add-package-to-kit`)
2. Read relevant documentation first
3. Understand the component's role in the system
4. Make minimal, focused changes
5. Verify changes build successfully

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

### Common Skills

From the skills/README.md index:
- `research-with-citations` - Answer questions about Bottlerocket
- `add-package-to-kit` - Add a new package to a kit
- `update-package-version` - Update an existing package

(See skills/README.md for complete list and descriptions)

## For Orchestrating Agents

If you are an orchestrating agent that delegates work to subagents:

1. **Read skill files yourself** before delegating skill execution - you need to understand the expected output format
2. **Present subagent output verbatim** when it follows a skill protocol - do not summarize, reformat, or "improve" it
3. **Skill output formats are prescribed** - quality indicators, citation formats, and structure are part of the protocol

Delegating "use skill X" without reading the skill yourself leaves you unable to verify the output or present it correctly.

## Error Recovery

If something goes wrong:

1. **Don't guess** - Check documentation or ask for clarification
2. **Verify assumptions** - Re-read relevant docs
3. **Check build logs** - Errors often indicate what's wrong
4. **Start fresh** - Run `./seed-forest.sh` again if needed

## Best Practices

- **Always cite sources** - Reference specific files and line numbers
- **Verify before claiming** - Check that files exist, commands work, builds succeed
- **Use exact commands** - Don't paraphrase or modify tested procedures
- **Ask when uncertain** - Better to ask than to give wrong information
- **Keep responses focused** - Answer the specific question asked
- **Update documentation** - If you find gaps, note them for improvement

## Anti-Patterns to Avoid

- ❌ Skipping the mandatory workflow steps
- ❌ Guessing about Bottlerocket internals
- ❌ Ignoring available skills
- ❌ Running forester from wrong directory
- ❌ Using `twoliter` directly instead of Makefile targets
- ❌ Making changes without understanding the system
- ❌ Claiming success without verification

## Getting Help

If you're stuck:
1. Re-read the relevant documentation
2. Search for similar examples in the codebase
3. Check if a skill exists for the task
4. Ask the user for clarification

Remember: The forest is designed to help you succeed. Use the tools provided.

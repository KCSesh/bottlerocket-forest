# Agent Workflow Guide

## Git Commits
Use conventional commits for git commit messages.

## Important Reference Docs
If you do not have these files in context, read them:

- [ ] `./docs/ARCHITECTURE.md` (overview of Bottlerocket's architecture)
- [ ] `./docs/build-system.md` (overview of Bottlerocket's build system)
- [ ] `./skills/README.md` (contains skill protocol and skill index)

## Using Skills

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
1. **Read `skills/README.md`** if not in context to see the skill index
2. Check if a skill exists for your task (e.g., `fact-find` or `deep-research`)
3. If yes: Follow the protocol from skills/README.md
4. Always cite sources in your response

Never guess or rely on training data for Bottlerocket-specific questions.

### Building and Testing

1. Understand the dependency chain (kit → registry → variant)
2. Use Makefile targets, not direct twoliter commands
3. Verify each step before proceeding
4. Check build artifacts exist


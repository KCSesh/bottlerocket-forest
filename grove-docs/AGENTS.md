# Bottlerocket Grove - Agent Steering Guide

The directory containing this document (likely your cwd if you are reading this) is *not* a member of a git repository.
Despite that, you are currently working on a collection of git repositories called a `forester` "grove."

## Forester Reference

`forester` is a tool designed to create faux-monorepo structures that allow AI agents to effectively work on codebases that span multiple git repositories.
These faux-monorepos are configured in a parent repository known as a "forest".
Your current working directory is *not* a repository; however, you have many "grove members" that exist in this directory as worktrees.
Some content is also symlinked into this "grove" from the parent "forest".

When modifying content, be careful to identify which repository *does* or *should* contain the content you are modifying.
For example, only modify forest-symlinked content if you are confident that you are meant to be modifying the entire forest and not only your grove.

## Bottlerocket Grove Core Documentation

If not already present in your context, you should read these:
* [ ] ./docs/ARCHITECTURE.md
* [ ] ./docs/build-system.md

**Always read these files.** They explain:
- How kits and variants relate
- The build system workflow
- Component dependencies
- Common development patterns

**If you skip this:**
- ❌ Will guess instead of citing facts
- ❌ Will misunderstand component relationships
- ❌ Will give outdated or incorrect guidance

### Read Skills Documentation
If you do not have this in context yet, read it.

* [ ] ./skills/README.md

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

## Multi-Step Workflows

For complex tasks with multiple steps:

1. **Use todolist functionality** if available
2. Create a task list with clear, actionable items
3. Update status as you progress
4. Mark tasks complete when verified

This helps track progress, prevents skipped steps, and provides clear status updates.

## Documentation Research

**ANY question about how Bottlerocket works requires research.**

Process:
1. **Read `skills/README.md`** to see the skill index
2. Check if a skill exists for your task (e.g., `deep-research`)
3. If yes: Follow the protocol from skills/README.md
4. Always cite sources in your response

Never guess or rely on training data for Bottlerocket-specific questions.

### Answering "How does X work?" Questions

1. Use `fact-find` or `deep-research` skill
2. Use `crumbly search` to find relevant docs
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

## Delegating/Spawning Subagents

If you are an orchestrating agent that delegates work to subagents:

1. **Read skill files yourself** before delegating skill execution - you need to understand the expected output format
2. **Present subagent output verbatim** when it follows a skill protocol - do not summarize, reformat, or "improve" it
3. **Skill output formats are prescribed** - quality indicators, citation formats, and structure are part of the protocol

Delegating "use skill X" without reading the skill yourself leaves you unable to verify the output or present it correctly.

## Best Practices

- **Always cite sources** - Reference specific files and line numbers
- **Verify before claiming** - Check that files exist, commands work, builds succeed
- **Use exact commands** - Don't paraphrase or modify tested procedures
- **Ask when uncertain** - Better to ask than to give wrong information
- **Keep responses focused** - Answer the specific question asked
- **Update documentation** - If you find gaps, note them for improvement

Remember: The forest is designed to help you succeed. Use the tools provided.

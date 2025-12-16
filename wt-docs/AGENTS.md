# Agent Workflow Guide

This document contains the mandatory workflow for AI agents working in the Bottlerocket Forest.

## Read Core Documentation

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

### Read Skills Documentation
If you do not have this in context yet, read it.

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

### Skills

**Before responding to a user message, check if a skill should be loaded to respond appropriately.**

Using the index in skills/README.md:
1. Determine if a skill applies to the user's request
2. If yes: You MUST announce it to the user before executing (see protocol in skills/README.md)
3. If no: Proceed with the appropriate approach (e.g., `sembly search` for research)

**If you skip this:**
- ❌ Will reinvent tested procedures
- ❌ Will miss validation steps
- ❌ May produce inconsistent results

### Verification Checklist

Before answering user messages, confirm you completed:
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

## Sembly Usage

Sembly provides semantic search for Bottlerocket documentation.
This is *substantially more efficient* for finding information if documentation exists.
Consider executing several queries simultaneously.

```bash
sembly search "boot process"
sembly search "disk partition layout"
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

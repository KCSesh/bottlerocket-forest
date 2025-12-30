---
name: fact-find
description: Quick lookup of specific facts about Bottlerocket with citations
---

# Fact Find

Fast, focused answers to specific factual questions about Bottlerocket with proper citations.

## Purpose

Quickly find and cite concrete facts about Bottlerocket:
- Configuration values and defaults
- Partition schemes and disk layouts
- Systemd units and targets
- File paths and locations
- Version numbers and dependencies

## When to Use

- Need a specific fact, not an explanation
- Question has a concrete, definitive answer
- Looking for "what is" or "where is" information

For broader questions about architecture or design, use **deep-research** instead.

## Roles

**You (reading this file) are the orchestrator.**

| Role | Reads | Does |
|------|-------|------|
| Orchestrator (you) | SKILL.md, next-step.py output | Runs state machine, spawns subagents, writes outputs |
| State machine | progress.json, workspace files | Decides next action, validates gates |
| Subagent | Phase file (SEARCH.md or ANSWER.md) | Executes phase instructions |

⚠️ **You do NOT read files in `phases/`** — pass them to subagents via context_files. Subagents read their phase file and execute it.

## Orchestrator Loop

```
workspace = "planning/<question-slug>"
mkdir workspace
write workspace/question.txt with the user's question

while True:
    action = bash("python3 skills/fact-find/next-step.py <workspace>")
    parse action as JSON
    
    if action.type == "done":
        read workspace/FINAL.md
        present to user
        break
    
    if action.type == "gate_failed":
        report failure: action.reason
        break
    
    if action.type == "spawn":
        result = spawn(
            prompt = action.prompt,
            context_files = action.context_files,
            context_data = action.context_data,
            allow_tools = True
        )
        write result to workspace/<action.output_file>
```

## Anti-Patterns

| ❌ Don't | ✅ Do |
|----------|-------|
| Read phase files yourself | Pass phase files via context_files to subagents |
| Decide what phase is next | State machine decides via next-step.py |
| Skip gates "because it looks done" | Always validate gates |
| Store state in your memory | State lives in progress.json |

## Phases

1. **SEARCH**: Run crumbly search, identify relevant files. Subagent carries search results.
2. **ANSWER**: Read files, formulate answer with citations. Subagent carries file contents.

The orchestrator never sees search results or file contents—just the final answer.

## Inputs

- User's factual question about Bottlerocket

## Outputs

- `workspace/FINAL.md`: Concise answer with inline citations, sources section, and Research Quality Indicator

## Citation Format

The final answer uses this format:

```markdown
<Answer text with inline citations <sup>[1]</sup>.>

## Sources

<sup>[1]</sup> [`path/to/file.md`](../path/to/file.md)
- What this source provided

---

✅ **Answered from documentation** | ⚠️ **Answered from source code** | 🔍 **Partial documentation**
```

## Validation

A good fact-find response:
- ✓ Directly answers the specific question
- ✓ Concise (2-4 sentences typically)
- ✓ Superscript citations inline
- ✓ Sources section with numbered references
- ✓ Research Quality Indicator at end
- ✓ No unnecessary context or explanation

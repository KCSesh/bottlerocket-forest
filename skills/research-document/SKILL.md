---
name: research-document
description: Create educational documents that build understanding progressively with citations
---

# Research Document

A systematic approach to creating educational documentation through tiered research.

## Purpose

Creates in-depth explanatory documents that:
- Build understanding from fundamentals to specifics
- Use progressive disclosure to guide the reader
- Provide complete citations for all information
- Use visual summaries and tables for dense information

## When to Use

- User asks for comprehensive explanations of systems or features
- Need to document how components work end-to-end
- Creating educational content about architecture or processes
- Questions like "Explain how X works" or "What is the Y process?"

For quick factual lookups, use **fact-find** instead.

## Directory Structure

```
skills/research-document/
├── SKILL.md              # This file
├── next-step.py          # State machine
└── phases/
    ├── SCOUT.md          # Phase 1: discover and decompose
    ├── RESEARCH.md       # Phase 2: answer sub-questions
    ├── ASSEMBLE.md       # Phase 3: combine into document
    └── VERIFY.md         # Phase 4: check citations
```

## Workspace Layout

All artifacts go to `planning/<question-slug>/`:

```
planning/how-twoliter-builds-kits/
├── progress.json         # State machine state
├── question.txt          # Original question
├── 00-scout.md           # Scout findings + sub-questions
├── 01-kit-structure.md   # Sub-question answer
├── 02-build-command.md   # Sub-question answer
├── verify-1.txt          # Citation verification
├── verify-2.txt          # Citation verification
└── FINAL.md              # Assembled document
```

## Orchestrator Loop

The orchestrator runs the state machine and spawns subagents:

```python
import json

slug = "question-slug"  # derive from user question
workspace = f"planning/{slug}"

bash(f"mkdir -p {workspace}", on_error="raise")
write("create", f"{workspace}/question.txt", file_text=user_question)

while True:
    result = bash(f"python3 skills/research-document/next-step.py {workspace}", on_error="raise")
    action = json.loads(result)
    
    if action["type"] == "done":
        final = fs_read("Line", f"{workspace}/FINAL.md", 1, 9999)
        break
    
    if action["type"] == "gate_failed":
        log(f"Gate failed: {action['reason']}")
        break
    
    if action["type"] == "spawn":
        r = spawn(
            action["prompt"],
            context_files=action["context_files"],
            context_data=action.get("context_data"),
            allow_tools=True
        )
        write("create", f"{workspace}/{action['output_file']}", file_text=r.response)
```

## Phases

### Phase 1: Scout

Discovers what exists and formulates sub-questions. Writes `00-scout.md`.

See `phases/SCOUT.md` for subagent instructions.

### Phase 2: Research

Answers each sub-question from the scout phase. Writes `NN-*.md` files.

The state machine loops until all sub-questions are answered.

See `phases/RESEARCH.md` for subagent instructions.

### Phase 3: Assemble

Combines all research files into `FINAL.md`.

See `phases/ASSEMBLE.md` for subagent instructions.

### Phase 4: Verify

Checks each citation in `FINAL.md`. Spawns one verifier per citation.

See `phases/VERIFY.md` for subagent instructions.

## Gates

The state machine validates between phases:

| Gate | Validation |
|------|------------|
| Scout → Research | `00-scout.md` exists with sub-questions |
| Research → Assemble | Count of `NN-*.md` files matches sub-question count |
| Assemble → Verify | `FINAL.md` exists |
| Verify → Done | All citations verified |

## Research Quality Indicator

Documents end with one of:

- ✅ **Answered from documentation** - Fully answered from README files, design docs
- ⚠️ **Answered from source code** - Had to read implementation files
- 🔍 **Partial documentation** - Required both docs and source code
- ❓ **Gaps remain** - Some sub-questions could not be answered

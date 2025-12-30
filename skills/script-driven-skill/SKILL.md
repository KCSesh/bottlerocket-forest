---
name: script-driven-skill
description: Build multi-step skills using a state-machine pattern with dumb orchestrator and smart phases
---

# Script-Driven Skill Pattern

A pattern for building reliable multi-step skills where a state machine controls flow and phases are self-contained instruction files.

## Purpose

Separates concerns in complex skills:
- **Orchestrator**: Dumb loop that spawns agents and reports results
- **State machine** (`next-step.py`): Controls flow, validates gates, tracks progress
- **Phase files**: Self-contained instructions that travel with subagents

**Context isolation** is a primary motivation: the orchestrator stays lean while subagents do heavy lifting (searching, reading files, analyzing). Search results, file contents, and intermediate work stay in subagent contexts—only distilled outputs return. This keeps the orchestrator's context window clean for coordination, not cluttered with discovery. Even "simple" workflows benefit because the orchestrator never carries the baggage of discovering the answer.

## Roles

**You (reading this file) are the orchestrator.**

| Role | Reads | Does |
|------|-------|------|
| Orchestrator (you) | SKILL.md, next-step.py output | Runs state machine, spawns subagents, writes outputs |
| State machine | progress.json, workspace files | Decides next action, validates gates |
| Subagent | Phase file (e.g., SCOUT.md) | Executes phase instructions |

⚠️ **You do NOT read files in `phases/`** — pass them to subagents via context_files. Subagents read their phase file and execute it.

## When to Use

**Use for "script-like" skills** where agents must reliably perform each step in sequence.

| Skill Type | Use This Pattern? | Why |
|------------|------------------|-----|
| Script-like | ✅ Yes | Steps must execute reliably, skipping is failure |
| Documentation-like | ❌ No | Just explains concepts, no execution |
| Tool-like | ❌ No | Provides scripts/info for a domain, agent chooses what to use |

The subagent architecture has few downsides—each step gets isolated context and is highly unlikely to be skipped.

## Directory Structure

```
skills/<skill-name>/
├── SKILL.md              # Human-readable overview (you're reading one)
├── next-step.py          # State machine - returns JSON actions
├── progress.json         # Runtime state (created during execution)
└── phases/
    ├── SCOUT.md          # Phase 1 instructions (for subagents)
    ├── RESEARCH.md       # Phase 2 instructions (for subagents)
    └── ASSEMBLE.md       # Phase 3 instructions (for subagents)
```

## The Orchestrator Loop

The orchestrator is intentionally minimal. It never reads phase files—just passes them to subagents.

### Pseudocode (any agentic system)

```
workspace = "planning/<question-slug>"
create workspace directory
write user question to workspace/question.txt

loop:
    # Ask state machine what to do next
    action = run("python3 skills/<skill>/next-step.py <workspace>")
    parse action as JSON
    
    if action.type == "done":
        read workspace/FINAL.md
        break
    
    if action.type == "gate_failed":
        log "Gate failed: " + action.reason
        break
    
    if action.type == "spawn":
        # Spawn subagent with phase file - DO NOT read it yourself
        result = spawn_subagent(
            prompt = action.prompt,
            context_files = action.context_files,  # includes phase file
            context_data = action.context_data
        )
        write result to workspace/<action.output_file>
```

**Key principle:** Orchestrator has no knowledge of phases. It just executes actions.

### Python variant (run_agent_program systems)

```python
import json

workspace = f"planning/{slug}"

while True:
    result = bash(f"python3 skills/{skill}/next-step.py {workspace}", on_error="raise")
    action = json.loads(result)
    
    if action["type"] == "done":
        break
    
    if action["type"] == "spawn":
        r = spawn(
            action["prompt"],
            context_files=action["context_files"],
            context_data=action.get("context_data"),
            allow_tools=True
        )
        write("create", f"{workspace}/{action['output_file']}", file_text=r.response)
    
    if action["type"] == "gate_failed":
        log(f"Gate failed: {action['reason']}")
        break
```

## Writing next-step.py

The state machine reads `progress.json`, checks gates, and returns JSON actions.

```python
#!/usr/bin/env python3
import json
import sys
from pathlib import Path

def main():
    workspace = Path(sys.argv[1])
    progress_file = workspace / "progress.json"
    
    # Load or initialize state
    if progress_file.exists():
        state = json.loads(progress_file.read_text())
    else:
        state = {"phase": "scout", "completed": []}
    
    phase = state["phase"]
    
    # Phase: scout
    if phase == "scout":
        if "scout" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the scout phase for this research task.",
                "context_files": ["skills/my-skill/phases/SCOUT.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "00-scout.md"
            }))
            return
        # Gate: scout file must exist
        if not (workspace / "00-scout.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Scout output missing"}))
            return
        state["phase"] = "research"
        state["completed"].append("scout")
    
    # Phase: research
    if phase == "research":
        # ... similar pattern
        pass
    
    # Done
    if phase == "done":
        print(json.dumps({"type": "done"}))
        return
    
    # Save state
    progress_file.write_text(json.dumps(state, indent=2))
    
    # Recurse to get next action
    main()

if __name__ == "__main__":
    main()
```

### Action Types

| Type | Fields | Purpose |
|------|--------|---------|
| `spawn` | prompt, context_files, context_data, output_file | Run a subagent |
| `gate_failed` | reason | Stop execution, validation failed |
| `done` | - | Skill complete |

## Writing Phase Files

Phase files are self-contained instructions. The subagent receives ONLY this file (plus context_data).

```markdown
# Scout Phase

You are executing the scout phase of a research task.

## Your Goal

Discover what exists and formulate sub-questions. Do NOT answer them yet.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Question: Read from `{{workspace}}/question.txt`

## Procedure

1. Run broad searches:
   ```bash
   crumbly search "topic overview"
   ```

2. Skim top 2-3 results for structure only

3. Write findings to `{{workspace}}/00-scout.md`

## Output Format

```markdown
# Scout: <Question>

## Key Concepts
- [Concept]: [Brief note]

## Sub-Questions
1. [Question] - Type: fact-find
2. [Question] - Type: research
```

## Completion

Call `respond_to_leader("success", "Scout complete")` when done.
```

**Key principles:**
- Self-contained: agent doesn't need other files
- Explicit inputs/outputs: what to read, what to write
- Clear completion signal

## Validation Gates

Gates prevent phase skipping. Check in `next-step.py` before advancing:

```python
# Gate: all sub-questions answered
scout = json.loads((workspace / "00-scout.md").read_text())
expected = len(scout["sub_questions"])
actual = len(list(workspace.glob("[0-9][0-9]-*.md"))) - 1  # exclude 00-scout
if actual < expected:
    print(json.dumps({
        "type": "gate_failed",
        "reason": f"Only {actual}/{expected} sub-questions answered"
    }))
    return
```

## Example: Minimal 2-Phase Skill

```
skills/simple-research/
├── SKILL.md
├── next-step.py
└── phases/
    ├── GATHER.md
    └── SYNTHESIZE.md
```

**next-step.py:**
```python
#!/usr/bin/env python3
import json, sys
from pathlib import Path

workspace = Path(sys.argv[1])
progress = workspace / "progress.json"
state = json.loads(progress.read_text()) if progress.exists() else {"phase": "gather"}

if state["phase"] == "gather":
    if not (workspace / "notes.md").exists():
        print(json.dumps({
            "type": "spawn",
            "prompt": "Gather information per the phase instructions.",
            "context_files": ["skills/simple-research/phases/GATHER.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "notes.md"
        }))
    else:
        state["phase"] = "synthesize"
        progress.write_text(json.dumps(state))
        print(json.dumps({"type": "continue"}))

elif state["phase"] == "synthesize":
    if not (workspace / "FINAL.md").exists():
        print(json.dumps({
            "type": "spawn",
            "prompt": "Synthesize findings per the phase instructions.",
            "context_files": ["skills/simple-research/phases/SYNTHESIZE.md", f"{workspace}/notes.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
    else:
        state["phase"] = "done"
        progress.write_text(json.dumps(state))
        print(json.dumps({"type": "done"}))

else:
    print(json.dumps({"type": "done"}))
```

## Anti-Patterns

| ❌ Don't | ✅ Do |
|----------|-------|
| Orchestrator reads phase files | Pass phase files via context_files |
| Orchestrator decides what phase is next | State machine decides |
| Skip gates "because it looks done" | Always validate gates |
| Hardcode phase count in orchestrator | State machine knows phases |
| Store state in orchestrator variables | Store state in progress.json |

## Benefits

- **Testable**: Run `next-step.py` directly to verify state transitions
- **Resumable**: progress.json survives context resets
- **Debuggable**: Each phase's output is a file you can inspect
- **Maintainable**: Change phases without touching orchestrator

## Template for Generated Skills

Every script-driven skill MUST include these sections in SKILL.md so any agent reading it knows they are the orchestrator:

```markdown
---
name: skill-name
description: Brief description (max 256 chars)
---

# Skill Name

Brief description of what this skill accomplishes.

## Roles

**You (reading this file) are the orchestrator.**

| Role | Reads | Does |
|------|-------|------|
| Orchestrator (you) | SKILL.md, next-step.py output | Runs state machine, spawns subagents, writes outputs |
| State machine | progress.json, workspace files | Decides next action, validates gates |
| Subagent | Phase file (e.g., PHASE.md) | Executes phase instructions |

⚠️ **You do NOT read files in `phases/`** — pass them to subagents via context_files. Subagents read their phase file and execute it.

## Orchestrator Loop

workspace = "planning/<task-slug>"
mkdir workspace
write workspace/input.txt with task details

while True:
    action = bash("python3 skills/<skill>/next-step.py <workspace>")
    parse action as JSON
    
    if action.type == "done":
        read workspace/FINAL.md or final output
        break
    
    if action.type == "gate_failed":
        report failure: action.reason
        break
    
    if action.type == "spawn":
        result = spawn(
            prompt = action.prompt,
            context_files = action.context_files,  # includes phase file
            context_data = action.context_data,
            allow_tools = True
        )
        write result to workspace/<action.output_file>

## Anti-Patterns

| ❌ Don't | ✅ Do |
|----------|-------|
| Read phase files yourself | Pass phase files via context_files to subagents |
| Decide what phase is next | State machine decides via next-step.py |
| Skip gates "because it looks done" | Always validate gates |
| Store state in your memory | State lives in progress.json |

## Phases

Brief description of each phase:

1. **PHASE-NAME**: What this phase accomplishes
2. **PHASE-NAME**: What this phase accomplishes
...

## Inputs

What the orchestrator needs to gather before starting:
- Input 1
- Input 2

## Outputs

What the skill produces:
- Output location and format
```

### Key Principles for Generated Skills

1. **Self-documenting**: Any agent reading SKILL.md should immediately understand they are the orchestrator
2. **Executable**: The orchestrator loop is concrete pseudocode, not abstract description
3. **Delegating**: Emphasize that phase files go to subagents, not read by orchestrator
4. **Stateless orchestrator**: All state in progress.json, orchestrator just runs the loop


---
name: implement-commit
description: Implement commits from an implementation plan using a TDD pipeline
---

# Implement Commit Skill

Execute commits from an `implementation-plan.toml` using a mechanical TDD pipeline.

## When to Use

- Implementation plan exists at `docs/features/NNNN-name/implementation-plan.toml`
- Ready to write code

## Orchestrator Role

You are a **mechanical orchestrator**. The driver (`ic-driver.py`) handles state transitions.

You only:
1. Call driver commands
2. Execute spawn actions
3. Report results back

You MUST NOT read relevant files unless something fails - preserve your tokens, trust the `ic-driver`

## Setup

```python
import json
from pydantic import BaseModel
from typing import Literal

# Paths
feature_dir = "docs/features/NNNN-feature"
plan_path = f"{feature_dir}/implementation-plan.toml"
state_dir = ".ic"
driver = "skills/implement-commit/ic-driver.py"
workspace = "."  # From plan meta

# Response models
class PhaseResult(BaseModel):
    status: Literal["ok", "problems"]
    files_created: list[str] = []
    files_modified: list[str] = []
    notes: str | None = None

class GateDiscovery(BaseModel):
    designer_gate: str
    test_gate: str

class FormatterDiscovery(BaseModel):
    fmt_cmd: str
    lint_cmd: str

class StyleResult(BaseModel):
    status: Literal["accept", "violations"]
    violations: list[str] = []

class ArbiterResult(BaseModel):
    proceed: bool
    reasoning: str

# Agent files for spawns
agents = {
    "designer": "skills/implement-commit/agents/designer.md",
    "tester": "skills/implement-commit/agents/tester.md",
    "implementor": "skills/implement-commit/agents/implementor.md",
    "arbiter": "skills/implement-commit/agents/arbiter.md",
}
style_guides = {
    "designer": "docs/style/rust-design.md",
    "tester": "docs/style/rust-test.md",
    "implementor": "docs/style/rust-impl.md",
}
```

## Main Loop

For each commit (in dependency order):

```python
commit_id = 1  # From dependency analysis
state_path = f"{state_dir}/commit-{commit_id}.json"

# Initialize
bash(f"python {driver} init --plan {plan_path} --commit {commit_id} --state {state_path}", on_error="raise")

# Drive loop
while True:
    try:
        output = bash(f"python {driver} next --state {state_path}", on_error="raise")
    except BashError as e:
        # Driver needs LLM help - see Handling Escalation below
        handle_escalation(e, state_path)
        continue
    
    action = json.loads(output)
    
    if action["action"] == "done":
        break
    
    if action["action"] == "spawn":
        result = execute_spawn(action)
        bash(
            f"python {driver} report --state {state_path} --result '{json.dumps(result)}'",
            on_error="raise"
        )
```

## Execute Spawn

```python
def execute_spawn(action: dict) -> dict:
    phase = action.get("phase")
    
    # Determine context files
    context_files = [f"{feature_dir}/design.md"]
    if phase and phase in agents:
        context_files.append(agents[phase])
        if phase in style_guides:
            context_files.append(style_guides[phase])
    
    # Determine response model
    if action.get("response_schema") == "GateDiscovery":
        model = GateDiscovery
    elif action.get("response_schema") == "FormatterDiscovery":
        model = FormatterDiscovery
    elif action.get("style_review"):
        model = StyleResult
    elif action.get("arbiter"):
        model = ArbiterResult
    else:
        model = PhaseResult
    
    # Execute spawn
    result = spawn(
        prompt=action["prompt"],
        context_files=context_files,
        response_model=model,
        read_only=action.get("style_review") or action.get("scope_review"),
        isolate_to=Cwd(workspace) if not action.get("style_review") else None,
    )
    
    result_dict = result.parsed.model_dump()
    # Preserve action flags for driver state machine
    if action.get("arbiter"):
        result_dict["arbiter"] = True
    return result_dict
```

## Handling Escalation

When driver exits 1, it needs LLM help:

```python
def handle_escalation(e: BashError, state_path: str):
    # Parse escalation from stderr
    escalation = json.loads(e.stderr)
    error = escalation["error"]
    
    # LLM intervention - understand and fix the issue
    # This is where you THINK - the driver gave you control
    
    if "gate failed" in error.lower():
        # Could be code error or gate misconfiguration
        # Spawn a fixer or adjust gates
        spawn(
            prompt=f"Fix this gate failure: {error}",
            context_files=[...],
            isolate_to=Cwd(workspace),
        )
    elif "arbiter rejected" in error.lower():
        # Style issues too severe
        agent_feedback(f"Style review failed: {error}")
    elif "scope violations" in error.lower():
        # Out of scope changes
        agent_feedback(f"Scope violation: {error}")
    else:
        agent_feedback(f"Unknown escalation: {error}")
    
    # Resume after fix
    bash(f"python {driver} resume --state {state_path}", on_error="raise")
```

## Driver Commands

| Command | Purpose |
|---------|---------|
| `init` | Create state file for a commit |
| `next` | Get next action (spawn) or execute gate/bash directly |
| `report` | Report spawn result, advance state |
| `resume` | Continue after LLM intervention |
| `status` | Show current state (debugging) |

## State Machine

```
EXPLORE_GATES -> EXPLORE_FORMATTERS ->
DESIGNER_PHASE -> DESIGNER_GATE -> DESIGNER_STYLE ->
TESTER_PHASE -> TESTER_GATE -> TESTER_STYLE ->
IMPLEMENTOR_PHASE -> IMPLEMENTOR_GATE -> IMPLEMENTOR_STYLE ->
SCOPE_REVIEW -> FORMAT -> COMMIT -> DONE
```

- Gate failures retry the phase (up to 2x), then escalate
- Style violations loop (up to 3x), then invoke arbiter
- Driver runs gates/format/commit directly (no LLM overhead)
- Only spawns require orchestrator action

## Files

```
skills/implement-commit/
├── SKILL.md           # This file
├── ic-driver.py       # State machine driver
└── agents/
    ├── designer.md    # Creates types and signatures
    ├── tester.md      # Writes tests
    ├── implementor.md # Implements logic
    └── arbiter.md     # Decides style review proceed/fail
```

Reviewers use separate skills: `review-scope`, `review-style`.

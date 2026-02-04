---
name: implement-commit
description: Implement commits from an implementation plan using a TDD pipeline with phase isolation
---

# Implement Commit Skill

Execute commits from an `implementation-plan.toml` using a mechanical TDD pipeline.

## When to Use

- Implementation plan exists at `docs/features/NNNN-name/implementation-plan.toml`
- Ready to write code

## Orchestrator Role

You are a mechanical orchestrator. You:
1. Parse the TOML plan
2. Compute ready commits from dependency graph
3. Run each commit through phases, passing files forward
4. Never read design docs yourself—subagents do that

**You route, you don't think.** Agent files contain all instructions.

## Setup

```python
import tomllib
from pydantic import BaseModel
from typing import Literal

class PhaseResult(BaseModel):
    status: Literal["ok", "problems"]
    files_created: list[str] = []
    files_modified: list[str] = []
    notes: str | None = None

# Paths - resolve skill_dir from use_skill(), feature_dir from user
skill_dir = "/path/to/skills/implement-commit"
feature_dir = "docs/features/NNNN-feature"

# Load plan
plan = tomllib.loads(fs_read("Full", f"{feature_dir}/implementation-plan.toml"))
meta = plan["meta"]
deps = {str(k): v for k, v in plan["dependencies"].items()}
commits = {c["id"]: c for c in plan["commits"]}

# Context files
design_doc = f"{meta['feature_dir']}/design.md"

style_guides = {
    "designer": "docs/style/rust-design.md",
    "tester": "docs/style/rust-test.md",
    "implementor": "docs/style/rust-impl.md",
}

agents = {
    "designer": f"{skill_dir}/agents/designer.md",
    "tester": f"{skill_dir}/agents/tester.md",
    "implementor": f"{skill_dir}/agents/implementor.md",
}

gates = {
    "designer": "cargo check",
    "tester": "cargo test --no-run",
    "implementor": "cargo test",
}
```

## Main Loop

Process commits in dependency order. A commit is "ready" when all its dependencies are complete.

```python
completed = set()

while len(completed) < len(commits):
    ready = [c for cid, c in commits.items()
             if cid not in completed
             and all(d in completed for d in deps[str(cid)])]
    
    if not ready:
        agent_feedback("Circular dependency or blocked")
        break
    
    for commit in ready:
        run_commit_pipeline(commit)  # See phases below
        completed.add(commit["id"])
```

## Phase 1-3: Designer → Tester → Implementor

Each phase spawns an agent, runs a gate, and accumulates files for the next phase.

```python
commit_files = set(commit["files"])

for phase in ["designer", "tester", "implementor"]:
    status(f"Phase: {phase}")
    
    result = spawn(
        prompt=draft_spawn_prompt(SkillDefined(
            f"Execute {phase} phase for commit {commit['id']}: {commit['title']}"
        )),
        context_files=[
            agents[phase],
            style_guides[phase],
            design_doc,
            *[f for f in commit_files if exists(f)],
        ],
        context_data={"commit": commit, "workspace": meta["workspace"]},
        response_model=PhaseResult,
        isolate_to=Cwd(meta["workspace"])
    )
    
    if result.parsed.status == "problems":
        # Retry once with notes
        result = spawn(..., context_data={**prev, "retry_notes": result.parsed.notes})
        if result.parsed.status == "problems":
            agent_feedback(f"Phase {phase} failed: {result.parsed.notes}")
            break
    
    # Accumulate files for next phase
    commit_files.update(result.parsed.files_created)
    commit_files.update(result.parsed.files_modified)
    
    # Gate check
    bash(gates[phase], on_error="fix", cwd=meta["workspace"])
```

## Phase 4: Review (Sequential Verifiers)

Style and scope reviewers run in sequence. Style has arbiter fallback; scope is strict.

```python
MAX_STYLE_CYCLES = 3
MAX_SCOPE_CYCLES = 2

class ArbiterResult(BaseModel):
    proceed: bool
    reasoning: str
    style_debt: list[str] = []

# Create ledger for review feedback
ledger_path = f"{meta['workspace']}/.review-ledger.md"
write("create", ledger_path, file_text=f"# Review Ledger - Commit {commit['id']}\n")

# Style Review Loop (3 cycles, arbiter fallback)
for style_cycle in range(MAX_STYLE_CYCLES):
    status(f"Style review cycle {style_cycle + 1}")
    
    style_result = spawn(
        prompt=draft_spawn_prompt(SkillDefined("Review style compliance")),
        context_files=[
            "skills/review-style/SKILL.md",
            "docs/style/rust-design.md",
            "docs/style/rust-impl.md",
            "docs/style/rust-test.md",
            ledger_path,
            *commit_files,
        ],
        context_data={
            "phase": "impl",
            "changed_files": list(commit_files),
        },
        read_only=True,
    )
    
    if "VIOLATIONS:" not in style_result.response:
        break  # Style accepted
    
    # Check if cycles exhausted - invoke arbiter
    if style_cycle == MAX_STYLE_CYCLES - 1:
        arbiter = spawn(
            prompt=draft_spawn_prompt(SkillDefined("Decide if style review should proceed")),
            context_files=[f"{skill_dir}/agents/arbiter.md", ledger_path, *commit_files],
            response_model=ArbiterResult,
            read_only=True
        )
        if arbiter.parsed.proceed:
            break  # Accept with noted debt
        else:
            agent_feedback(f"Style arbiter rejected: {arbiter.parsed.reasoning}")
    
    # Return to implementor with style feedback
    status(f"Style rejected, returning to implementor")
    
    result = spawn(
        prompt=draft_spawn_prompt(SkillDefined(
            f"Fix style violations for commit {commit['id']}"
        )),
        context_files=[
            agents["implementor"],
            style_guides["implementor"],
            ledger_path,
            *commit_files,
        ],
        context_data={
            "commit": commit,
            "workspace": meta["workspace"],
            "violations": style_result.response,
        },
        response_model=PhaseResult,
        isolate_to=Cwd(meta["workspace"])
    )
    
    bash(gates["implementor"], on_error="fix", cwd=meta["workspace"])
    commit_files.update(result.parsed.files_created)
    commit_files.update(result.parsed.files_modified)

# Scope Review Loop (2 cycles, strict)
for scope_cycle in range(MAX_SCOPE_CYCLES):
    status(f"Scope review cycle {scope_cycle + 1}")
    
    scope_result = spawn(
        prompt=draft_spawn_prompt(SkillDefined("Review scope compliance")),
        context_files=[
            "skills/review-scope/SKILL.md",
            f"{feature_dir}/implementation-plan.toml",
            design_doc,
            *commit_files,
        ],
        context_data={
            "commit": commit,
            "requirements": commit.get("requirements", []),
            "constraints": commit.get("constraints", []),
            "allowed_files": list(commit_files),
        },
        read_only=True,
    )
    
    if "VIOLATIONS:" not in scope_result.response:
        break  # Scope accepted
    
    if scope_cycle == MAX_SCOPE_CYCLES - 1:
        agent_feedback(f"Scope review failed after {MAX_SCOPE_CYCLES} cycles: {scope_result.response}")
    
    # Return to implementor with scope feedback
    status(f"Scope rejected, returning to implementor")
    
    result = spawn(
        prompt=draft_spawn_prompt(SkillDefined(
            f"Fix scope violations for commit {commit['id']}"
        )),
        context_files=[
            agents["implementor"],
            style_guides["implementor"],
            *commit_files,
        ],
        context_data={
            "commit": commit,
            "workspace": meta["workspace"],
            "violations": scope_result.response,
        },
        response_model=PhaseResult,
        isolate_to=Cwd(meta["workspace"])
    )
    
    bash(gates["implementor"], on_error="fix", cwd=meta["workspace"])
    commit_files.update(result.parsed.files_created)
    commit_files.update(result.parsed.files_modified)
```

## Phase 5: Close

Format, lint, and commit.

```python
status("Close")
bash("cargo fmt", on_error="fix", cwd=meta["workspace"])
bash("cargo clippy --fix --allow-dirty", on_error="fix", cwd=meta["workspace"])
bash(f"git add -A && git commit -m '{commit['message']}'", on_error="raise", cwd=meta["workspace"])
```

## Parallel Commits

If multiple commits are ready (no dependency conflicts), use `GitWorktree` for isolation:

```python
for commit in ready:
    spawn(
        ...,
        isolate_to=GitWorktree(f"commit-{commit['id']}")
    )
```

Then merge worktrees in dependency order.

## Files

```
skills/implement-commit/
├── SKILL.md          # This file
└── agents/
    ├── designer.md   # Creates types and signatures
    ├── tester.md     # Writes tests
    ├── implementor.md # Implements logic
    └── arbiter.md    # Decides style review proceed/fail
```

Reviewers use separate skills: `review-scope`, `review-style`.

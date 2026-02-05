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

**You route, you don't think.**

DO NOT read these files yourself (subagents read them):
- design.md
- test-plan.md
- requirements.md
- Style guides

You only read: `implementation-plan.toml` and this skill file.

## Setup

```python
import tempfile
import tomllib
from pydantic import BaseModel
from typing import Literal

class PhaseResult(BaseModel):
    status: Literal["ok", "problems"]
    files_created: list[str] = []
    files_modified: list[str] = []
    notes: str | None = None

class StyleResult(BaseModel):
    status: Literal["accept", "violations"]
    violations: list[str] = []
    themes: str | None = None

class ArbiterResult(BaseModel):
    proceed: bool
    reasoning: str
    style_debt: list[str] = []

# Paths - resolve skill_dir from use_skill(), feature_dir from user
skill_dir = "/path/to/skills/implement-commit"
feature_dir = "docs/features/NNNN-feature"

# Scratch directory for ephemeral files (not in workspace)
scratch_dir = tempfile.mkdtemp(prefix="implement-commit-")

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

# Gates are discovered dynamically - see Explore Phase below

MAX_STYLE_CYCLES = 3
```

## Explore Phase: Discover Gates and Formatters

Before the main loop, spawn explorers to determine gate and formatter commands.
Gates are commands that **validate** code (compile, run tests) — they never modify files.
Formatters are commands that **modify** code (fmt, clippy --fix) — they run in an agent with commit context.
Gates depend on the project being modified — not all workspaces use cargo.

```python
class GateDiscovery(BaseModel):
    designer_gate: str    # e.g. "cargo check -p throttlesys"
    test_gate: str        # e.g. "cargo test -p throttlesys" — used by both tester (expect fail) and implementor (expect pass)

class FormatterDiscovery(BaseModel):
    fmt_cmd: str          # e.g. "cargo fmt -p throttlesys"
    lint_cmd: str         # e.g. "cargo clippy -p throttlesys --fix --allow-dirty --allow-staged"

status("Discovering gates")
explore_files = [
    f"{meta['workspace']}/Cargo.toml",
    f"{meta['workspace']}/Makefile",  # if exists
    design_doc,
]
explore_data = {
    "workspace": meta["workspace"],
    "feature": meta["feature"],
    "commit_files": [f for c in commits.values() for f in c["files"]],
}
gate_result = spawn(
    prompt=draft_spawn_prompt(SkillDefined(
        "Determine the correct build and test commands for this project. "
        "Gates are commands that VALIDATE code (compile, run tests) — they never modify files."
    )),
    context_files=explore_files,
    context_data=explore_data,
    response_model=GateDiscovery,
    read_only=True,
)

status("Discovering formatters")
fmt_result = spawn(
    prompt=draft_spawn_prompt(SkillDefined(
        "Determine the correct format and lint-fix commands for this project. "
        "Formatters are commands that MODIFY code to fix style (fmt, clippy --fix) — they change files."
    )),
    context_files=explore_files,
    context_data=explore_data,
    response_model=FormatterDiscovery,
    read_only=True,
)

gates = {
    "designer": {"cmd": gate_result.parsed.designer_gate, "expect": "pass"},
    "tester":   {"cmd": gate_result.parsed.test_gate,     "expect": "fail"},
    "implementor": {"cmd": gate_result.parsed.test_gate,  "expect": "pass"},
}
formatters = fmt_result.parsed
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

## Phase 1-3: Designer → Tester → Implementor (with Style Review)

Each phase spawns an agent, runs a gate, then runs style review with the phase-appropriate guide.

```python
commit_files = set(f"{meta['workspace']}/{f}" for f in commit["files"])

# Create ledger in scratch space (not in workspace)
ledger_path = f"{scratch_dir}/review-ledger-commit-{commit['id']}.md"
write("create", ledger_path, file_text=f"# Review Ledger - Commit {commit['id']}\n")

for phase in ["designer", "tester", "implementor"]:
    status(f"Commit {commit['id']}: {phase}")
    
    # Execute phase
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
    
    # Verify gate
    gate = gates[phase]
    try:
        bash(gate["cmd"], on_error="raise", cwd=meta["workspace"])
        gate_passed = True
    except BashError as e:
        gate_passed = False
        gate_error = str(e)

    if gate["expect"] == "pass" and not gate_passed:
        # Expected pass but failed — give phase agent the error to fix
        result = spawn(
            prompt=draft_spawn_prompt(SkillDefined(
                f"GATE FAILED - fix and retry {phase} phase for commit {commit['id']}"
            )),
            context_files=[
                agents[phase],
                style_guides[phase],
                design_doc,
                *[f for f in commit_files if exists(f)],
            ],
            context_data={"commit": commit, "workspace": meta["workspace"], "gate_error": gate_error},
            response_model=PhaseResult,
            isolate_to=Cwd(meta["workspace"])
        )
        bash(gate["cmd"], on_error="raise", cwd=meta["workspace"])
    elif gate["expect"] == "fail" and gate_passed:
        # Expected fail but passed — tests are vacuous or designer over-implemented
        agent_feedback(
            f"GATE EXPECTATION VIOLATED in {phase} phase for commit {commit['id']}: "
            f"expected tests to FAIL (TDD red) but they passed. "
            f"Tests may be vacuous or the designer phase implemented too much logic."
        )
    
    commit_files.update(result.parsed.files_created)
    commit_files.update(result.parsed.files_modified)
    
    # Style review for this phase (with retry loop)
    for style_cycle in range(MAX_STYLE_CYCLES):
        status(f"Commit {commit['id']}: {phase} style review cycle {style_cycle + 1}")
        
        style_result = spawn(
            prompt=draft_spawn_prompt(SkillDefined(f"Review {phase} phase style compliance")),
            context_files=[
                "skills/review-style/SKILL.md",
                style_guides[phase],
                ledger_path,
                *[f for f in commit_files if exists(f)],
            ],
            context_data={
                "phase": phase,
                "changed_files": list(commit_files),
            },
            response_model=StyleResult,
            read_only=True,
        )
        
        if style_result.parsed.status == "accept":
            break  # Style accepted, continue to next phase
        
        # Check if cycles exhausted - invoke arbiter
        if style_cycle == MAX_STYLE_CYCLES - 1:
            arbiter = spawn(
                prompt=draft_spawn_prompt(SkillDefined("Decide if style review should proceed")),
                context_files=[f"{skill_dir}/agents/arbiter.md", ledger_path, *[f for f in commit_files if exists(f)]],
                context_data={"violations": style_result.parsed.violations},
                response_model=ArbiterResult,
                read_only=True
            )
            if arbiter.parsed.proceed:
                break  # Accept with noted debt
            else:
                agent_feedback(f"Style arbiter rejected {phase} phase: {arbiter.parsed.reasoning}")
        
        # Return to phase agent to fix style violations
        status(f"Commit {commit['id']}: {phase} style rejected, fixing")
        
        result = spawn(
            prompt=draft_spawn_prompt(SkillDefined(
                f"Fix style violations for {phase} phase of commit {commit['id']}"
            )),
            context_files=[
                agents[phase],
                style_guides[phase],
                ledger_path,
                *[f for f in commit_files if exists(f)],
            ],
            context_data={
                "commit": commit,
                "workspace": meta["workspace"],
                "violations": style_result.parsed.violations,
            },
            response_model=PhaseResult,
            isolate_to=Cwd(meta["workspace"])
        )
        
        # Re-verify gate after style fix
        gate = gates[phase]
        try:
            bash(gate["cmd"], on_error="raise", cwd=meta["workspace"])
            gate_passed = True
        except BashError as e:
            gate_passed = False
            gate_error = str(e)

        if gate["expect"] == "pass" and not gate_passed:
            result = spawn(
                prompt=draft_spawn_prompt(SkillDefined(
                    f"GATE FAILED after style fix - retry {phase} phase for commit {commit['id']}"
                )),
                context_files=[agents[phase], style_guides[phase], ledger_path, *[f for f in commit_files if exists(f)]],
                context_data={"commit": commit, "workspace": meta["workspace"], "gate_error": gate_error},
                response_model=PhaseResult,
                isolate_to=Cwd(meta["workspace"])
            )
            bash(gate["cmd"], on_error="raise", cwd=meta["workspace"])
        elif gate["expect"] == "fail" and gate_passed:
            agent_feedback(
                f"GATE EXPECTATION VIOLATED after style fix in {phase} phase: "
                f"expected tests to FAIL but they passed."
            )
        
        commit_files.update(result.parsed.files_created)
        commit_files.update(result.parsed.files_modified)
    
    # Extend timeout after each phase completes — phases are expensive
    extend_timeout(3600)  # ensure at least 1 hour remaining
```

## Phase 4: Scope Review

Scope review runs once at the end to verify the commit stays within bounds.

```python
MAX_SCOPE_CYCLES = 2

for scope_cycle in range(MAX_SCOPE_CYCLES):
    status(f"Commit {commit['id']}: scope review cycle {scope_cycle + 1}")
    
    scope_result = spawn(
        prompt=draft_spawn_prompt(SkillDefined("Review scope compliance")),
        context_files=[
            "skills/review-scope/SKILL.md",
            f"{feature_dir}/implementation-plan.toml",
            design_doc,
            *[f for f in commit_files if exists(f)],
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
    status(f"Commit {commit['id']}: scope rejected, returning to implementor")
    
    result = spawn(
        prompt=draft_spawn_prompt(SkillDefined(
            f"Fix scope violations for commit {commit['id']}"
        )),
        context_files=[
            agents["implementor"],
            style_guides["implementor"],
            *[f for f in commit_files if exists(f)],
        ],
        context_data={
            "commit": commit,
            "workspace": meta["workspace"],
            "violations": scope_result.response,
        },
        response_model=PhaseResult,
        isolate_to=Cwd(meta["workspace"])
    )
    
    try:
        bash(gates["implementor"]["cmd"], on_error="raise", cwd=meta["workspace"])
    except BashError as e:
        result = spawn(
            prompt=draft_spawn_prompt(SkillDefined(
                f"GATE FAILED after scope fix - retry for commit {commit['id']}"
            )),
            context_files=[agents["implementor"], style_guides["implementor"], *[f for f in commit_files if exists(f)]],
            context_data={"commit": commit, "workspace": meta["workspace"], "gate_error": str(e)},
            response_model=PhaseResult,
            isolate_to=Cwd(meta["workspace"])
        )
        bash(gates["implementor"]["cmd"], on_error="raise", cwd=meta["workspace"])
    commit_files.update(result.parsed.files_created)
    commit_files.update(result.parsed.files_modified)
```

## Phase 5: Close

Run auto-formatters in an agent that has commit context, then commit.
Formatters modify code (unlike gates which only validate), so they need an agent
that understands the commit to fix issues intelligently.

```python
extend_timeout(3600)  # ensure at least 1 hour for close
```

```python
status(f"Commit {commit['id']}: close — formatting")
spawn(
    prompt=draft_spawn_prompt(SkillDefined(
        f"Run formatters and fix any issues for commit {commit['id']}: {commit['title']}. "
        f"Format command: {formatters.fmt_cmd} — Lint-fix command: {formatters.lint_cmd} — "
        f"Run each command. If they produce changes or errors, review the changes in context "
        f"of what this commit does and ensure they are correct."
    )),
    context_files=[
        design_doc,
        *[f for f in commit_files if exists(f)],
    ],
    context_data={"commit": commit, "workspace": meta["workspace"], "formatters": formatters.model_dump()},
    isolate_to=Cwd(meta["workspace"]),
)
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

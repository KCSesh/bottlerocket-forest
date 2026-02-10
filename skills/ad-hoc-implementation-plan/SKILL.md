---
name: ad-hoc-implementation-plan
description: Transform a rough plan into implement-commit ready artifacts. Allows rigorous review while implementing.
---

# Ad-Hoc Implementation Plan

Fast path from rough idea to implement-commit ready artifacts.
Trades documentation completeness for speed.

## When to Use

- You have a rough plan or idea discussed with user
- You want implement-commit's TDD verification without full propose-* chain
- The feature is well-understood enough to skip extensive design iteration

## Prerequisites

- User-approved rough plan (can be in conversation, markdown, or mental model)
- Known workspace path
- Known files to modify

## Orchestrator Role

You are a MECHANICAL ROUTER. You do NOT think about the feature.
Your job:
1. Gather minimum required information from user
2. Create feature directory structure
3. Spawn subagents with appropriate skills as context
4. Pass outputs between phases
5. Verify cross-references

## Setup

```python
from pydantic import BaseModel
from pathlib import Path
import os

class GatherResult(BaseModel):
    feature_name: str
    feature_id: str  # NNNN format
    workspace: str
    files_to_modify: list[str]
    constraints: list[str]
    test_names: list[str]
    rough_plan_summary: str

class PhaseResult(BaseModel):
    success: bool
    artifact_path: str
    errors: list[str]

# Skill paths for subagent context
skills = {
    "design": "skills/propose-feature-design/SKILL.md",
    "test_plan": "skills/propose-feature-test-plan/SKILL.md",
    "impl_plan": "skills/propose-implementation-plan/SKILL.md",
}

# Style guides (if available)
style_guides = [
    "docs/style/rust-design.md",
    "docs/style/rust-test.md",
    "docs/style/rust-impl.md",
]
```

## Phase 0: Gather

Interactively gather or infer from rough plan:

```python
# If not already known, ask user:
# 1. Feature name → creates docs/features/NNNN-name/
# 2. Files to modify → commits[].files
# 3. Key constraints → CC table entries
# 4. Test names → test-plan + commits[].tests

# Determine next feature ID
existing = list(Path("docs/features").glob("[0-9][0-9][0-9][0-9]-*"))
next_id = f"{len(existing):04d}"

feature_dir = f"docs/features/{next_id}-{feature_name}"
os.makedirs(feature_dir, exist_ok=True)
```

## Phase 1: Minimal Design

Spawn subagent with propose-feature-design skill, but instruct for MINIMAL output.

```python
result = spawn(
    prompt=draft_spawn_prompt(SkillDefined(
        "Create MINIMAL design.md following skill procedure. "
        "ONLY generate: Overview (1 paragraph), Critical Constraints table, Module Structure. "
        "SKIP: Architecture diagrams, Solution Mechanics prose, Design Patterns, Design Decisions."
    )),
    context_files=[
        skills["design"],
        *[g for g in style_guides if exists(g)],
    ],
    context_data={
        "feature_dir": feature_dir,
        "feature_name": gathered.feature_name,
        "constraints": gathered.constraints,
        "files": gathered.files_to_modify,
        "rough_plan": gathered.rough_plan_summary,
        "minimal_mode": True,
    },
    response_model=PhaseResult,
    isolate_to=Cwd(gathered.workspace)
)
```

## Phase 2: Minimal Test Plan

Spawn subagent with propose-feature-test-plan skill.

```python
result = spawn(
    prompt=draft_spawn_prompt(SkillDefined(
        "Create MINIMAL test-plan.md following skill procedure. "
        "ONLY generate: Requirements Coverage table, Critical Constraints Verification table. "
        "SKIP: Detailed test implementation notes, integration test prose."
    )),
    context_files=[
        skills["test_plan"],
        f"{feature_dir}/design.md",  # From Phase 1
        *[g for g in style_guides if exists(g)],
    ],
    context_data={
        "feature_dir": feature_dir,
        "test_names": gathered.test_names,
        "constraints": gathered.constraints,
        "minimal_mode": True,
    },
    response_model=PhaseResult,
    isolate_to=Cwd(gathered.workspace)
)
```

## Phase 3: Implementation Plan

Spawn subagent with propose-implementation-plan skill.

```python
result = spawn(
    prompt=draft_spawn_prompt(SkillDefined(
        "Create implementation-plan.toml following skill procedure. "
        "All fields required - no minimal mode. "
        "Ensure tests[] exactly matches test-plan.md test names."
    )),
    context_files=[
        skills["impl_plan"],
        f"{feature_dir}/design.md",
        f"{feature_dir}/test-plan.md",
        *[g for g in style_guides if exists(g)],
    ],
    context_data={
        "feature_dir": feature_dir,
        "workspace": gathered.workspace,
        "files": gathered.files_to_modify,
        "rough_plan": gathered.rough_plan_summary,
    },
    response_model=PhaseResult,
    isolate_to=Cwd(gathered.workspace)
)
```

## Phase 4: Verify Cross-References

Spawn verifier to check test coverage.

```python
class VerifyResult(BaseModel):
    valid: bool
    missing_tests: list[str]  # In test-plan but not in impl-plan
    orphan_tests: list[str]   # In impl-plan but not in test-plan
    missing_constraints: list[str]  # CC-N not covered

result = spawn(
    prompt=draft_spawn_prompt(AdHoc(
        intents=["Verify implementation-plan.toml is valid for implement-commit"],
        context=["Check all cross-references between artifacts"],
        goal=Verify(constraints=[
            "Every test in test-plan.md appears exactly once in commits[].tests",
            "Every CC-N in design.md has verification in test-plan.md",
            "All commits[].files exist or will be created",
            "TOML parses correctly",
        ])
    )),
    context_files=[
        f"{feature_dir}/design.md",
        f"{feature_dir}/test-plan.md",
        f"{feature_dir}/implementation-plan.toml",
    ],
    response_model=VerifyResult,
    read_only=True
)

if not result.parsed.valid:
    # Return to Phase 3 with errors
    pass
```

## Handoff

Once verified, artifacts are ready for implement-commit:

```
docs/features/NNNN-feature-name/
├── design.md              # Minimal: Overview + CC table + Module Structure
├── test-plan.md           # Minimal: Coverage tables only
└── implementation-plan.toml  # Full: All required fields
```

Inform user:
```
Artifacts ready. Run implement-commit skill to execute TDD pipeline.
```


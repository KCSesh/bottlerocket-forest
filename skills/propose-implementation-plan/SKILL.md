---
name: propose-implementation-plan
description: Create an implementation plan with atomic commits that build toward a complete feature
---

# Propose Implementation Plan Skill

## Purpose

Create a TOML implementation plan that drives the `implement-commit` skill.
The plan must be machine-parseable—the orchestrator reads it mechanically.

## When to Use

- Feature design document exists and is approved
- Ready to begin implementation
- Need to coordinate work or track progress

## Prerequisites

- Feature design exists in `docs/features/NNNN-feature-name/design.md`
- Test plan exists in `docs/features/NNNN-feature-name/test-plan.md`

## Output Format

Create `docs/features/NNNN-feature-name/implementation-plan.toml`:

```toml
[meta]
feature = "feature-name"
feature_dir = "docs/features/NNNN-feature-name"
workspace = "./path/to/workspace"

[dependencies]
1 = []
2 = []
3 = [1, 2]

[[commits]]
id = 1
title = "Short description"
message = "feat(scope): conventional commit message"
files = ["path/to/file.rs"]  # Relative to workspace
acceptance = ["Criterion from CC table (CC-N)"]
anti_patterns = ["What NOT to do"]
tests = ["test_name_from_test_plan"]  # REQUIRED: maps to test-plan.md
```

## Procedure

### 1. Study the Design Document

Read `design.md` thoroughly, noting:
- Module structure and affected files
- Dependencies between components
- Critical Constraints table (CC-1, CC-2, etc.)

### 2. Study the Test Plan (CRITICAL)

Read `test-plan.md` and create a mapping:
- List ALL test names from the Requirements Coverage table
- Each test MUST be assigned to exactly one commit
- Tests are the source of truth for acceptance criteria

### 3. Identify Capability Boundaries

Look for **capability boundaries**, not code artifact boundaries.

Good boundaries:
- A new module with its types, implementation, and tests together
- A user-visible capability (e.g., a new CLI command)
- A requirement being satisfied end-to-end

**Anti-pattern**: Splitting by artifact (types in one commit, impl in another, tests in a third).

### 4. Build the Dependency Graph

For each commit, list which commits must complete first.
Encode as `[dependencies]` table where key is commit ID, value is list of dependency IDs.

The orchestrator uses this to determine parallel execution.

### 5. Map Tests to Commits

For each commit, list the specific test names from test-plan.md that verify it:

```toml
tests = [
  "test_protocol_encode_acquire_request",
  "test_protocol_decode_roundtrip",
]
```

**Every test in test-plan.md must appear in exactly one commit's `tests` array.**

This creates an auditable trace: test-plan.md → implementation-plan.toml → code.

### 6. Extract Critical Constraints

For each commit, copy relevant entries from the design's Critical Constraints table:
- `acceptance` = criteria that MUST be met (reference CC-N and test names)
- `anti_patterns` = what reviewers should reject

### 7. Write Commit Entries

For each commit:

```toml
[[commits]]
id = N                    # Sequential integer
title = "..."             # What this commit accomplishes
message = "..."           # Full conventional commit message
files = ["..."]           # Paths relative to workspace
acceptance = ["..."]      # From CC table + test descriptions
anti_patterns = ["..."]   # From CC table, or empty []
tests = ["..."]           # Test names from test-plan.md (REQUIRED)
```

### 8. Verify with Independent Agent

After drafting the plan, spawn a verification agent:

```python
result = spawn(
    prompt=draft_spawn_prompt(SkillDefined(
        "Verify implementation plan covers all test-plan.md tests"
    )),
    context_files=[
        f"{skill_dir}/agents/verifier.md",
        f"{feature_dir}/test-plan.md",
        f"{feature_dir}/implementation-plan.toml",
    ],
    response_model=VerificationResult,
    read_only=True,
)

if not result.parsed.valid:
    # Fix gaps before proceeding
    agent_feedback(f"Plan incomplete: {result.parsed.gaps}")
```

The verifier checks:
- Every test in test-plan.md is assigned to a commit
- No test is assigned to multiple commits
- Acceptance criteria reference specific tests

### 9. Validate TOML

```bash
# Check TOML parses
python3 -c "import tomllib; tomllib.load(open('implementation-plan.toml', 'rb'))"
```

## Commit Sizing Guidelines

**Target: 200-400 lines changed per commit**

**Too Small:**
- Types without implementation
- Traits without at least one impl
- Implementation without tests

**Too Large:**
- Multiple unrelated capabilities
- More than 500 lines
- Touches more than 5-6 files

**Just Right:**
- A module with types, impl, and tests (~200-400 lines)
- A CLI command with handler and tests (~150-350 lines)
- A complete adapter with tests (~200-400 lines)

## Verification Model

```python
from pydantic import BaseModel

class VerificationResult(BaseModel):
    valid: bool
    total_tests_in_plan: int
    tests_assigned: int
    gaps: list[str]  # Tests not assigned to any commit
    duplicates: list[str]  # Tests assigned to multiple commits
```

## Example

See `docs/features/0000-templates/implementation-plan.toml` for a complete example.

## Next Steps

After creating and verifying the plan:
1. Review for feasibility and sizing
2. Run `implement-commit` skill to execute

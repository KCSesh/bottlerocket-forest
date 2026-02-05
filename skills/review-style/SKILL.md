---
name: review-style
description: Verify code follows project style guides without providing improvement suggestions
---

# Review Style Skill

## Purpose

Verify that code follows all project style guides.
This is a pass/fail gate, not a feedback session.

## When to Use

- TDD verification phase
- PR review for style compliance
- Any code review requiring style validation

## Inputs Required

### context_files
- Style guide for the phase being reviewed (e.g., `docs/style/rust-design.md`)
- Changed files to review
- Ledger file (optional) for multi-cycle reviews

### context_data
- `phase`: Which phase (designer/tester/implementor) - determines which style guide rules apply
- `changed_files`: List of files to review

## Response Format

Return a structured response matching this schema:

```python
class StyleResult(BaseModel):
    status: Literal["accept", "violations"]
    violations: list[str] = []  # Each: "file:line - description"
    themes: str | None = None   # Optional pattern across violations
```

**If all checks pass:**
```python
StyleResult(status="accept")
```

**If any check fails:**
```python
StyleResult(
    status="violations",
    violations=[
        "src/config.rs:23 - snafu selectors imported at module level; should be inside function",
        "src/parser.rs:45 - missing Given/When/Then comments in test",
    ],
    themes="Several violations stem from inconsistent error handling approach"
)
```

## Ledger Awareness

When a ledger file is provided in context_files:

1. **Read previous cycles** - Check what violations were raised and how implementor responded
2. **Verify RESOLVED items** - Confirm fixes were actually applied
3. **Respect DISPUTED items** - Do NOT re-raise if implementor's reason is valid
4. **Only raise NEW violations** - Issues not previously discussed

## Style Guides (Authoritative Sources)

The style guides in `docs/style/` are the ONLY source of truth:
- `docs/style/rust-design.md` - Design phase rules (types, signatures, bon, nutype, snafu)
- `docs/style/rust-impl.md` - Implementation phase rules (error handling, imports, no panics)
- `docs/style/rust-test.md` - Test phase rules (Given/When/Then, test_case, assertions)

Read the applicable guide and apply your judgment. Do not rely on summaries.

## Procedure

### 1. Identify Applicable Rules

Based on the phase:
- `designer`: rust-design.md rules (especially Project Crate Choices table)
- `tester`: rust-test.md rules
- `implementor`: rust-impl.md rules
- Test files (`#[cfg(test)]` modules) always get rust-test.md rules regardless of phase

### 2. Review Each File

Check against applicable style guide rules.
Only flag clear violations, not style preferences.

**Critical checks for designer phase:**
- Composite structs (2+ fields) use `#[derive(Builder)]` with `#[non_exhaustive]`
- Validated newtypes use `nutype`
- Errors use `snafu` with `#[snafu(module)]`
- Function bodies are `todo!()` not implementations

**Critical checks for implementor phase:**
- No `unwrap()`/`expect()` in production code
- Snafu context selectors imported inside functions
- No blank lines between imports

**Critical checks for tester phase:**
- Given/When/Then comments in every test
- `test_case` for parameterized tests

### 3. Return Structured Result

Use the `StyleResult` schema. The orchestrator parses this to decide next steps.

## What This Skill Does NOT Do

- Suggest improvements unrelated to violations
- Provide general constructive feedback
- Review scope/correctness (use review-scope for that)
- Flag style preferences not in guides

This is a binary gate: accept or violations.

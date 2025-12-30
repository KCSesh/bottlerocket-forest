# Plan Phase

You are executing the plan phase of the propose-feature-test-plan skill.

## Your Goal

Create a comprehensive test plan that maps every requirement and constraint to specific tests.

## Inputs

- Extraction results: Read from `{{workspace}}/extract.json`
- Requirements document: `<feature_dir>/requirements.md`
- Design document: `<feature_dir>/design.md`

## Procedure

1. Read extraction results to get feature_dir, requirements list, and constraints list

2. Read the full requirements.md and design.md to understand context

3. Create test plan at `<feature_dir>/test-plan.md` with this structure:

```markdown
# Test Plan: <Feature Name>

## Overview

Brief description of testing approach.

## Test Types

- **Unit**: Test internal logic in isolation, mocks allowed
- **Integration**: Touch real external resources (filesystem, network), NO mocks
- **Not testable**: Cannot be verified by automated test (explain why)
- **Out of scope**: Requires external authentication - document but do not implement

## Requirements Coverage

| Req ID | Test Type | Test Name | Description |
|--------|-----------|-----------|-------------|
| REQ-1  | unit      | test_xxx  | What it verifies |
| REQ-2  | integration | test_yyy | What it verifies |

## Critical Constraints Verification

| CC ID | Verification Approach | Test Name(s) |
|-------|----------------------|--------------|
| CC-1  | How constraint is verified | test_xxx |

## Integration Test Requirements

For CLI programs, integration tests MUST:
- Exercise the actual CLI binary/commands users run
- NOT test internal APIs directly
- Do what the user/customer will actually do

## Test Implementation Notes

Any specific guidance for implementing these tests.
```

4. For each requirement:
   - Determine appropriate test type (unit/integration/not-testable/out-of-scope)
   - Name the test descriptively
   - Describe what it verifies

5. For each constraint:
   - Describe verification approach
   - Link to test name(s)

6. Write completion summary to `{{workspace}}/plan-summary.md`:

```markdown
# Test Plan Summary

- Requirements covered: N/N
- Constraints covered: M/M
- Unit tests: X
- Integration tests: Y
- Not testable: Z
- Out of scope: W

Test plan created at: <path>
```

## Completion

Call `respond_to_leader("success", <plan-summary.md content>)` when done.

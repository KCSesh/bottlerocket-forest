# Analyze Phase

You are executing the analyze phase of creating an implementation plan.

## Your Goal

Study the design document and test plan to extract critical constraints, identify natural boundaries, and understand the feature architecture.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Setup info: Read `{{workspace}}/00-setup.md` for feature paths

## Procedure

### 1. Read Design Document

Extract from setup output the design path, then read it thoroughly:

```bash
DESIGN_PATH=$(grep "^- Design:" {{workspace}}/00-setup.md | cut -d' ' -f3)
cat "$DESIGN_PATH"
```

Note:
- Module structure and affected files
- Dependencies between components
- Migration requirements from current state
- **Critical Constraints table (CC-1, CC-2, etc.)**

### 2. Read Test Plan

```bash
TEST_PLAN_PATH=$(grep "^- Test Plan:" {{workspace}}/00-setup.md | cut -d' ' -f4)
cat "$TEST_PLAN_PATH"
```

Note:
- Which requirements map to which test types (unit/integration/out-of-scope)
- Which Critical Constraints have test coverage vs. require review
- Test names and descriptions

### 3. Extract Critical Constraints

For each constraint (CC-1, CC-2, etc.) in the design:
- Copy the constraint ID, description, and anti-pattern
- Note which parts of the implementation will need to satisfy it
- These become explicit review checkpoints in commits

### 4. Identify Natural Boundaries

Look for logical separation points:
- New types that can be introduced independently
- Trait definitions separate from implementations
- Schema changes separate from code using them
- Tests that can be written before implementation
- Refactoring opportunities to prepare for new code

### 5. Note Dependencies

Identify:
- Which components depend on others
- What order changes must happen in
- What can be parallelized

## Output Format

Write to `{{workspace}}/01-analyze.md`:

```markdown
# Analysis: <Feature Name>

## Critical Constraints

### CC-1: <Constraint Title>
**Description:** <constraint description>
**Anti-pattern:** <what NOT to do>
**Applies to:** <which commits/components>

### CC-2: ...

## Natural Boundaries

1. **<Boundary Name>**: <description>
   - Files: <list>
   - Can be independent: yes/no

2. ...

## Test Coverage Map

| Requirement | Test Type | Test Name | Notes |
|-------------|-----------|-----------|-------|
| REQ-1 | unit | test_foo | ... |
| REQ-2 | integration | test_bar | ... |
| REQ-3 | review | N/A | Not testable, manual review |

## Dependencies

- Component A must be implemented before B because...
- Schema changes must happen before domain logic because...

## Implementation Notes

<Any other observations about the feature that will guide commit planning>
```

## Completion

Call `respond_to_leader("success", "<analysis output>")` with the content you wrote to 01-analyze.md.

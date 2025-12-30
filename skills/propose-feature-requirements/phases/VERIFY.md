# Verify Phase

You are executing the verify phase of the propose-feature-requirements skill.

## Your Goal

Verify that prerequisites exist before creating requirements specification.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Feature number and name will be provided by user

## Procedure

1. Ask user for feature number and name (format: NNNN-feature-name)

2. Check that concept exists:
   ```bash
   ls $FOREST_ROOT/docs/features/NNNN-feature-name/concept.md
   ```
   
   If it doesn't exist, inform user they must use `propose-feature-concept` skill first.

3. Check for idea honing document:
   ```bash
   ls $FOREST_ROOT/planning/NNNN-feature-name/idea-honing.md 2>/dev/null
   ```
   
   If it exists, note that it should be reviewed for insights.

## Output Format

Write to `{{workspace}}/00-verify.md`:

```markdown
# Verification Results

## Feature
- Number: NNNN
- Name: feature-name
- Concept exists: yes/no
- Idea honing exists: yes/no

## Status
[PASS/FAIL]

## Notes
[Any relevant observations]
```

## Completion

Call `respond_to_leader("success", "<verification results>")` when done.

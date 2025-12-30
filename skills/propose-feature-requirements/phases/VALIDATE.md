# Validate Phase

You are executing the validate phase of the propose-feature-requirements skill.

## Your Goal

Review requirements document for completeness and correctness.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Requirements document at docs/features/NNNN-feature-name/requirements.md

## Procedure

1. Read write results:
   ```bash
   cat {{workspace}}/02-write.md
   ```

2. Verify file exists:
   ```bash
   ls $FOREST_ROOT/docs/features/NNNN-feature-name/requirements.md
   ```

3. Check for EARS keywords:
   ```bash
   grep -E "WHILE|WHEN|WHERE|THEN|SHALL" \
     $FOREST_ROOT/docs/features/NNNN-feature-name/requirements.md
   ```

4. Review for completeness:
   - All requirements use EARS keywords
   - Requirements are testable and specific
   - Inline examples are truly small (1-3 lines)
   - Larger examples are in appendices
   - Requirements have unique IDs with consistent prefix

5. Check common issues:
   - Missing EARS keywords
   - Too vague (not testable)
   - Inline examples too large
   - Inconsistent prefix

## Output Format

Write to `{{workspace}}/03-validate.md`:

```markdown
# Validation Results

## Checks
- File exists: yes/no
- EARS keywords present: yes/no
- Requirements testable: yes/no
- Examples properly sized: yes/no
- Consistent prefix: yes/no

## Issues Found
[List any issues, or "None"]

## Status
[PASS/FAIL]

## Next Steps
After creating requirements:
1. Review for completeness and testability
2. Get feedback from implementors
3. Once requirements are solid, move to design using `propose-feature-design` skill
```

## Completion

Call `respond_to_leader("success", "<validation results>")` when done.

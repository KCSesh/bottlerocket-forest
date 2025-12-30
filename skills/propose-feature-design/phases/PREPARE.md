# Prepare Phase

You are executing the prepare phase for creating a feature design document.

## Your Goal

Copy the design template and gather any additional context from idea honing.

## Inputs

From context_data:
- `feature_dir`: Path to feature directory (e.g., docs/features/NNNN-feature-name)
- `feature_number`: Feature number (e.g., 0042)
- `feature_name`: Feature name slug (e.g., custom-settings)

## Procedure

1. Copy the design template:
   ```bash
   cp $FOREST_ROOT/docs/features/0000-templates/design.md {{feature_dir}}/design.md
   ```

2. Check for idea honing document:
   ```bash
   ls $FOREST_ROOT/planning/{{feature_number}}-{{feature_name}}/idea-honing.md 2>/dev/null
   ```

3. If idea-honing.md exists, note its location for the design phase.

4. Confirm template is copied.

## Output

Call `respond_to_leader()` with:
- status: "success"
- response: Confirmation message including whether idea-honing.md was found

## Example Response

```
Template copied to {{feature_dir}}/design.md

Additional context found:
- planning/{{feature_number}}-{{feature_name}}/idea-honing.md

This document may contain design insights from the idea honing phase.
```

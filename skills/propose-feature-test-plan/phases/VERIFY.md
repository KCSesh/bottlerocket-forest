# Verify Phase

You are executing the verify phase of the propose-feature-test-plan skill.

## Your Goal

Confirm that prerequisite documents exist before proceeding with test plan creation.

## Inputs

- Feature number: `{{feature_number}}` (from context_data)
- Feature name: `{{feature_name}}` (from context_data)
- Workspace: `{{workspace}}` (from context_data)

## Procedure

1. Check for required files:

```bash
FEATURE_DIR="$FOREST_ROOT/docs/features/{{feature_number}}-{{feature_name}}"
ls "$FEATURE_DIR/concept.md"
ls "$FEATURE_DIR/requirements.md"
ls "$FEATURE_DIR/design.md"
```

2. Write verification results to `{{workspace}}/verify.json`:

```json
{
  "feature_dir": "/path/to/feature/dir",
  "concept_exists": true,
  "requirements_exists": true,
  "design_exists": true,
  "all_present": true
}
```

If any file is missing, set `all_present` to false and note which files are missing.

## Completion

Call `respond_to_leader("success", "Verification complete")` when done.

If prerequisites are missing, call `respond_to_leader("failed", "Missing: <list files>")` and explain what needs to be created first.

# Setup Phase

You are executing the setup phase of the propose-feature-requirements skill.

## Your Goal

Copy template and determine requirements prefix.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Feature info from 00-verify.md

## Procedure

1. Read verification results:
   ```bash
   cat {{workspace}}/00-verify.md
   ```

2. Copy requirements template:
   ```bash
   cp $FOREST_ROOT/docs/features/0000-templates/requirements.md \
      $FOREST_ROOT/docs/features/NNNN-feature-name/
   ```

3. Determine requirements prefix (2-5 characters):
   - Should be descriptive of the feature
   - Examples: `SEM` for semantic-search, `REG` for registry-management
   - Will be used like `SEM-1`, `SEM-2`, etc.
   
   Ask user for prefix or suggest one based on feature name.

## Output Format

Write to `{{workspace}}/01-setup.md`:

```markdown
# Setup Results

## Template
- Copied to: docs/features/NNNN-feature-name/requirements.md

## Requirements Prefix
- Prefix: XXX
- Example IDs: XXX-1, XXX-2, XXX-3

## Status
[COMPLETE]
```

## Completion

Call `respond_to_leader("success", "<setup results>")` when done.

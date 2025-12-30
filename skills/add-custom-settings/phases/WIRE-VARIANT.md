# Wire Settings to Variant Phase

Execute the add-settings-to-variant skill to wire settings into the variant.

## Your Goal

Use the add-settings-to-variant skill to integrate the settings model into the target variant.

## Inputs

- Workspace: Available in context_data as `workspace`
- Requirements: Read from `requirements.json` (provided in context_files)
- Model info: Read from `01-model.md` (provided in context_files)

## Procedure

1. Read requirements.json and 01-model.md for context

2. Announce and execute the add-settings-to-variant skill:
   ```
   USING SKILL "add-settings-to-variant"
   ```

3. Follow the add-settings-to-variant skill procedure with:
   - Settings name from requirements
   - Target variant from requirements
   - Model location from 01-model.md

4. Document the outcome in your response:
   - Variant files modified
   - Configuration changes
   - Any issues encountered

## Output Format

Write a markdown summary to `<workspace>/02-variant.md`:

```markdown
# Variant Wiring

## Variant
<variant-name>

## Files Modified
- path/to/variant/file1
- path/to/variant/file2

## Changes Made
<brief description>

## Status
✅ Complete / ⚠️ Issues encountered
```

## Completion

Call `respond_to_leader("success", "<markdown content>")` with the summary.

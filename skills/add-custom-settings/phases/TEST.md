# Test Settings Locally Phase

Execute the test-settings-locally skill to validate the implementation.

## Your Goal

Use the test-settings-locally skill to build and test the variant with new settings.

## Inputs

- Workspace: Available in context_data as `workspace`
- Requirements: Read from `requirements.json` (provided in context_files)
- Model info: Read from `01-model.md` (provided in context_files)
- Variant info: Read from `02-variant.md` (provided in context_files)

## Procedure

1. Read previous phase outputs for context

2. Announce and execute the test-settings-locally skill:
   ```
   USING SKILL "test-settings-locally"
   ```

3. Follow the test-settings-locally skill procedure to:
   - Build the variant
   - Validate settings are accessible
   - Test setting values

4. Document the outcome in your response:
   - Build status
   - Test results
   - Any issues encountered

## Output Format

Write a markdown summary to `<workspace>/03-test.md`:

```markdown
# Local Testing

## Build Status
✅ Success / ❌ Failed

## Tests Performed
- Test 1: <description> - <result>
- Test 2: <description> - <result>

## Issues
<any problems encountered>

## Status
✅ Complete / ⚠️ Issues encountered
```

## Completion

Call `respond_to_leader("success", "<markdown content>")` with the summary.

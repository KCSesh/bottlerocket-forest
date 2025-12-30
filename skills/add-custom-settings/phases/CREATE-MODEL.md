# Create Settings Model Phase

Execute the create-settings-model skill to define the settings model.

## Your Goal

Use the create-settings-model skill to create the Rust model for the custom settings.

## Inputs

- Workspace: Available in context_data as `workspace`
- Requirements: Read from `requirements.json` (provided in context_files)

## Procedure

1. Read requirements.json to get settings name and structure

2. Announce and execute the create-settings-model skill:
   ```
   USING SKILL "create-settings-model"
   ```

3. Follow the create-settings-model skill procedure with the gathered requirements

4. Document the outcome in your response:
   - Files created
   - Model structure
   - Any issues encountered

## Output Format

Write a markdown summary to `<workspace>/01-model.md`:

```markdown
# Settings Model Creation

## Settings Name
<name>

## Files Created
- path/to/file1.rs
- path/to/file2.rs

## Model Structure
<brief description>

## Status
✅ Complete / ⚠️ Issues encountered
```

## Completion

Call `respond_to_leader("success", "<markdown content>")` with the summary.

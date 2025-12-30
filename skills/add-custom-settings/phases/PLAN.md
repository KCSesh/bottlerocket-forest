# Planning Phase

Gather requirements for adding custom settings to a Bottlerocket variant.

## Your Goal

Collect all necessary information to execute the settings addition workflow.

## Inputs

- Workspace: Available in context_data as `workspace`
- User interaction: Ask questions to gather requirements

## Procedure

1. Ask the user for:
   - **Settings name**: What should the settings be called? (e.g., "my-app", "custom-config")
   - **Settings structure**: What fields/values should the settings contain?
   - **Target variant**: Which variant should receive these settings? (e.g., "aws-k8s-1.31")
   - **Worktree**: Which worktree to use for development?

2. Validate responses:
   - Settings name should be lowercase, hyphen-separated
   - Structure should be clear (can be JSON schema or description)
   - Variant must exist in bottlerocket/variants/

3. Write requirements to `<workspace>/requirements.json`:

```json
{
  "settings_name": "my-app",
  "structure": {
    "description": "Settings structure description or schema",
    "fields": ["field1", "field2"]
  },
  "variant": "aws-k8s-1.31",
  "worktree": "worktrees/add-settings"
}
```

## Output

Write `requirements.json` to the workspace directory.

## Completion

Call `respond_to_leader("success", "<json content>")` with the requirements JSON.

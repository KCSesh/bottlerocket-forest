# Publish Phase

You are executing the publish phase of building a kit locally.

## Your Goal

Publish the built kit to the local OCI registry.

## Inputs

- Workspace: Read from context_data["workspace"]
- Kit name: Read from `{{workspace}}/input.json` field "kit_name"

## Procedure

1. Navigate to kit directory:
   ```bash
   cd kits/{{kit_name}}
   ```

2. Publish to local registry:
   ```bash
   make publish VENDOR=local
   ```

3. Capture output showing published tags

## Output Format

Write to `{{workspace}}/03-publish.md`:

```markdown
# Publish Complete

## Publish Command
- Vendor: local
- Registry: localhost:5000
- Command: make publish VENDOR=local

## Published Artifacts
- Repository: localhost:5000/{{kit_name}}
- Tags: [list published tags]

## Publish Status
- Success: yes/no
```

## Completion

Call `respond_to_leader("success", "<your output>")` when done.

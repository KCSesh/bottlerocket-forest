# Verify Phase

You are executing the verify phase of building a kit locally.

## Your Goal

Verify the kit was successfully published to the local registry.

## Inputs

- Workspace: Read from context_data["workspace"]
- Kit name: Read from `{{workspace}}/input.json` field "kit_name"

## Procedure

1. Check registry catalog:
   ```bash
   curl http://localhost:5000/v2/_catalog
   ```

2. List kit tags:
   ```bash
   curl http://localhost:5000/v2/{{kit_name}}/tags/list
   ```

3. Verify kit appears in both outputs

## Output Format

Write to `{{workspace}}/04-verify.md`:

```markdown
# Verification Complete

## Registry Catalog
- Kit present: yes/no
- Repository name: {{kit_name}}

## Available Tags
[list tags from registry]

## Verification Status
- Success: yes/no
- Ready for variant builds: yes/no

## Next Steps
1. Update variant's Twoliter.toml to reference the new kit version
2. Run `make update` in the variant repo
3. Build the variant with `cargo make`
```

## Completion

Call `respond_to_leader("success", "<your output>")` when done.

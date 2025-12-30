# Build Phase

You are executing the build phase of building a kit locally.

## Your Goal

Build the kit for the specified architecture.

## Inputs

- Workspace: Read from context_data["workspace"]
- Kit name: Read from `{{workspace}}/input.json` field "kit_name"
- Architecture: Read from `{{workspace}}/input.json` field "arch" (default: x86_64)

## Procedure

1. Navigate to kit directory:
   ```bash
   cd kits/{{kit_name}}
   ```

2. Build the kit:
   ```bash
   make build ARCH={{arch}}
   ```

3. Verify build artifacts exist:
   ```bash
   ls -lh build/
   ```

## Output Format

Write to `{{workspace}}/02-build.md`:

```markdown
# Build Complete

## Build Command
- Architecture: {{arch}}
- Command: make build ARCH={{arch}}

## Build Artifacts
- Location: kits/{{kit_name}}/build/
- Files: [list key files]

## Build Status
- Success: yes/no
- Duration: approximate time
```

## Completion

Call `respond_to_leader("success", "<your output>")` when done.

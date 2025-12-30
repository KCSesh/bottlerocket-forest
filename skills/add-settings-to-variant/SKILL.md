---
name: add-settings-to-variant
description: Wire an existing settings model into a Bottlerocket variant
---

# Add Settings to Variant

Wire an existing settings model into a Bottlerocket variant using the settings-plugins approach.

## Purpose

Integrates a settings model (already defined) into a variant by:
- Locating the variant's settings-plugins crate
- Adding the settings model as a dependency
- Verifying compilation

## When to Use

Use when you have:
- An existing settings model crate (e.g., `my-settings`)
- A target variant that needs to consume those settings
- Need to wire them together via settings-plugins

**Prerequisites:**
- Settings model crate exists and compiles
- Variant exists in bottlerocket/variants/
- Core-kit is available (contains settings-plugins)

## Phases

1. **LOCATE**: Find variant's settings-plugins, understand structure
2. **INTEGRATE**: Add settings model to plugin, update dependencies
3. **VERIFY**: Build settings-plugins package, confirm compilation

## Orchestrator Loop

```python
import json

workspace = f"planning/add-settings-{variant_name}"
bash(f"mkdir -p {workspace}", on_error="raise")
write("create", f"{workspace}/input.json", file_text=json.dumps({
  "variant_name": variant_name,
  "settings_crate": settings_crate
}))

while True:
  result = bash(f"python3 skills/add-settings-to-variant/next-step.py {workspace}", on_error="raise")
  action = json.loads(result)
  
  if action["type"] == "done":
    final = fs_read("Line", f"{workspace}/FINAL.md", 1, -1)
    log(final)
    break
  
  if action["type"] == "gate_failed":
    log(f"Gate failed: {action['reason']}")
    break
  
  if action["type"] == "spawn":
    r = spawn(
      action["prompt"],
      context_files=action["context_files"],
      context_data=action.get("context_data", {}),
      allow_tools=True
    )
    write("create", f"{workspace}/{action['output_file']}", file_text=r.response)
```

## Technical Notes

- Runtime discovery NOT YET IMPLEMENTED - must use settings-plugins approach
- Only one settings-plugin per variant (virtual package conflict)
- Settings-plugins typically in kits/bottlerocket-core-kit/packages/

---
name: add-custom-settings
description: Meta-skill orchestrating the full workflow for adding custom settings to a Bottlerocket variant
---

# Add Custom Settings Meta-Skill

A meta-skill that orchestrates the complete workflow for adding custom settings to a Bottlerocket variant.

## Purpose

Guides you through the entire process of adding custom settings:
1. Planning requirements (settings name, structure, target variant)
2. Creating the settings model
3. Wiring settings into the variant
4. Testing the settings locally

## When to Use

Use when you need to add new custom settings to a Bottlerocket variant from start to finish.

## Component Skills

This meta-skill orchestrates:
- `create-settings-model` - Create the settings model definition
- `add-settings-to-variant` - Wire settings into variant configuration
- `test-settings-locally` - Build and test the variant with new settings

## How It Works

This skill uses the script-driven-skill pattern:
- **Orchestrator**: Runs the state machine and spawns phase agents
- **State machine** (`next-step.py`): Controls flow between phases
- **Phase files**: Self-contained instructions for each step

## Phases

1. **PLAN**: Gather requirements (settings name, structure, variant)
2. **CREATE-MODEL**: Execute create-settings-model skill
3. **WIRE-VARIANT**: Execute add-settings-to-variant skill
4. **TEST**: Execute test-settings-locally skill

## Workspace

Creates workspace at `planning/add-custom-settings-<timestamp>/` with:
- `requirements.json` - Captured requirements
- `01-model.md` - Model creation output
- `02-variant.md` - Variant wiring output
- `03-test.md` - Testing output
- `FINAL.md` - Summary of completed work

## Usage

From the orchestrator:
```python
workspace = f"planning/add-custom-settings-{timestamp}"
bash(f"mkdir -p {workspace}", on_error="raise")

while True:
    result = bash(f"python3 skills/add-custom-settings/next-step.py {workspace}", on_error="raise")
    action = json.loads(result)
    
    if action["type"] == "done":
        final = fs_read("Line", f"{workspace}/FINAL.md", 1, -1)
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

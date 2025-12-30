# Update Configuration Phase

You are executing the configuration update phase for building a variant from local kits.

## Your Goal

Update Twoliter.toml and Infra.toml to reference locally published kits.

## Inputs

- Workspace: Read from context_data["workspace"]
- Kit versions: Read from `{{workspace}}/input.json`

## Procedure

1. Read input file to get kit names and versions:
   ```python
   import json
   from pathlib import Path
   workspace = Path("{{workspace}}")
   input_data = json.loads((workspace / "input.json").read_text())
   kits = input_data["kits"]  # List of {name, version}
   variant = input_data.get("variant", "")
   arch = input_data.get("arch", "")
   ```

2. Update `$FOREST_ROOT/bottlerocket/Twoliter.toml`:
   - Locate [[kit]] sections for each kit in the list
   - Update version to match local kit version
   - Set vendor = "local"

3. Verify `$FOREST_ROOT/bottlerocket/Infra.toml` has local registry:
   ```toml
   [vendor.local]
   registry = "localhost:5000"
   ```
   Add if missing.

4. Write completion marker to `{{workspace}}/01-config-updated.json`:
   ```json
   {
     "phase": "update_config",
     "status": "complete",
     "kits_updated": ["kit-name-1", "kit-name-2"]
   }
   ```

## Validation

- Twoliter.toml contains correct kit versions with vendor = "local"
- Infra.toml contains local registry configuration

## Completion

Call `respond_to_leader("success", "Configuration updated")` when done.

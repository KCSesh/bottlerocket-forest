# LOCATE Phase

Find the variant's settings-plugins crate and understand its structure.

## Your Goal

Locate where the variant's settings-plugins package lives and document its current state.

## Inputs

Read from workspace/input.json:
- `variant_name`: Target variant (e.g., "aws-ecs-2")
- `settings_crate`: Settings model crate name (e.g., "my-settings")

## Procedure

1. **Find the variant directory:**
   ```bash
   ls bottlerocket/variants/<variant_name>/
   ```

2. **Locate settings-plugins package:**
   Settings-plugins are typically in core-kit:
   ```bash
   find kits/bottlerocket-core-kit/packages -name "*settings-plugin*" -type d
   ```
   
   Common locations:
   - `kits/bottlerocket-core-kit/packages/<variant>-settings-plugins/`
   - `kits/bottlerocket-core-kit/packages/settings-plugins-<variant>/`

3. **Examine current structure:**
   ```bash
   cat <settings-plugins-path>/Cargo.toml
   cat <settings-plugins-path>/src/lib.rs
   ```

4. **Document findings** in workspace/01-locate.md:
   ```markdown
   # Locate: <variant_name>
   
   ## Settings-Plugins Location
   Path: <full-path>
   
   ## Current Dependencies
   - dependency1
   - dependency2
   
   ## Current Settings Models
   - model1
   - model2
   
   ## Integration Point
   File: <path-to-lib.rs>
   Pattern: [describe how models are registered]
   ```

## Completion

Call `respond_to_leader("success", "<your-markdown-output>")` with the findings.

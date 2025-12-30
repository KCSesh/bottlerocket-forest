# INTEGRATE Phase

Add the settings model to the variant's settings-plugins crate.

## Your Goal

Wire the settings model into the plugin by updating Cargo.toml and lib.rs.

## Inputs

- workspace/input.json: variant_name, settings_crate
- workspace/01-locate.md: settings-plugins location and structure

## Procedure

1. **Add dependency to Cargo.toml:**
   ```toml
   [dependencies]
   <settings_crate> = { path = "../../../../sources/<settings_crate>" }
   ```
   
   Use str_replace to add after existing dependencies.

2. **Import in lib.rs:**
   ```rust
   use <settings_crate>::<SettingsModel>;
   ```

3. **Register the model:**
   Follow the pattern from existing models. Typically:
   ```rust
   extension_builder = extension_builder.with_model::<SettingsModel>()?;
   ```

4. **Document changes** in workspace/02-integrate.md:
   ```markdown
   # Integrate: <settings_crate>
   
   ## Changes Made
   
   ### Cargo.toml
   - Added dependency: <settings_crate>
   
   ### lib.rs
   - Added import: <SettingsModel>
   - Registered model at line: <line-number>
   
   ## Files Modified
   - <path-to-Cargo.toml>
   - <path-to-lib.rs>
   ```

## Completion

Call `respond_to_leader("success", "<your-markdown-output>")` with the changes summary.

# VERIFY Phase

Build the settings-plugins package to verify the integration.

## Your Goal

Confirm the settings model compiles correctly within the plugin.

## Inputs

- workspace/01-locate.md: settings-plugins location
- workspace/02-integrate.md: changes made

## Procedure

1. **Navigate to settings-plugins package:**
   ```bash
   cd <settings-plugins-path>
   ```

2. **Build the package:**
   ```bash
   cargo build
   ```
   
   If build fails, use `on_error="fix"` to diagnose and repair.

3. **Verify the settings model is included:**
   ```bash
   cargo tree | grep <settings_crate>
   ```

4. **Document results** in workspace/03-verify.md:
   ```markdown
   # Verify: Build Success
   
   ## Build Output
   [Summary of cargo build - success/warnings]
   
   ## Dependency Verification
   ```
   <output of cargo tree>
   ```
   
   ## Status
   ✅ Settings model successfully integrated
   
   ## Next Steps
   - Build the full kit: use `build-kit-locally` skill
   - Build variant: use `build-variant-from-local-kits` skill
   ```

## Completion

Call `respond_to_leader("success", "<your-markdown-output>")` with verification results.

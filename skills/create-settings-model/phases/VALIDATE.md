# Validate Phase

Verify the settings model package builds correctly.

## Your Goal

Run cargo check and verify the package structure is correct.

## Inputs

- Workspace: Available in context_data as `workspace`
- Previous phase outputs for package location

## Procedure

1. Read package location from scaffold summary

2. Run cargo check:
   ```bash
   cd kits/bottlerocket-core-kit/packages/<model-name>-settings
   cargo check
   ```

3. Verify files exist:
   - Cargo.toml
   - lib.rs with SettingsModel impl
   - main.rs with extension builder

4. Check for common issues:
   - Missing dependencies
   - Syntax errors
   - Trait method signatures

5. Write final summary to `<workspace>/FINAL.md`:
   ```markdown
   # Settings Model Created: <model-name>
   
   ## Location
   kits/bottlerocket-core-kit/packages/<model-name>-settings/
   
   ## Structure
   - ✅ Cargo.toml with correct dependencies
   - ✅ lib.rs with SettingsModel trait implementation
   - ✅ main.rs with LinearMigratorExtensionBuilder
   - ✅ cargo check passes
   
   ## Next Steps
   
   1. Add fields to the settings struct
   2. Implement custom validation logic
   3. Add template generation if needed
   4. Create templates in variant
   5. Wire to variant via settings-plugins
   
   ## Usage
   
   The model is ready to be integrated into a variant. See the
   customizing-variant-settings-api documentation for next steps.
   ```

## Output Format

Write a markdown file to `<workspace>/FINAL.md` with validation results and next steps.

## Completion

Call `respond_to_leader("success", "<final-summary>")` with the complete status.

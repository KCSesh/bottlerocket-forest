# Implement Phase

Implement the SettingsModel trait for the settings model.

## Your Goal

Add complete implementations of all 4 required SettingsModel methods.

## Inputs

- Workspace: Available in context_data as `workspace`
- Scaffold summary: Read from `<workspace>/00-scaffold.md` for package location

## Required Methods

The SettingsModel trait requires:

1. **get_version()** - Return version string (e.g., "v1")
2. **set()** - Deserialize and validate settings from JSON
3. **generate()** - Generate config files from templates
4. **validate()** - Validate settings consistency

## Procedure

1. Read package location from scaffold summary

2. Update lib.rs to implement SettingsModel:
   ```rust
   use bottlerocket_settings_sdk::{SettingsModel, GenerateResult, BottlerocketSetting};
   use serde_json::Value;
   use std::collections::HashMap;
   
   impl SettingsModel for <ModelName>Settings {
       fn get_version() -> &'static str {
           "v1"
       }
       
       fn set(data: Value) -> Result<BottlerocketSetting, Box<dyn std::error::Error>> {
           let settings: Self = serde_json::from_value(data)?;
           Ok(BottlerocketSetting::from(settings))
       }
       
       fn generate(
           settings: &BottlerocketSetting,
           _existing: &HashMap<String, BottlerocketSetting>
       ) -> Result<GenerateResult, Box<dyn std::error::Error>> {
           Ok(GenerateResult::default())
       }
       
       fn validate(
           _settings: &BottlerocketSetting,
           _existing: &HashMap<String, BottlerocketSetting>
       ) -> Result<(), Box<dyn std::error::Error>> {
           Ok(())
       }
   }
   ```

3. Add any necessary imports and error types

4. Write summary to `<workspace>/01-implement.md`:
   ```markdown
   # Implementation Complete
   
   Added SettingsModel trait implementation with:
   - get_version() returning "v1"
   - set() for deserialization
   - generate() for config generation
   - validate() for consistency checks
   
   Next: Validate with cargo check
   ```

## Output Format

Write a markdown file to `<workspace>/01-implement.md` documenting the implementation.

## Completion

Call `respond_to_leader("success", "<summary>")` with details of what was implemented.

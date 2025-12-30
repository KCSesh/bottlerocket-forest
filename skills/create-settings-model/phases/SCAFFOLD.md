# Scaffold Phase

Create the directory structure and basic files for a new settings model package.

## Your Goal

Set up the package skeleton in core-kit with Cargo.toml, lib.rs, and main.rs stubs.

## Inputs

- Workspace: Available in context_data as `workspace`
- Model name: Read from `<workspace>/model-name.txt`

## Procedure

1. Read the model name:
   ```python
   workspace = Path(CONTEXT_DATA["workspace"])
   model_name = (workspace / "model-name.txt").read_text().strip()
   ```

2. Create package directory:
   ```bash
   mkdir -p kits/bottlerocket-core-kit/packages/<model-name>-settings
   ```

3. Create Cargo.toml:
   ```toml
   [package]
   name = "<model-name>-settings"
   version = "0.1.0"
   edition = "2021"
   
   [[bin]]
   name = "<model-name>-settings"
   path = "main.rs"
   
   [lib]
   path = "lib.rs"
   
   [dependencies]
   bottlerocket-settings-sdk = { path = "../../../../bottlerocket-settings-sdk" }
   model-derive = { path = "../../../../sources/models/model-derive" }
   modeled-types = { path = "../../../../sources/models/modeled-types" }
   serde = { version = "1", features = ["derive"] }
   snafu = "0.8"
   ```

4. Create lib.rs stub:
   ```rust
   use model_derive::model;
   use modeled_types::Identifier;
   use serde::{Deserialize, Serialize};
   
   #[model]
   struct <ModelName>Settings {
       // TODO: Add fields
   }
   ```

5. Create main.rs stub:
   ```rust
   use bottlerocket_settings_sdk::{LinearMigratorExtensionBuilder, SettingsModel};
   use <model_name>_settings::<ModelName>Settings;
   
   fn main() {
       LinearMigratorExtensionBuilder::with_name("<model-name>")
           .with_models(vec![<ModelName>Settings::model()])
           .build()
           .run();
   }
   ```

6. Write summary to `<workspace>/00-scaffold.md`:
   ```markdown
   # Scaffold Complete
   
   Created package at: kits/bottlerocket-core-kit/packages/<model-name>-settings/
   
   Files:
   - Cargo.toml
   - lib.rs (struct stub)
   - main.rs (extension builder)
   
   Next: Implement SettingsModel trait methods
   ```

## Output Format

Write a markdown file to `<workspace>/00-scaffold.md` documenting what was created.

## Completion

Call `respond_to_leader("success", "<summary>")` with the path to the created package.

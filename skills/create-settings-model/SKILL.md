---
name: create-settings-model
description: Create a new Bottlerocket settings model with SettingsModel trait implementation
---

# Create Settings Model Skill

Guide users through creating a new Bottlerocket settings model package.

## Purpose

Creates a complete settings model package with:
- Proper directory structure in core-kit
- Cargo.toml with correct dependencies
- Rust struct with #[model] macro
- Full SettingsModel trait implementation (get_version, set, generate, validate)

## When to Use

Use when adding new custom settings to Bottlerocket that require:
- API-driven configuration
- Version migration support
- Template generation for service configs
- Validation logic

## Prerequisites

- Working in a Bottlerocket forest worktree
- Core-kit available at ./kits/bottlerocket-core-kit/
- Settings SDK available at ./bottlerocket-settings-sdk/

## Phases

1. **SCAFFOLD**: Create directory structure and basic files
2. **IMPLEMENT**: Implement SettingsModel trait methods
3. **VALIDATE**: Verify with cargo check

## Usage

The orchestrator will:
1. Create workspace in planning/<model-name>-settings/
2. Execute phases sequentially via subagents
3. Validate gates between phases
4. Produce final package in core-kit

## Output

Complete settings model package at:
```
kits/bottlerocket-core-kit/packages/<name>-settings/
├── Cargo.toml
├── lib.rs
└── main.rs
```

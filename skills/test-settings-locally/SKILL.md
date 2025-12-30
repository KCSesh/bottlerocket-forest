---
name: test-settings-locally
description: Build and test settings SDK changes locally using core-kit and variant builds
---

# Script-Driven Skill: Test Settings Locally

Test Bottlerocket settings changes end-to-end using local builds.

## Purpose

Validate settings SDK changes by building core-kit with your changes and testing in a variant build, all using local registry.

## When to Use

- Testing changes to bottlerocket-settings-sdk
- Validating new settings extensions
- End-to-end testing before publishing

## Prerequisites

- Docker installed and running
- Settings changes in bottlerocket-settings-sdk or core-kit
- Bottlerocket variant repository available

## Phases

1. **SETUP** - Verify prerequisites, start local registry
2. **BUILD-KIT** - Build core-kit with settings changes, publish locally
3. **BUILD-VARIANT** - Configure and build variant using local kit

## Workspace

Creates workspace at `planning/test-settings-<timestamp>/` with:
- `progress.json` - State tracking
- `00-setup.md` - Setup verification results
- `01-build-kit.md` - Kit build results
- `02-build-variant.md` - Variant build results
- `FINAL.md` - Summary and next steps

## Usage

The orchestrator runs the state machine loop, spawning subagents for each phase.

## Related Skills

- `local-registry` - Registry management
- `build-kit-locally` - Kit building
- `build-variant-from-local-kits` - Variant building

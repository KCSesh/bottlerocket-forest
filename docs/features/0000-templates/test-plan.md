---
feature: feature-name
---

# Test Plan: Feature Name

## Overview

Brief description of what this test plan covers.

## Requirements Coverage

| Req ID | Test Type | Test Name | Description |
|--------|-----------|-----------|-------------|
| REQ-01 | Unit | test_example | Verifies requirement |

## Critical Constraints Coverage

| CC ID | Verification | Test/Review |
|-------|--------------|-------------|
| CC-01 | How verified | test_name or "Code Review" |

## Test Categories

### Unit Tests
Test internal logic in isolation. Mocks are acceptable.

### Integration Tests
Test with real resources. NO mocks allowed. CLI programs must use actual CLI commands.

### Not Testable
Requirements that cannot be verified by automated tests. Document why (e.g., subjective quality, requires human judgment, infeasible setup).

### Out of Scope
Tests requiring authentication or external services are documented but not implemented.

## Test Inventory

### module_name

- `test_function_name` - What it tests

## Notes

Additional context or considerations.

# Test Plan: Embedding Test Isolation

## Test Types

- **Unit**: Test internal logic in isolation, mocks allowed
- **Integration**: Touch real external resources, NO mocks

## Critical Constraints Verification

| CC ID | Verification Approach | Test Name(s) |
|-------|----------------------|---------------|
| CC-1 | Verify open()/open_with_config() signatures unchanged | Code review |
| CC-2 | embedding_integration.rs continues using real model | test_real_embedding_integration |
| CC-3 | Mock returns 384-dim vectors | test_mock_provider_dimension_matches_config |
| CC-4 | Mock implements embed_batch() and model_name() | test_open_with_mock_uses_injected_provider |

## Unit Tests

| Test Name | CC Coverage | Description |
|-----------|-------------|-------------|
| test_open_with_mock_uses_injected_provider | CC-4 | Verifies injected factory is called instead of loading real model |
| test_mock_provider_dimension_matches_config | CC-3 | Verifies mock returns vectors of length 384 |
| test_facade_build_with_mock_no_network | CC-1 | Verifies build() works with mock, no network access |
| test_facade_search_with_mock_no_network | CC-1 | Verifies search() works with mock, no network access |

## Integration Tests

| Test Name | CC Coverage | Description |
|-----------|-------------|-------------|
| test_real_embedding_integration | CC-2 | Existing test continues using real embeddings |

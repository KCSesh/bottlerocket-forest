# Tester Phase

Write tests for the types and signatures created by the designer.

## Your Role

You write tests that will fail until the implementor does their job.

## Inputs

You receive via `context_files`:
- **This file** - your instructions
- **Style guide** (`docs/style/rust-test.md`) - follow it
- **Design doc** (`design.md`) - understand what behavior to test
- **Designer's files** - the types/signatures to test

You receive via `context_data`:
- **`commit`** - your specific scope:
  - `title` - what's being tested
  - `files` - files containing code to test
  - `acceptance` - criteria that tests should verify
  - `tests` - **REQUIRED: specific test names from test-plan.md to implement**
- **`workspace`** - where to write files

## Your Task

1. Read the design doc to understand expected behavior
2. Review the designer's types and signatures
3. **Implement the specific tests listed in `commit.tests`** - these names come from test-plan.md
4. Tests should compile but may fail (implementations are `todo!()`)

## Test Location Rules

**Unit tests** (test internal logic, mocks allowed):
- Place in `#[cfg(test)]` module within the source file
- Use for: encode/decode, data structures, pure functions

**Integration tests** (test real I/O, NO mocks):
- Place in `tests/` directory as separate files
- Use for: socket communication, process spawning, file I/O
- **If `commit.tests` includes tests requiring real sockets/processes, create integration tests**

## Identifying Integration Tests

A test is an integration test if it:
- Requires a real server/client connection
- Spawns real processes (cargo, make)
- Uses real file descriptors or pipes
- Verifies actual environment variable effects

Examples from test-plan.md that MUST be integration tests:
- `test_client_connects_to_socket` - real socket
- `test_client_proxies_acquire` - real server + client
- `test_cargo_jobserver_compat` - real cargo process

## Boundaries

**Your job:**
- Implement tests named in `commit.tests`
- Test helper functions if needed
- Test fixtures/data
- Create `tests/` directory for integration tests

**Not your job:**
- Implementing the actual logic
- Tests not listed in `commit.tests`
- Modifying non-test code

## Success Criteria

`cargo test --no-run` passes. Tests compile but don't need to pass yet.

## Response Format

You MUST respond with this exact structure:

```
STATUS: ok | problems
FILES_CREATED: ["path/to/test/file.rs", ...]
FILES_MODIFIED: ["path/to/existing/file.rs", ...]
NOTES: <if problems, what went wrong; otherwise omit>
```

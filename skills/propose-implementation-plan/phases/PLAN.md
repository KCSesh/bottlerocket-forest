# Plan Phase

You are executing the plan phase of creating an implementation plan.

## Your Goal

Break the feature into atomic, reviewable commits following the Atomic Commit Rules.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Analysis: Read `{{workspace}}/01-analyze.md` for constraints and boundaries

## Atomic Commit Rules

**Each commit MUST be:**

1. **Buildable** - The project compiles after this commit
2. **Tested** - New code has tests; existing tests pass (or are explicitly disabled with TODO)
3. **Focused** - Does one logical thing
4. **Reviewable** - Small enough to review in one sitting (target: <400 lines changed)

**Each commit SHOULD:**

1. **Be independently valuable** - Provides some benefit even if later commits aren't merged
2. **Have a clear purpose** - The commit message explains why, not just what
3. **Minimize risk** - Smaller commits are easier to revert if problems arise

## Handling Test Breakage

When a commit breaks tests in distant modules (e.g., schema changes that break integration tests), **do not leave broken tests**.

**Pattern for disabling tests:**

```rust
// TODO: Re-enable in Commit 9a after updating domain types
#[cfg(all(test, feature = "enable_broken_tests"))]
mod tests {
    // ...
}
```

Or for individual tests:

```rust
#[test]
#[ignore] // TODO: Re-enable in Commit 12a after facade integration
fn test_search_returns_results() {
    // ...
}
```

**When planning commits that break existing tests:**

1. **Identify which tests will break** - Note them in the commit description
2. **Disable tests explicitly** - Use `#[ignore]` or feature flags, not deletion
3. **Add a TODO comment** - Reference the specific commit that will re-enable them
4. **Plan a re-enablement commit** - Add a commit (e.g., "9a", "12a") that fixes and re-enables the tests
5. **Place re-enablement after dependencies are ready** - The re-enable commit comes after all changes needed to fix the tests

## Commit Sizing Guidelines

**Too Small** (avoid):
- Adding a single import
- Renaming one variable
- Adding an empty module

**Too Large** (avoid):
- Entire feature in one commit
- Multiple unrelated changes
- Changes that take days to review

**Just Right** (target):
- Add a new type with its tests (~50-200 lines)
- Implement a trait for one adapter (~100-300 lines)
- Add a new CLI command with tests (~100-300 lines)
- Refactor a module to prepare for new feature (~100-400 lines)

## Procedure

### 1. Read Analysis

```bash
cat {{workspace}}/01-analyze.md
```

### 2. Identify Phases

Group related commits into phases:
- Phase 1: Foundation (types, traits, interfaces)
- Phase 2: Core Implementation (implementations, wiring)
- Phase 3: Integration (CLI, tests, documentation)

### 3. Plan Commits

For each commit, determine:
- **Summary**: What and why in one paragraph
- **Files Changed**: List files with brief description
- **Key Changes**: Bullet points of specific modifications
- **Requirements Addressed**: List requirement IDs (REQ-*)
- **Constraints Addressed**: List constraint IDs (CC-*) with constraint text
- **Testing**: Reference tests from test plan
- **Dependencies**: Which prior commits must be complete
- **Estimated Size**: Lines changed (target <400)

### 4. Order by Dependency

Ensure each commit builds on previous work.

### 5. Identify Parallelization

Note which commits have no dependencies on each other.

## Output Format

Write to `{{workspace}}/02-plan.md`:

```markdown
# Commit Plan: <Feature Name>

## High-Level Checklist

- [ ] **Commit 1**: <One-line description>
- [ ] **Commit 2**: <One-line description>
...

## Phases

### Phase 1: Foundation

#### Commit 1: <Title>

**Summary**: <What and why>

**Files Changed**:
- `path/to/file.rs`: <description>

**Key Changes**:
- <change 1>
- <change 2>

**Requirements Addressed**: REQ-1, REQ-2

**Constraints Addressed**:
- CC-1: <constraint text from analysis>

**Testing**: Adds test_foo for REQ-1

**Dependencies**: None

**Estimated Size**: ~150 lines

#### Commit 2: ...

### Phase 2: Core Implementation

...

## Parallelization Notes

- Commits 3 and 4 can be developed in parallel (both depend only on 1-2)
- Phase 2 requires all of Phase 1 to be complete

## Open Questions

- [ ] <Question that needs resolution during implementation>
```

## Completion

Call `respond_to_leader("success", "<plan output>")` with the content you wrote to 02-plan.md.

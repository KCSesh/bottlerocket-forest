# Finalize Phase

You are executing the finalize phase of creating an implementation plan.

## Your Goal

Validate the commit plan and write the final implementation-plan.md document.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Analysis: Read `{{workspace}}/01-analyze.md`
- Plan: Read `{{workspace}}/02-plan.md`

## Procedure

### 1. Validate the Plan

Check that:
- [ ] Each commit is atomic and buildable
- [ ] Commits are appropriately sized (target <400 lines)
- [ ] Dependencies are clearly stated
- [ ] Testing approach is documented for each commit
- [ ] Phases group related work logically
- [ ] Critical constraints are mapped to commits
- [ ] Requirements are mapped to commits

### 2. Read Template

```bash
cat {{workspace}}/implementation-plan.md
```

### 3. Fill in Template

Replace template sections with:
- Feature metadata (number, name, status)
- High-level checklist from 02-plan.md
- Detailed commit descriptions from 02-plan.md
- Parallelization notes
- Open questions

### 4. Add Validation Checklist

Include at the end:

```markdown
## Validation

Verify the implementation plan:

- [ ] Each commit is atomic and buildable
- [ ] Commits are appropriately sized (target <400 lines)
- [ ] Dependencies are clearly stated
- [ ] Testing approach is documented for each commit
- [ ] Phases group related work logically
- [ ] Critical constraints are mapped to commits
- [ ] Requirements are mapped to commits
```

### 5. Write Final Document

Write the complete implementation plan to `{{workspace}}/implementation-plan.md`.

## Output Format

The final `{{workspace}}/implementation-plan.md` should follow this structure:

```markdown
# Implementation Plan: <Feature Name>

**Feature**: NNNN-feature-name
**Status**: Draft
**Created**: YYYY-MM-DD

## Overview

<Brief description of the feature and implementation approach>

## High-Level Checklist

- [ ] **Commit 1**: <description>
- [ ] **Commit 2**: <description>
...

## Detailed Commit Descriptions

### Phase 1: Foundation

#### Commit 1: <Title>

<Full commit details from 02-plan.md>

...

## Parallelization Notes

<From 02-plan.md>

## Open Questions

<From 02-plan.md>

## Validation

<Validation checklist>

## Next Steps

After creating the implementation plan:
1. Review with team for feasibility and sizing
2. Adjust based on feedback
3. Begin implementation, checking off commits as completed
4. Update plan if implementation reveals needed changes
5. Use the checklist to track progress
```

## Completion

Call `respond_to_leader("success", "Implementation plan created at {{workspace}}/implementation-plan.md")`.

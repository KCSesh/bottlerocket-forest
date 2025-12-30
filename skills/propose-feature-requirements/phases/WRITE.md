# Write Phase

You are executing the write phase of the propose-feature-requirements skill.

## Your Goal

Fill in the requirements specification using EARS notation with examples and appendices.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Setup info from 01-setup.md
- Concept document at docs/features/NNNN-feature-name/concept.md
- Idea honing document (if exists) at planning/NNNN-feature-name/idea-honing.md

## Procedure

1. Read setup results to get feature info and prefix:
   ```bash
   cat {{workspace}}/01-setup.md
   ```

2. Read concept document for context

3. If idea honing exists, review for insights about edge cases and constraints

4. Fill in requirements.md sections:

### Overview
Write brief description of what this specification covers. Reference concept document.

### Functional Requirements
For each requirement, use EARS format:

```
**WHILE** [state or condition]
**WHEN** [trigger or event]
**THEN** the system **SHALL** [required behavior]
```

Add inline examples only if truly small (1-3 lines):
```
key = "value"
```

For larger examples, note them for appendices.

### Non-Functional Requirements
Add performance, usability, security, or other quality requirements:

```
**WHILE** [operating condition]
**THEN** the system **SHALL** [performance requirement]
```

**Scalability prompts** (ask these for each major operation):
- What happens when there are 1000x more items? 1M files? 10M rows?
- Should memory usage scale with data size, or stay constant?
- What's the acceptable latency? Does it degrade with scale?

These questions surface constraints that will become Critical Constraints in the design phase.
If an operation must be O(1) memory or O(log n) time, state it here.

### Error Handling
Specify how errors should be handled:

```
**WHILE** [operation in progress]
**WHERE** [error condition occurs]
**THEN** the system **SHALL** [error handling behavior]
```

### Appendices
For larger examples that would clutter requirements:
- API response schemas
- Configuration file formats
- Data structures
- Protocol specifications

Each appendix should be clearly labeled:
```
## Appendix A: API Response Schema
## Appendix B: Configuration File Format
```

5. Ensure all requirements use unique IDs with the chosen prefix

## Output Format

Write to `{{workspace}}/02-write.md`:

```markdown
# Write Results

## Requirements File
- Location: docs/features/NNNN-feature-name/requirements.md
- Functional requirements: N
- Non-functional requirements: N
- Error handling requirements: N
- Appendices: N

## Status
[COMPLETE]
```

## Completion

Call `respond_to_leader("success", "<write results>")` when done.

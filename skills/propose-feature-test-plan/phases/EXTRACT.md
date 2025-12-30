# Extract Phase

You are executing the extract phase of the propose-feature-test-plan skill.

## Your Goal

Extract all REQ-* identifiers from requirements.md and all CC-* identifiers from design.md.

## Inputs

- Verification results: Read from `{{workspace}}/verify.json`
- Feature directory path is in that file

## Procedure

1. Read the feature directory from verify.json

2. Extract requirements:

```bash
grep -o 'REQ-[0-9]*' "$FEATURE_DIR/requirements.md" | sort -u
```

3. Extract critical constraints:

```bash
grep -o 'CC-[0-9]*' "$FEATURE_DIR/design.md" | sort -u
```

4. Write extraction results to `{{workspace}}/extract.json`:

```json
{
  "requirements": ["REQ-1", "REQ-2", "REQ-3"],
  "constraints": ["CC-1", "CC-2"],
  "feature_dir": "/path/to/feature/dir"
}
```

## Completion

Call `respond_to_leader("success", "Extracted N requirements and M constraints")` when done.

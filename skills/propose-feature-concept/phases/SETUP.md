# Setup Phase

You are executing the setup phase of the propose-feature-concept skill.

## Your Goal

Prepare the feature directory structure and gather necessary information.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- User has described a feature idea

## Procedure

### 1. Determine Feature Number

Find the next available feature number:

```bash
ls -1d docs/features/[0-9][0-9][0-9][0-9]-* 2>/dev/null | tail -1
```

If no features exist, start with `0001`. Otherwise, increment the last number.

### 2. Create Feature Name

Work with the user to create a concise, descriptive name:
- Use lowercase with hyphens
- Keep it short (2-4 words)
- Make it descriptive

Example: `semantic-search`, `registry-management`, `skill-validation`

### 3. Check for Idea Honing Document

```bash
ls planning/NNNN-feature-name/idea-honing.md 2>/dev/null
```

If it exists, note it for reference when drafting.

### 4. Create Feature Directory

```bash
mkdir -p docs/features/NNNN-feature-name
```

Replace `NNNN` with the four-digit number and `feature-name` with the agreed name.

## Output Format

Write to `{{workspace}}/setup.json`:

```json
{
  "feature_number": "0001",
  "feature_name": "feature-name",
  "feature_dir": "docs/features/0001-feature-name",
  "has_idea_honing": false,
  "idea_honing_path": null
}
```

## Completion

Call `respond_to_leader("success", "<json content>")` with the JSON output.

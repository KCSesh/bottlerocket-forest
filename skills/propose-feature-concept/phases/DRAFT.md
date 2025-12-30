# Draft Phase

You are executing the draft phase of the propose-feature-concept skill.

## Your Goal

Copy the template and work with the user to fill in the concept document.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Setup info: Read from `{{workspace}}/setup.json`
- Template: `docs/features/0000-templates/concept.md`

## Procedure

### 1. Copy Concept Template

```bash
cp docs/features/0000-templates/concept.md <feature_dir>/
```

Use the feature_dir from setup.json.

### 2. Fill in Concept Document

Work with the user to complete `concept.md` as a narrative:

**Frontmatter**
- Set `feature:` to `NNNN-feature-name`
- Set `status:` to `proposed`
- Add optional `tracking-issue:` if applicable

**Problem Section**
- Paint a picture of the current situation
- Describe the pain this causes
- Explain why this matters

**Solution Section**
- Describe what users will experience
- Focus on "what" and "why" rather than "how"
- Keep it narrative, not a list

**How It Works Section**
- Tell the story of using the feature
- Walk through the workflow naturally
- Show real usage, not abstract steps

**Benefits Section**
- Explain the value provided
- Connect back to the problem
- Show what becomes possible

**Technical Notes Section**
- Brief constraints or considerations
- Keep it short - details go in design.md

If idea honing document exists (check setup.json), reference it for valuable material.

## Output Format

Write the completed concept.md to the feature directory.

## Completion

Call `respond_to_leader("success", "Draft complete at <path>")` when done.

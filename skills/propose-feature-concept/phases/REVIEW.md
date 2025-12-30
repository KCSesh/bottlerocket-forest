# Review Phase

You are executing the review phase of the propose-feature-concept skill.

## Your Goal

Review the concept document for narrative flow and quality.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Setup info: Read from `{{workspace}}/setup.json`
- Concept document: Read from feature_dir/concept.md

## Procedure

### 1. Review for Narrative Flow

Ensure the document:
- Reads like a story, not a specification
- Avoids bullet points and numbered lists where possible
- Focuses on user experience and value
- Explains "why" before "what"

### 2. Check Common Issues

**Too technical**: If the concept reads like a design doc, note that it should refocus on the problem and user experience.

**Too abstract**: If the concept is vague, note that it needs concrete examples of the pain point.

**List-heavy**: If there are many bullet points, note that it should be rewritten as narrative prose.

### 3. Validation

Verify the concept was created correctly:

```bash
# Check directory exists
ls -la <feature_dir>/

# Verify concept file exists
ls <feature_dir>/concept.md

# Check it has content
cat <feature_dir>/concept.md
```

## Output Format

Write to `{{workspace}}/review.md`:

```markdown
# Review: <Feature Name>

## Status
- [ ] Reads like a story
- [ ] Avoids excessive lists
- [ ] Focuses on user experience
- [ ] Explains "why" before "what"

## Issues Found
- [Issue 1 if any]
- [Issue 2 if any]

## Recommendations
- [Recommendation 1 if any]

## Next Steps
After creating the concept:
1. Review and refine the narrative
2. Get feedback on whether the feature is valuable
3. Once concept is solid, move to requirements using `propose-feature-requirements` skill
```

## Completion

Call `respond_to_leader("success", "<review content>")` with the review markdown.

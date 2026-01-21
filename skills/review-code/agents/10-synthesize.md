# Synthesis Subagent

This file contains instructions for a subagent collaborating on reviewing a PR.

You have a specific goal to contribute to the overall review, which is driven by the PRINCIPLES.md you will receive. Fulfill your specific purpose within the context of the broad problem. Constrain yourself not to solve problems outside your scope—other agents handle those. If there are organizational challenges to completing your task, report them in your status response.

## Your Purpose

Synthesize all analysis into a final review document. You are the last phase—your output is what the user sees. Organize findings by category; do not rank or prioritize.

## You Will Receive

**Files:**
- `PRINCIPLES.md` - Review ethos
- This instruction file
- `01-questions.md` - Question backlog
- `02-answers.md` - All answered questions
- `04-holistic-pre.md` - Pre-assessment
- `05-holistic-post.md` - Post-assessment
- `filtered-implications.md` - Categorized implications

**Data:**
- `workspace`: path to write output

## Task

1. Read filtered-implications.md (primary input)
2. Add context from other artifacts
3. Organize into readable document by category
4. Write output to `{workspace}/REVIEW.md`

## Output Format

```markdown
# Code Review: <PR title>

## Summary
<One paragraph: what the PR does>

## Actionable Comments

For each surfaced implication, provide a ready-to-use PR comment entry:

### Correctness

| SHA | File | Line | Comment |
|-----|------|------|---------|
| `abc123` | `path/to/file.rs` | 42 | <observation + consequence, ready to post> |

### Assumptions

| SHA | File | Line | Comment |
|-----|------|------|---------|

### Type System Opportunities

| SHA | File | Line | Comment |
|-----|------|------|---------|

### Questions for Author

| SHA | File | Line | Question |
|-----|------|------|----------|

## Cross-Cutting Observations
<Patterns from holistic-post that span multiple commits>

## Answered Questions (Reference)
<Key Q&A that provides context for the concerns>

## Artifact References
- Questions: `01-questions.md`
- Answers: `02-answers.md`
- Commit reviews: `03-commit-*.md`
- Holistic: `04-holistic-pre.md`, `05-holistic-post.md`
- Implications: `filtered-implications.md`
```

## Guidance

**Actionable output.** Each row should have enough information to post a PR comment:
- SHA: The commit to comment on
- File: The file path in the diff
- Line: The line number to attach the comment to
- Comment/Question: Ready-to-post text

Parse SHA and file:line from the enriched citations in filtered-implications.md (format: `{sha}:path/to/file.rs:line`).

**No severity ranking.** Items within each category are flat lists.

**No approval recommendation.** The skill surfaces concerns; humans decide action.

**No "blocking" language.** Every surfaced concern is worth the author seeing.

Be specific. "Consider refactoring" is not actionable. "Extract lines 42-87 into a function" is.

No praise. "LGTM" is acceptable if no concerns. "Great work" is not.

## Response to Caller

```
OUTPUT: REVIEW.md
STATUS: ok | problems
NOTES: <if problems>
```

Use error reporting mechanism if context insufficient or task ambiguous.

**Do not commit artifacts.** Files in `planning/` are working documents.

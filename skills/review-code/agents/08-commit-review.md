# Commit Analysis Subagent

This file contains instructions for a subagent collaborating on reviewing a PR.

You have a specific goal to contribute to the overall review, which is driven by the PRINCIPLES.md you will receive. Fulfill your specific purpose within the context of the broad problem. Constrain yourself not to solve problems outside your scope—other agents handle those. If there are organizational challenges to completing your task, report them in your status response.

## Your Purpose

Analyze a single commit as an atomic unit. You have access to the Q&A document with answered questions—use it. Focus on verifying this commit's claims against its actual behavior and documenting observations.

## Judgment Boundary

State facts and consequences. Do NOT assess:
- Whether something is "acceptable" or "appropriate"
- Whether a tradeoff is "worth it"
- Whether an issue is "minor" or "blocking"

**Example violation:** "Using snafu::Whatever is acceptable for a build tool"
**Correct form:** "Uses snafu::Whatever instead of typed errors. Consequence: error context is stringly-typed; callers cannot match on error variants."

## You Will Receive

**Files:**
- `PRINCIPLES.md` - Review ethos
- This instruction file
- `01-questions.md` - Refined question backlog
- `02-answers.md` - All answered questions

**Data:**
- `workspace`: path to write output
- `repo_path`: path to repository with PR checked out
- `base_ref`: base branch name
- `commit_sha`: this commit's SHA
- `commit_message`: this commit's message
- `commit_number`: which commit this is (1-indexed)
- `total_commits`: total commits in PR
- `all_commit_messages`: list of all commit messages in order

Get commit diff via `git show {commit_sha}` or `git diff {commit_sha}^..{commit_sha}`. Read files directly from filesystem for full context.

## Task

1. Identify questions from 02-answers.md that apply to this commit (cite by ID only)
2. Understand what this commit claims to do
3. Verify the code actually does what it claims
   - If verification requires domain knowledge you lack, state what you checked and what remains unverified
   - Never claim verification you cannot demonstrate from code
4. Document observations with specific file:line references
5. Every consequence must follow from the observation—do not introduce external judgments
6. If you discover NEW questions not in the Q&A doc, note them (but don't answer)
7. Write output to `{workspace}/03-commit-{NN}.md`

## Output Format

Write `03-commit-NN.md`:

```markdown
# Commit {N} of {M}: <commit subject>

## Relevant Q&A
<List question IDs that apply: Q1, Q5, Q12. Do NOT summarize answers.>

## Claimed Purpose
<What the commit message says this does>

## Actual Behavior
<What the code actually does—verify it matches the claim>

## Observations

### <Category: e.g., Error Handling, State Management, API Design>

- OBSERVATION: <what the code does—factual>
- CONSEQUENCE: <what behavior/state/constraint follows from this—factual>
- CITATION: `{commit_sha}:path/to/file.rs:42`

CONSEQUENCE must be factual, not evaluative:
✅ "Errors cannot be programmatically distinguished"
✅ "Callers must handle all error types uniformly"
❌ "This is acceptable for a build tool"
❌ "This trade-off is reasonable"

### <Next Category>

- OBSERVATION: <what the code does>
- CONSEQUENCE: <what follows from this>
- CITATION: `path/to/file.rs:87`

## Runtime Checks (Potential Compile-Time Alternatives)

- OBSERVATION: <runtime check or stringly-typed value>
- ALTERNATIVE: <what compile-time mechanism exists (newtype, typestate, NonZero, etc.)>
- CITATION: `{commit_sha}:path/to/file.rs:42`

## New Questions Discovered
<Questions that arose during analysis, not in Q&A doc>

## Questions for Author

Observations where intent cannot be determined from code or commit message.

- OBSERVATION: <what the code does>
- CONSEQUENCE: <possible interpretations or what clarification would resolve>
- CITATION: `{commit_sha}:path/to/file.rs:42`
```

## Response to Caller

```
OUTPUT: 03-commit-{NN}.md
STATUS: complete | incomplete | blocked
NOTES: <if incomplete or blocked, explain what's missing>
```

Status meanings:
- complete: analysis finished, observations documented
- incomplete: analysis partial due to context limits
- blocked: cannot proceed (explain in NOTES)

Use error reporting mechanism if context insufficient or task ambiguous.

**Do not commit artifacts.** Files in `planning/` are working documents.

# Research Subagent

This file contains instructions for a subagent collaborating on reviewing a PR.

You have a specific goal to contribute to the overall review, which is driven by the PRINCIPLES.md you will receive. Fulfill your specific purpose within the context of the broad problem. Constrain yourself not to solve problems outside your scope—other agents handle those. If there are organizational challenges to completing your task, report them in your status response.

## Your Purpose

Answer a RESEARCH question using the `deep-research` skill. These are questions requiring deep understanding like "How does the update system work?" or "What's the architecture of X?"

## You Will Receive

**Files:**
- `PRINCIPLES.md` - Review ethos
- This instruction file
- `skills/deep-research/SKILL.md` - Deep-research skill instructions

**Data:**
- `workspace`: path to write output
- `question_id`: identifier for this question
- `question`: the research question to answer

## Task

1. Use the deep-research skill to answer the question
2. Ensure answer builds understanding progressively
3. Write output to `{workspace}/answers/research-{question_id}.md`

## Execution

```
USING SKILL "deep-research"
QUESTION: <the question>
```

Follow the deep-research skill procedure. The skill will create its own workspace; copy the final output to the expected location.

## Output Format

```markdown
# Research: Q{question_id}

## Question
<the question>

## Summary
<brief answer>

## Detailed Understanding
<from deep-research output>

## Key Citations
- <file:line> - <what it shows>
```

## Response to Caller

```
OUTPUT: answers/research-{question_id}.md
STATUS: ok | problems
NOTES: <if problems>
```

Use error reporting mechanism if context insufficient or task ambiguous.

**Do not commit artifacts.** Files in `planning/` are working documents.

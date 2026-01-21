# Fact-Finding Subagent

This file contains instructions for a subagent collaborating on reviewing a PR.

You have a specific goal to contribute to the overall review, which is driven by the PRINCIPLES.md you will receive. Fulfill your specific purpose within the context of the broad problem. Constrain yourself not to solve problems outside your scope—other agents handle those. If there are organizational challenges to completing your task, report them in your status response.

## Your Purpose

Answer a FACT question using the `fact-find` skill. These are questions with concrete, citable answers like "What does function X do?" or "What's the existing behavior of Y?"

## You Will Receive

**Files:**
- `PRINCIPLES.md` - Review ethos
- This instruction file
- `skills/fact-find/SKILL.md` - Fact-find skill instructions

**Data:**
- `workspace`: path to write output
- `question_id`: identifier for this question
- `question`: the fact question to answer

## Task

1. Use the fact-find skill to answer the question
2. Ensure answer includes citations
3. Write output to `{workspace}/answers/fact-{question_id}.md`

## Execution

```
USING SKILL "fact-find"
QUESTION: <the question>
```

Follow the fact-find skill procedure completely.

## Output Format

```markdown
# Fact: Q{question_id}

## Question
<the question>

## Answer
<the answer>

## Citations
- <file:line> - <relevant quote or description>
```

## Response to Caller

```
OUTPUT: answers/fact-{question_id}.md
STATUS: ok | problems
NOTES: <if problems>
```

Use error reporting mechanism if context insufficient or task ambiguous.

**Do not commit artifacts.** Files in `planning/` are working documents.

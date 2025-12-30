# Scout Phase

You are executing the scout phase of a research task.

## Goal

Discover what exists and formulate sub-questions. Do NOT answer them yet.

## Inputs

- Workspace: `{{workspace}}`
- Question: Read from `{{workspace}}/question.txt`

## Procedure

1. Read the question from `{{workspace}}/question.txt`

2. Run broad searches to find relevant areas:
   ```bash
   crumbly search "topic overview"
   crumbly search "topic architecture"
   ```

3. Skim top 2-3 results for structure only:
   - Note file names and section headings
   - Identify key terms and component names
   - Find where detailed information lives
   - Do NOT read implementation details

4. Write findings to `{{workspace}}/00-scout.md`

## Output Format

Write to `{{workspace}}/00-scout.md`:

```markdown
# Scout: <Original Question>

## Key Concepts Discovered
- [Concept 1]: [Brief description]
- [Concept 2]: [Brief description]

## Relevant Files Found
- `path/to/file.md` - [What it covers]
- `path/to/code.rs` - [What it covers]

## Terminology
- [Term]: [Definition as used in this codebase]

## Sub-Questions

### 1. [Sub-question text]
- **Type:** fact-find | research-document
- **Why:** [Why this classification]
- **Key files:** [Files likely to answer this]

### 2. [Sub-question text]
- **Type:** fact-find | research-document
- **Why:** [Why this classification]
- **Key files:** [Files likely to answer this]
```

## Sub-Question Classification

| If the sub-question... | Type |
|------------------------|------|
| Has a concrete, specific answer | fact-find |
| Asks "what is X" or "where is Y" | fact-find |
| Asks "how does X work" | research-document |
| Involves multiple components interacting | research-document |
| Would need 3+ source files to answer | research-document |

## Completion

Call `respond_to_leader("success", "Scout complete - N sub-questions identified")` when done.

# Search Phase

You are executing the search phase of a fact-find task.

## Your Goal

Find relevant files that can answer the user's question. Do NOT answer the question yet.

## Inputs

- Workspace: provided via context_data
- Question: Read from `<workspace>/question.txt`

## Procedure

1. Read the question:
   ```bash
   cat <workspace>/question.txt
   ```

2. Run focused crumbly searches with key terms:
   ```bash
   crumbly search "specific terms from question"
   ```

3. Check top 3-5 results for relevance

4. If documentation is insufficient, search source code:
   ```bash
   # IMPORTANT: Always scope searches to specific directories!
   rg "search_term" --type rust bottlerocket/sources/
   ```

## Output Format

Write to `<workspace>/00-search.md`:

```markdown
# Search Results: <Question Summary>

## Relevant Files

1. `path/to/file.md` - Brief reason why relevant
2. `path/to/other.rs` - Brief reason why relevant

## Search Strategy

- Queries used: "query1", "query2"
- Source: documentation | source code | both
```

## Completion

Call `respond_to_leader("success", "Search complete")` when done.

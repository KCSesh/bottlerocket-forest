# Research Phase

You are answering a specific sub-question from a research task.

## Inputs

- Workspace: `{{workspace}}`
- Sub-question: `{{subquestion}}`
- Type: `{{subquestion_type}}`
- Key files: `{{key_files}}`
- Output file: `{{output_file}}`

## Procedure

1. Read the scout file `{{workspace}}/00-scout.md` for context

2. Research the sub-question using the key files identified

3. For **fact-find** type:
   - Search and read the relevant files
   - Write a direct answer with inline citations

4. For **research-document** type:
   - Create a subdirectory and recurse with own scout phase
   - The subdirectory's FINAL.md becomes the answer

5. Write your answer to `{{workspace}}/{{output_file}}`

## Output Format

Write to `{{workspace}}/{{output_file}}`:

```markdown
# {{subquestion}}

<Direct answer with inline citations>

The kit directory must contain a `Twoliter.toml` file <sup>[1]</sup> and a `Cargo.toml` 
that lists packages as dependencies <sup>[2]</sup>.

## Sources

<sup>[1]</sup> [`path/to/file.md`](../path/to/file.md) - Section name
<sup>[2]</sup> [`path/to/other.rs`](../path/to/other.rs) - Function or context
```

## If You Cannot Answer

If you cannot answer from available sources:
- Write "Could not determine from available sources"
- Note what you searched
- Do NOT guess

## Completion

Call `respond_to_leader("success", "Sub-question answered")` when done.

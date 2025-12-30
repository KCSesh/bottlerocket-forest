# Answer Phase

You are executing the answer phase of a fact-find task.

## Your Goal

Read the identified files and formulate a concise answer with proper citations.

## Inputs

- Workspace: provided via context_data
- Question: `<workspace>/question.txt`
- Search results: `<workspace>/00-search.md` (provided via context_files)

## Procedure

1. Read the question and search results

2. Read the relevant files identified in search results:
   ```bash
   cat path/to/file.md
   # Or for targeted reading:
   grep -n "relevant terms" path/to/file.md
   ```

3. Formulate a direct, concise answer (2-4 sentences typically)

## Output Format

Write to `<workspace>/FINAL.md` using this exact format:

```markdown
<Direct answer with inline superscript citations like this <sup>[1]</sup>.>

## Sources

<sup>[1]</sup> [`path/to/file.md`](../path/to/file.md)
- What this source provided

<sup>[2]</sup> [`path/to/other.rs`](../path/to/other.rs)
- What this source provided

---

<Research Quality Indicator - one of:>
- ✅ **Answered from documentation** - Found in README files, design docs, or narrative documentation.
- ⚠️ **Answered from source code** - Had to read implementation files due to insufficient documentation.
- 🔍 **Partial documentation** - Required both docs and source code to answer fully.
```

## Citation Guidelines

- Use `<sup>[1]</sup>`, `<sup>[2]</sup>`, etc. inline with facts
- Paths relative to the target repository
- Markdown links for file paths
- GitHub URLs for cross-repo references
- Brief bullet points describing what each source provided

## Validation

A good fact-find response:
- ✓ Directly answers the specific question
- ✓ Concise (2-4 sentences typically)
- ✓ Superscript citations inline
- ✓ Sources section with numbered references
- ✓ Research Quality Indicator at end
- ✓ No unnecessary context or explanation

## Completion

Call `respond_to_leader("success", "Answer complete")` when done.

# Fact Find Phase

You are executing a fact-find task: search for information and provide a cited answer.

## Your Goal

Find relevant files and formulate a concise answer with proper citations.

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

5. Read the relevant files:
   ```bash
   cat path/to/file.md
   # Or for targeted reading:
   grep -n "relevant terms" path/to/file.md
   ```

6. Formulate a direct, concise answer (2-4 sentences typically)

## Output Format

Write to `<workspace>/ANSWER.md` using this exact format:

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

Before completing, verify your answer:
- ✓ Directly answers the specific question
- ✓ Concise (2-4 sentences typically)
- ✓ Superscript citations inline
- ✓ Sources section with numbered references
- ✓ Research Quality Indicator at end
- ✓ No unnecessary context or explanation

## Completion

Call `respond_to_leader("success", "Answer complete")` when done.

# Assemble Phase

You are assembling research findings into a final document.

## Inputs

- Workspace: `{{workspace}}`
- Research files: All `NN-*.md` files in workspace (passed via context_files)

## Procedure

1. Read all research files from context

2. Synthesize into a coherent document following the structure below

3. Renumber citations sequentially across all sources

4. Write to `{{workspace}}/FINAL.md`

## Output Structure

```markdown
# [System Name]

**Keywords:** keyword1, keyword2, keyword3

## Overview

[Visual summary - ASCII diagram or table showing the whole system]

[1-2 sentences of context]

## [Main Topic]

[Synthesize from sub-question answers]

### [Subtopic]

**Goal:** [One sentence]

[Content with citations carried forward from sub-questions]

## Appendix: [Detailed Reference]

[Dense details, tables, configuration options]

## Conflicts & Resolutions

[If sources disagreed, document how you resolved it]
(Write "None" if no conflicts found)

## Sources

[Consolidated numbered citations from all sub-questions]
```

## Writing Guidelines

- Lead overview with a visual (diagram or table)
- Use tables for 3+ similar items
- Every factual claim needs a `<sup>[N]</sup>` citation
- Renumber citations sequentially (1, 2, 3...)

## Quality Indicator

End with one of:
- ✅ **Answered from documentation**
- ⚠️ **Answered from source code**
- 🔍 **Partial documentation**
- ❓ **Gaps remain**

## Completion

Call `respond_to_leader("success", "Document assembled")` when done.

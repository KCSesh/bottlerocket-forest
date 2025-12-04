---
name: research-document
description: Create educational documents that build understanding progressively with citations
---

# Research Document

A systematic approach to creating educational documentation that builds understanding progressively.

## Purpose

Creates in-depth explanatory documents that:
- Build understanding from fundamentals to specifics
- Use progressive disclosure to guide the reader
- Provide complete citations for all information
- Use visual summaries and tables for dense information

## When to Use

- User asks for comprehensive explanations of systems or features
- Need to document how components work end-to-end
- Creating educational content about architecture or processes
- Questions like "Explain how X works" or "What is the Y process?"

For quick factual lookups, use **fact-find** instead.

## Prerequisites

**YOU MUST COMPLETE THESE FIRST:**

1. Run `./seed-forest.sh` from forest root

**If this fails, STOP and fix the error.**

## Procedure

💡 TIP: If you have todolist functionality, create a task list for these steps.

### 1. Plan the Document Structure

**CRITICAL: Overview-first, details-last.**

Before searching, plan the information architecture:

1. **Overview** - Visual summary (diagram, flowchart, or table) showing the whole system at a glance
2. **Fundamentals** - Underlying concepts/models the reader needs to understand the specifics
3. **Main Body** - How the system works, organized by concept not by component
4. **Reference/Appendix** - Dense details, tables of components, configuration options

**Information placement rules:**
- If it's a list of similar items → table, not paragraphs
- If it's implementation detail → appendix, not main body
- If it requires prerequisite knowledge → explain the prerequisite first

❌ **Anti-patterns to avoid:**

| Anti-pattern | Problem | Fix |
|--------------|---------|-----|
| Verbose item descriptions | Paragraphs for each service/component | Use tables |
| Implementation-first | Jumping into details before explaining the model | Add Fundamentals section |
| Flat structure | All details in main body | Move reference material to appendix |
| Text-only overview | Prose summary that's hard to scan | Lead with diagram or table |

✅ **Good structure:**
- Reader sees the whole system in 10 seconds (overview visual)
- Reader understands the model before seeing specifics (fundamentals)
- Reader can skip to appendix for details without wading through prose

### 2. Research with Multiple Searches

```bash
# High-level architecture and purpose
sembly search "system-name architecture overview purpose"

# Implementation details
sembly search "system-name implementation components"

# Configuration and behavior
sembly search "system-name configuration settings behavior"
```

### 3. Read and Organize Information

Read identified files and categorize by document section:

| Information Type | Goes In |
|------------------|--------|
| Purpose, problem solved | Overview |
| Underlying models, concepts | Fundamentals |
| How things work together | Main body |
| Lists of components/services | Tables (main body or appendix) |
| Configuration options, flags | Appendix |
| Edge cases, advanced topics | Appendix |

### 4. If Documentation Is Insufficient

Sembly indexes documentation, not source code. When docs don't fully answer the question:

1. **Use what you learned** - Documentation often names components, files, or concepts
2. **Search code informed by docs** - Use discovered names/paths to target your search:
   ```bash
   rg "component_name" --type rust
   find . -name "*component_name*"
   ```
3. **Note the gap** - Reflect in Research Quality Indicator

### 5. Structure the Document

```markdown
# [System Name]

**Keywords:** keyword1, keyword2, keyword3

## Overview

[Visual summary - diagram, flowchart, or table showing the whole system]

| Component | Purpose | Result |
|-----------|---------|--------|
| ... | ... | ... |

[1-2 sentences of context. No more.]

## How [Underlying Model] Works

[Explain the conceptual model the reader needs.
This section answers: "What do I need to understand before the specifics make sense?"]

### [Key Concept 1]

[Brief explanation with example if helpful]

### [Key Concept 2]

[Brief explanation]

## [Main Topic] Reference

[Organized by concept, not by component. Use tables for lists.]

### [Subtopic]

**Goal:** [One sentence]

**Key components:**

| Component | Purpose |
|-----------|---------|
| ... | ... |

[Brief prose only if needed to explain interactions]

## Appendix: [Detailed Reference]

[Dense details, configuration options, etc.]

### [Detail Category]

| Item | Description |
|------|-------------|
| ... | ... |

## Sources

[Numbered citations]
```

### 6. Writing Guidelines

**Overview section:**
- Lead with a visual (ASCII diagram, flowchart, or summary table)
- Maximum 2-3 sentences of prose after the visual
- Reader should grasp the whole system in 10 seconds

**Fundamentals section:**
- Explain the model/concepts before using them
- Use concrete examples
- Keep it short—just enough to understand what follows

**Main body:**
- Organize by concept, not by component
- Use tables when listing 3+ similar items
- Prose explains relationships and flow; tables list facts
- Each subsection should have a clear "Goal:" statement

**Tables vs prose:**

```markdown
❌ BAD - Verbose paragraphs:
"The foo service is responsible for initializing the bar subsystem. 
It runs early in boot and creates the necessary directories. The baz 
service handles network configuration. It waits for foo to complete..."

✅ GOOD - Table:
| Service | Purpose |
|---------|---------|
| foo | Initialize bar subsystem, create directories |
| baz | Configure network (after foo) |
```

**Appendix:**
- Put detailed reference material here
- Configuration options, all flags, advanced topics
- Readers who need details can find them; others skip it

### 7. Add Complete Citations

Use superscript citations inline, with a Sources section at the end:

```markdown
The system uses an A/B partition scheme <sup>[1]</sup>.

## Sources

<sup>[1]</sup> [`SECURITY_FEATURES.md`](https://github.com/org/repo/blob/develop/SECURITY_FEATURES.md)
- Dual partition sets for updates
```

**Citation path guidelines:**
- Use paths relative to the repository where documentation will live
- Make file paths into markdown links
- For same repo: `[path/to/file.md](../path/to/file.md)`
- For other repos: `[FILE.md](https://github.com/org/repo/blob/develop/FILE.md)`

### 8. Validate Document Quality

**Structure checklist:**
- ✓ Overview has a visual (diagram or table) before prose
- ✓ Fundamentals section explains underlying model
- ✓ Main body organized by concept, not component
- ✓ Dense lists use tables, not paragraphs
- ✓ Detailed reference material in appendix

**Conciseness checklist:**
- ✓ No verbose paragraphs describing list items
- ✓ Prose explains relationships; tables list facts
- ✓ Each section earns its length

**Progressive disclosure:**
- ✓ Reader can understand overview without reading further
- ✓ Fundamentals come before they're needed
- ✓ Details are available but not forced on reader

**Citations:**
- ✓ Superscript inline references throughout
- ✓ Numbered Sources section at end

## Research Quality Indicator

End your response with:

- ✅ **Answered from documentation** - Fully answered from README files, design docs, or narrative documentation.
- ⚠️ **Answered from source code** - Had to read implementation files because documentation was insufficient.
- 🔍 **Partial documentation** - Required both docs and source code to answer fully.

**Guidelines:**
- If you read more than 2-3 source/config files, it's NOT "from documentation"
- README files and markdown docs count as documentation
- Systemd units, .rs files, .spec files, .toml configs are source code
- Reflects whether someone else could answer from docs alone

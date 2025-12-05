---
name: research-document
description: Create educational documents that build understanding progressively with citations
---

# Research Document

A systematic approach to creating educational documentation through tiered research.

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

## Research Model: Scout → Decompose → Research → Assemble

Research happens in tiers, not all at once. All artifacts go to the filesystem.

```
┌─────────────────────────────────────────────────────────────────┐
│                     TIERED RESEARCH MODEL                       │
└─────────────────────────────────────────────────────────────────┘

     TIER 0: SCOUT                    TIER 1: FOCUSED RESEARCH
     ─────────────────                ────────────────────────
     
     ┌─────────────┐                  ┌─────────────┐
     │   Initial   │                  │ Sub-question│──▶ Cited answer
     │   Question  │                  │ (fact-find) │    (short)
     └──────┬──────┘                  └─────────────┘
            │                         ┌─────────────┐
            ▼                         │ Sub-question│──▶ Cited answer
     ┌─────────────┐    Decompose     │ (fact-find) │    (short)
     │    Scout    │────────────────▶ └─────────────┘
     │   Search    │                  ┌─────────────┐
     └──────┬──────┘                  │ Sub-question│──▶ RECURSE
            │                         │ (research)  │    (own scout/decompose)
            ▼                         └─────────────┘
     ┌─────────────┐                         │
     │ Write to:   │                         ▼
     │ planning/   │                  ┌─────────────┐
     │ <slug>/     │                  │  Assemble   │──▶ Final document
     │ 00-scout.md │                  │  from files │
     └─────────────┘                  └─────────────┘
```

**Why this works:** 
- Agents struggle to admit uncertainty mid-task. Scouting first identifies gaps before the pressure to produce output.
- Writing to filesystem enables collaboration between agents (or context window resets for single-agent systems).
- Classifying sub-questions prevents "too big" questions from getting shallow treatment.

## Workspace Setup

All research artifacts go to `planning/<question-slug>/`:

```
planning/
└── how-twoliter-builds-kits/
    ├── 00-scout.md           # Scout findings + sub-questions
    ├── 01-kit-structure.md   # Sub-question answer (fact-find)
    ├── 02-build-command.md   # Sub-question answer (fact-find)
    ├── 03-buildsys/          # Sub-question that needed recursion
    │   ├── 00-scout.md
    │   ├── 01-spec-parsing.md
    │   └── 02-docker-build.md
    └── FINAL.md              # Assembled document
```

**Slug format:** lowercase, hyphens, descriptive (e.g., `how-twoliter-builds-kits`)

### For Multi-Agent Systems

- **Lead agent**: Scout, decompose, assemble
- **Subagents**: Each sub-question becomes a task; subagent writes answer to the workspace
- Subagents can recursively spawn if their question is too big

### For Single-Agent Systems

Execute tiers sequentially, using the filesystem as your "memory":
1. Scout and write `00-scout.md`
2. Research each sub-question, writing `01-*.md`, `02-*.md`, etc.
3. Read all files back and assemble `FINAL.md`

This allows context window resets between phases if needed.

## Procedure

### Phase 1: Scout (Learn the Shape)

**Goal:** Understand what you're dealing with. Write findings to `00-scout.md`.

```bash
mkdir -p planning/<question-slug>
```

```bash
# Broad search to find relevant areas
sembly search "system-name overview"
sembly search "system-name architecture"
```

Read 2-3 top results. Capture in `00-scout.md`:

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
...
```

**Sub-question classification:**

| If the sub-question... | Type | Action |
|------------------------|------|--------|
| Has a concrete, specific answer | fact-find | Answer in 1-2 paragraphs |
| Asks "what is X" or "where is Y" | fact-find | Answer in 1-2 paragraphs |
| Asks "how does X work" | research-document | Recurse (own scout/decompose) |
| Involves multiple components interacting | research-document | Recurse |
| Would need 3+ source files to answer | research-document | Recurse |

**Recursion check:** If more than 2 sub-questions are type `research-document`, consider whether the original question is too broad.

### Phase 2: Research Sub-Questions

For each sub-question, write to `NN-<slug>.md`:

**For fact-find sub-questions:**

```markdown
# <Sub-Question>

<Direct answer with inline citations>

The kit directory must contain a `Twoliter.toml` file <sup>[1]</sup> and a `Cargo.toml` 
that lists packages as dependencies <sup>[2]</sup>.

## Sources

<sup>[1]</sup> [`twoliter/README.md`](../twoliter/README.md) - Kit requirements section
<sup>[2]</sup> [`kits/bottlerocket-core-kit/Cargo.toml`](../kits/bottlerocket-core-kit/Cargo.toml) - Example kit manifest
```

**For research-document sub-questions:**

Create a subdirectory and recurse:

```
planning/how-twoliter-builds-kits/03-buildsys/
├── 00-scout.md
├── 01-....md
└── FINAL.md
```

The sub-question's `FINAL.md` becomes the answer.

**If you can't answer from sources:**
- Write "Could not determine from available sources"
- Note what you searched
- Do NOT guess

### Phase 3: Assemble Final Document

Read all sub-question answers from the workspace. Combine into `FINAL.md`:

```markdown
# [System Name]

**Keywords:** keyword1, keyword2, keyword3

## Overview

[Visual summary - diagram or table showing the whole system]

[1-2 sentences of context]

## How [Underlying Model] Works

[Synthesize from sub-question answers about fundamentals]

## [Main Topic]

[Synthesize from sub-question answers about the core process]

### [Subtopic]

**Goal:** [One sentence]

[Content with citations carried forward from sub-questions]

## Appendix: [Detailed Reference]

[Dense details, tables, configuration options]

## Sources

[Consolidated numbered citations from all sub-questions]
```

### Writing Guidelines

**Overview section:**
- Lead with a visual (ASCII diagram, flowchart, or summary table)
- Maximum 2-3 sentences of prose after the visual
- Reader should grasp the whole system in 10 seconds

**Tables vs prose:**
- If listing 3+ similar items → use a table
- Prose explains relationships; tables list facts

**Citations:**
- Use `<sup>[1]</sup>` inline with facts
- Every factual claim needs a citation
- Consolidate sources at the end
- When assembling, renumber citations sequentially

## Validation Checklist

Before finalizing:

- [ ] Every factual claim has a citation
- [ ] No claims marked "could not determine" remain unexplained
- [ ] Overview has a visual before prose
- [ ] Dense lists use tables, not paragraphs
- [ ] Sources section has all referenced citations
- [ ] All sub-question files exist in workspace

## Research Quality Indicator

End your document with:

- ✅ **Answered from documentation** - Fully answered from README files, design docs, or narrative documentation.
- ⚠️ **Answered from source code** - Had to read implementation files because documentation was insufficient.
- 🔍 **Partial documentation** - Required both docs and source code to answer fully.
- ❓ **Gaps remain** - Some sub-questions could not be answered; noted in document.

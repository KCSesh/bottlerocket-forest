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

## Research Model: Scout → Decompose → Research → Assemble → Verify

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
     │ <slug>/     │                  │  Assemble   │──▶ Draft document
     │ 00-scout.md │                  │  from files │
     └─────────────┘                  └──────┬──────┘
                                             │
                                             ▼
                                      ┌─────────────┐
                                      │   Verify    │──▶ Final document
                                      │  citations  │
                                      └─────────────┘
```

**Why this works:** 
- Agents struggle to admit uncertainty mid-task. Scouting first identifies gaps before the pressure to produce output.
- Writing to filesystem enables collaboration between agents (or context window resets for single-agent systems).
- Classifying sub-questions prevents "too big" questions from getting shallow treatment.
- Verification catches hallucinated or unsupported claims before finalizing.

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

- **Lead agent**: Scout, decompose, assemble, coordinate verification
- **Subagents**: Each sub-question becomes a task; subagent writes answer to the workspace
- **Verifiers**: Lightweight agents that check citations (no tools needed)
- Subagents can recursively spawn if their question is too big (up to depth limit)

### Plan/Todolist Discipline

If you have todolist functionality, use it to enforce workflow completion:

**Before each phase, enumerate the work in your plan:**
- Phase 2: One item per sub-question from scout
- Phase 4: One item per citation to verify

**The pattern:** Enumerate → Execute → Confirm count matches

This prevents "doing some" instead of "doing all." The plan is a contract—once you've written "Verify citation [7]" you're committed to doing it.

### For Single-Agent Systems

Execute tiers sequentially, using the filesystem as your "memory":
1. Scout and write `00-scout.md`
2. Research each sub-question, writing `01-*.md`, `02-*.md`, etc.
3. Read all files back and assemble `FINAL.md`
4. Self-check citations (less reliable but still valuable)

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

**Depth limit:** Maximum recursion depth is 2 levels. If a sub-sub-question would need its own recursion, either:
- Force it to fact-find treatment (accept less depth)
- Flag it for human review
- Split the original question into multiple research tasks

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

### Phase 3: Assemble Draft Document

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

## Conflicts & Resolutions

[If sources disagreed, document how you resolved it]
- [Topic]: Source A said X, Source B said Y. Resolution: [reasoning]

(Write "None" if no conflicts found)

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

**Long documents (output token limits):**

Claude has output limits (~4K-8K tokens per response). For documents longer than ~10 pages:
- Write section-by-section, appending to FINAL.md
- Or: generate detailed outline first, then expand each section separately
- Don't try to generate the entire document in one response

### Phase 4: Verify

**Goal:** Confirm citations support their claims. Catch unsupported statements.

#### Step 1: Enumerate Citations (REQUIRED)

Before spawning any verifiers, you MUST enumerate all citations in your plan/todolist:

1. Count citations in FINAL.md (e.g., 12 citations)
2. Add one plan item per citation:
   - [ ] Verify [1]: "claim text preview..."
   - [ ] Verify [2]: "claim text preview..."
   - ... (all 12)
3. Add final item: "Confirm 12/12 verifiers completed"

**Do NOT proceed to spawning until your plan has N+1 items** (N citations + confirmation).

This prevents the failure mode of "verifying a few representative citations" instead of all of them.

#### Step 2: Spawn Verifiers

**For Multi-Agent Systems:**

Spawn lightweight verifiers with pre-loaded context. Each verifier needs NO tools - just reasoning.

**Citation Verifier (spawn one per citation):**

```
Prompt:
  You are a skeptical reviewer. Your job is to find problems, not confirm correctness.
  
  CLAIM: "The build process starts by loading Twoliter.toml"
  CITED SOURCE: twoliter/twoliter/src/project/mod.rs
  SOURCE CONTENT:
  [paste relevant lines from the file]

  Does the source content ACTUALLY support this specific claim?
  
  Reply with one of:
  - SUPPORTED: [brief explanation of what in the source confirms this]
  - UNSUPPORTED: [what's missing or wrong]
  - PARTIAL: [what's confirmed vs what's not]
  
  Be skeptical. If the source is tangentially related but doesn't directly 
  state the claim, that's PARTIAL or UNSUPPORTED.
```

**Uncited Claim Detector (spawn once per document section):**

```
Prompt:
  You are a skeptical reviewer. Your job is to find problems.
  
  SECTION:
  [paste section text]

  Find factual claims that lack a <sup>[N]</sup> citation.
  Factual claims include: file paths, behavior descriptions, 
  configuration values, component names, process steps.
  
  Opinions, transitions, and summaries don't need citations.
  
  List each uncited claim you find. Do NOT say "looks good" or "none found" 
  unless you've checked every sentence. If you find nothing, explain what 
  you checked to confirm there are no uncited claims.
```

**Batch these:** Use spawn_batch to run all citation verifiers in parallel. The batch size MUST equal the citation count from Step 1.

#### Step 3: Confirm Coverage

After verifiers complete:
- Number of citations in document: N
- Number of verifiers spawned: N
- Any UNSUPPORTED or PARTIAL results: [list]

If counts don't match, you skipped citations. Go back and verify the missing ones.

Fix any UNSUPPORTED or PARTIAL before finalizing.

#### For Single-Agent Systems

Self-check is less reliable but still valuable. Use the adversarial framing:

1. For each citation, re-read the source and ask "does this ACTUALLY say what I claimed, or am I pattern-matching?"
2. Scan each section asking "what would a skeptic challenge here?"

Note: Agents checking their own work tend to confirm it. The filesystem-based workflow helps - you can reset context and review with fresh eyes.

## Validation Checklist

Before finalizing:

- [ ] Every factual claim has a citation
- [ ] No claims marked "could not determine" remain unexplained
- [ ] Overview has a visual before prose
- [ ] Dense lists use tables, not paragraphs
- [ ] Conflicts & Resolutions section completed (or "None")
- [ ] Sources section has all referenced citations
- [ ] All sub-question files exist in workspace
- [ ] Verification phase completed with explicit count confirmation
- [ ] Number of verifiers spawned equals number of citations

## Research Quality Indicator

End your document with:

- ✅ **Answered from documentation** - Fully answered from README files, design docs, or narrative documentation.
- ⚠️ **Answered from source code** - Had to read implementation files because documentation was insufficient.
- 🔍 **Partial documentation** - Required both docs and source code to answer fully.
- ❓ **Gaps remain** - Some sub-questions could not be answered; noted in document.

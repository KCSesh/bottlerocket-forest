---
name: review-code
description: Deep code review generating PR comments via principled question-driven analysis
---

# Code Review Skill

Review a PR by generating and answering ALL questions derived from review principles. Produces a final document with specific PR comments and references to reasoning artifacts.

## How This Works

Code review is interrogation: generate questions, **answer all of them**, surface what's unclear or concerning.

**What this skill produces:**
- Questions derived from review principles
- Answers to every question (or explicit "needs author")
- Implications drawn from answers
- Surfaced concerns organized by category

**What this skill does NOT produce:**
- Severity rankings (critical/major/minor)
- Blocking vs non-blocking classifications
- Approval recommendations

**Process:**
0. **Setup** - Check out PR branch locally
1. **Scout** - Maps PR territory, generates initial questions (may be shallow)
2. **Refine Questions** - Fills knowledge gaps, generates the REAL question backlog, classifies by type
3. **Answer Questions** - Routes each question to appropriate handler, produces Q&A document
4. **Holistic Pre** - Assesses approach using Q&A (parallel with step 5)
5. **Per-Commit** - Reviews each commit using Q&A (parallel)
5.5. **Filter Implications** - Binary surface decision + categorization
6. **Holistic Post** - Reassesses, checks requirements fulfillment
7. **Synthesize** - Combines into concerns organized by category

**Question types and handlers:**
- FACT → `fact-find` skill (cited answers)
- RESEARCH → `deep-research` skill (educational documents)
- DESIGN → design evaluation (propose alternatives, evaluate against principles)
- CORRECTNESS → correctness analysis (trace code, verify behavior)
- INTENT → marked as needs-author (cannot self-answer)

**Key insight:** Every question gets an explicit, documented answer (or explicit "needs author"). This creates verifiable artifacts.

## Input

- PR reference (URL, number, or branch)
- Optional: user-identified critical issues or documentation

## Setup

Before starting, the orchestrator checks out the PR:

```bash
# In the target repository directory
gh pr checkout <PR-number>
```

**Prerequisites:**
- Working directory must be the repository containing the PR
- No uncommitted changes (fail early if dirty)
- GitHub CLI authenticated

After checkout, all subagents read code from the local filesystem instead of using `gh` CLI for diffs.

## Workspace

Create `planning/<pr-slug>/` for all artifacts. **Do not commit these artifacts**—they are working documents, not deliverables.



```
planning/<pr-slug>/
├── 00-scout.md           # Initial territory map
├── 01-questions.md       # Refined, classified question backlog
├── 02-answers.md         # Consolidated Q&A document
├── answers/              # Individual answer artifacts
│   ├── fact-Q1.md
│   ├── design-Q2.md
│   └── ...
├── 03-commit-NN.md       # Per-commit reviews
├── 04-holistic-pre.md
├── filtered-implications.md  # Categorized implications to surface
├── 05-holistic-post.md
└── REVIEW.md             # Final output
```

## Subagent Protocol

**Files to pass:** PRINCIPLES.md, the agent instruction file, artifacts from prior phases

**Data to pass:** `workspace` path, phase-specific data

**Responses:**
```
OUTPUT: <filename>
STATUS: ok | problems
NOTES: <if problems>
```

Trust `STATUS: ok`. On `problems`, read notes and decide.

## Repository Access

The PR is checked out locally before review begins. Subagents receive:
- `repo_path`: Path to repository with PR checked out
- `base_ref`: The base branch (e.g., `main`)
- `head_ref`: The PR branch (checked out)

Subagents read files directly from `repo_path`. Use `git diff {base_ref}...HEAD` for diffs.

## Phases

### 0. Setup

Check out the PR locally before any analysis:

```bash
gh pr checkout <PR-number>
```

Capture `base_ref` from PR metadata for diff commands.

### 1. Scout

**Agent:** `agents/01-scout.md`
**Files:** PRINCIPLES.md, agent file
**Data:** workspace, repo_path, base_ref, pr_number, user context
**Output:** `00-scout.md`

### 2. Refine Questions

**Agent:** `agents/02-refine-questions.md`
**Files:** PRINCIPLES.md, agent file, 00-scout.md
**Data:** workspace, repo_path, base_ref, linked issues/docs
**Output:** `01-questions.md`

This phase uses fact-find to fill knowledge gaps and produces a classified question backlog.

### 3. Answer Questions

For each question in `01-questions.md`, spawn the appropriate answering agent:

| Type | Agent | Output |
|------|-------|--------|
| FACT | `agents/03-answer-fact.md` | `answers/fact-{id}.md` |
| RESEARCH | `agents/04-answer-research.md` | `answers/research-{id}.md` |
| DESIGN | `agents/05-answer-design.md` | `answers/design-{id}.md` |
| CORRECTNESS | `agents/06-answer-correctness.md` | `answers/correctness-{id}.md` |
| INTENT | (skip—mark as needs-author) | |

Run answering agents in parallel where possible.

After all complete, consolidate into `02-answers.md`:
```markdown
# Answered Questions

| ID | Type | Question | Answer Summary | Detail |
|----|------|----------|----------------|--------|
| Q1 | FACT | ... | ... | [answers/fact-Q1.md] |

## Needs Author
| ID | Question | Why We Can't Answer |
```

### 4. Holistic Pre-Assessment

**Agent:** `agents/07-holistic-pre.md`
**Files:** PRINCIPLES.md, agent file, 01-questions.md, 02-answers.md
**Data:** workspace, PR description, commit messages
**Output:** `04-holistic-pre.md`

Can run in parallel with Phase 5.

### 5. Per-Commit Review

**Agent:** `agents/08-commit-review.md` (one per commit, parallel)
**Files:** PRINCIPLES.md, agent file, 01-questions.md, 02-answers.md
**Data:** workspace, repo_path, base_ref, commit_sha, commit_message, commit_number, total_commits, all_commit_messages
**Output:** `03-commit-NN.md`

```python
for each commit in PR:
    spawn commit_reviewer(
        commit_diff=commit.diff,
        commit_number=N,
        ...
    ) → 03-commit-{N}.md
# One agent per commit - do not batch
```

### 5.5. Filter Implications

**Agent:** `agents/085-filter-implications.md`
**Files:** PRINCIPLES.md, agent file, all 03-commit-*.md
**Data:** workspace
**Output:** `filtered-implications.md`

Agent parses implications and author questions directly from commit review files.

Binary surface decision (surface or don't) plus categorization:
- **correctness** - Logic errors, bugs, incorrect behavior
- **assumption** - Unstated assumptions that may not hold
- **type-system** - Type safety concerns, missing constraints
- **intent** - Questions about author's intent that need clarification

```python
spawn implication_filter(workspace=workspace) → filtered-implications.md
# Agent extracts implications from 03-commit-*.md files
```

### 6. Holistic Post-Assessment

**Agent:** `agents/09-holistic-post.md`
**Files:** PRINCIPLES.md, agent file, 01-questions.md, 02-answers.md, 04-holistic-pre.md, all 03-commit-*.md, filtered-implications.md
**Data:** workspace
**Output:** `05-holistic-post.md`

### 7. Synthesize

**Agent:** `agents/10-synthesize.md`
**Files:** PRINCIPLES.md, agent file, all artifacts
**Data:** workspace
**Output:** `REVIEW.md`

Organizes surfaced concerns by category (correctness, assumption, type-system, intent), not by severity.

### 8. Deliver

Present `REVIEW.md` to user. Offer to post comments to GitHub.

## Token Preservation

You orchestrate; subagents analyze. Trust status responses.

**Do not read intermediate artifacts** except:
- `STATUS: problems` → read notes
- User asks a question
- Debugging a failure

**Always read:** Status responses, REVIEW.md final output.

## Skill Files

```
skills/review-code/
├── SKILL.md
├── PRINCIPLES.md
└── agents/
    ├── 01-scout.md
    ├── 02-refine-questions.md
    ├── 03-answer-fact.md
    ├── 04-answer-research.md
    ├── 05-answer-design.md
    ├── 06-answer-correctness.md
    ├── 07-holistic-pre.md
    ├── 08-commit-review.md
    ├── 085-filter-implications.md
    ├── 09-holistic-post.md
    └── 10-synthesize.md
```

# Implication Filter Subagent

This file contains instructions for a subagent collaborating on reviewing a PR.

You have a specific goal to contribute to the overall review, which is driven by the PRINCIPLES.md you will receive. Fulfill your specific purpose within the context of the broad problem. Constrain yourself not to solve problems outside your scope—other agents handle those. If there are organizational challenges to completing your task, report them in your status response.

## Your Purpose

Filter and categorize implications from commit reviews. Apply binary classification criteria and assign categories. No ranking, no severity assessment.

## You Will Receive

**Files:**
- `PRINCIPLES.md` - Review ethos
- This instruction file
- All `03-commit-NN.md` files

**Data:**
- `workspace`: path to write output

## Task

1. Collect all implications from commit review files, including "Questions for Author" sections
2. For each implication, apply binary classification:
   - SURFACE: Describes a consequence, constraint, or question not explicitly stated in the code or commit message
   - SKIP: Duplicates another implication, restates information already in the diff, or describes standard language/library behavior
3. Categorize each surfaced implication using the rules below
4. Preserve implication text exactly as written in source—do not rephrase, summarize, or editorialize
5. Parse citations to populate table columns:
   - Citation format in source: `{sha}:path/to/file.rs:line` (e.g., `3beb4d77:tools/pcrsys/src/aws/uefi.rs:37-56`)
   - Split on first `:` → SHA is before, file:line is after
   - SHA column: first 8 characters of the SHA
   - Citation column: the `path/to/file.rs:line` portion
   - Example: `3beb4d77c447419d504128723000059dd8e144b5:tools/pcrsys/src/aws/uefi.rs:37-56`
     → SHA: `3beb4d77`, Citation: `tools/pcrsys/src/aws/uefi.rs:37-56`
6. Write output to `{workspace}/filtered-implications.md`

## Categorization Rules

Apply first matching rule:
- **correctness**: Implication describes behavior that fails, crashes, returns wrong values, loses data, or violates a stated invariant
- **assumption**: Implication uses words like "assumes", "depends on", "requires", "expects", or describes implicit constraints
- **type-system**: Implication mentions runtime checks that could be compile-time, or stringly-typed values
- **intent**: Implication is phrased as a question or contains "unclear", "ambiguous", or asks "why"

## Judgment Boundaries

ALLOWED:
- Binary classification (SURFACE or SKIP)
- Categorization into correctness/assumption/type-system/intent

NOT ALLOWED:
- Severity ranking (minor/major/critical)
- Blocking vs non-blocking
- "Probably fine", "acceptable", "appropriate", "reasonable", "sufficient"
- Tradeoff analysis
- Assessments of importance or significance

## Output Format

```markdown
# Filtered Implications

## Correctness
| Commit | SHA | Implication | Citation |
|--------|-----|-------------|----------|
| N | <sha> | <implication—verbatim from source> | file:line |

## Assumptions
| Commit | SHA | Implication | Citation |
|--------|-----|-------------|----------|

## Type System Opportunities
| Commit | SHA | Implication | Citation |
|--------|-----|-------------|----------|

## Intent (Needs Author)
| Commit | SHA | Question | Citation |
|--------|-----|----------|----------|

## Skipped
| Implication | Skip Reason |
|-------------|-------------|
| <implication> | duplicate of commit N / restates diff line Y / standard library behavior: <cite doc> |

Skip reasons must cite specific duplicates, diff lines, or language/library documentation—not assessments of importance.
```

## Response to Caller

```
OUTPUT: filtered-implications.md
STATUS: ok | problems
NOTES: <if problems>
```

Use error reporting mechanism if context insufficient or task ambiguous.

**Do not commit artifacts.** Files in `planning/` are working documents.

# Code Review Principles

This document defines the ethos guiding effective code review. Internalize these principles—they inform every question you ask and every judgment you make.

## Core Ethos

**Review is interrogation, not inspection.** Generate questions, attempt to answer them from context, surface what remains unclear or concerning.

**Priority hierarchy:** Correct → Readable → Maintainable → Performant. Optimize in that order.

**No praise.** Factual assessment only. Positive commentary wastes tokens and attention.

## Skill Output Model

This skill produces: **questions → answers → implications → surfaced concerns**

This skill does NOT produce:
- Severity rankings (minor/major/critical)
- Blocking vs non-blocking classifications
- Approval recommendations
- Tradeoff assessments ("but it's probably fine")

**The human reviewer decides what to act on.** The skill surfaces factual implications; humans apply judgment about priorities and acceptable risk.

## Judgment Boundaries

**ALLOWED assessments (binary, factual):**
- "This implication describes undesirable behavior"
- "This violates the stated requirement"
- "This breaks the documented invariant"
- "This is inconsistent with X"

**NOT ALLOWED assessments (subjective, graduated):**
- "This is minor/blocking/acceptable"
- "This is probably fine"
- "This tradeoff is worth it"
- "I recommend approving/rejecting"

State facts and implications. Let humans decide significance.

## Principles

### 1. Self-Interrogation Before Escalation
Ask questions. Answer them yourself from available context. Only surface to the author what you genuinely cannot determine or what reveals a problem.

### 2. Implicit Requirements Extraction
Code introduces requirements even when unstated. At function level: "returns sorted" implies callers depend on order. At system level: new functionality has implicit requirements that engineers with domain knowledge leave unspoken. Surface these. Ask: intentional? documented? verified?

### 3. Verification Over Trust
Prefer guarantees in order: compile-time (types, generics, typestate, newtypes) → runtime checks → tests → documentation. Tests should verify what types cannot. Worthless tests: language features, library behavior. Valuable tests: domain logic, edge cases, error paths.

### 4. Type System as Primary Verification

The type system is the strongest verification tool. Maximize its use:

**Compile-time guarantees eliminate entire bug classes:**
- Newtypes prevent mixing semantically distinct values (UserId vs OrderId)
- Typestate encodes valid state transitions (Builder → Configured → Running)
- Generics with trait bounds constrain behavior at compile time
- NonZero, NonEmpty types eliminate null/empty checks

**When reviewing, ask:**
- Could this runtime check be a compile-time guarantee?
- Does this stringly-typed value deserve a newtype?
- Could invalid states be made unrepresentable?
- Are state transitions enforced by types or just documented?

**Hierarchy of trust:** What the compiler enforces > what tests verify > what documentation promises > what comments claim.

### 5. Testability as Design Feedback
Hard to test = design smell. IO behind traits. Pure functions. Dependency injection. Code written to be verified.

### 6. Change Coherence
Per-commit: single idea, builds on previous, independently reviewable. Per-PR: every change serves the stated goal. Unrelated fixes obscure intent.

### 7. Customer Surface Awareness
Customer surfaces aren't always public methods. Consider: CLI, config formats, API behavior, error messages, upgrade paths. What promises are we making or changing?

### 8. Error Handling Completeness
"Shouldn't happen" → panic/assert. "Might happen" → Result/Option. Are errors actionable? Can callers recover?

### 9. Naming as Documentation
Names reveal intent. If you must read implementation to understand purpose, the name is wrong.

### 10. Readability as Answerability
How hard is it to uncover purpose and hidden assumptions?

**Progressive disclosure:** Good abstractions orchestrate smaller abstractions. 60 lines of english-like orchestration beats 500 LOC of inline algorithm. Docstrings describe abstraction interactions; invariants belong in code (newtypes, not comments).

**Cognitive load thresholds:** Nesting >4 levels, functions >60 lines, files >550 lines → almost certainly not readable.

**"Reads like english"** is a valuable quality.



## The "Acceptable" Trap

Never write "this is acceptable" or "this is fine" or "this is appropriate."

Instead, state the factual trade-off:
- ❌ "Using snafu::Whatever is acceptable for build tooling"
- ✅ "Using snafu::Whatever means errors cannot be programmatically distinguished; all error handling must be string-based"

- ❌ "This edge case handling is sufficient"
- ✅ "Empty input returns None; callers must handle this case"

- ❌ "The approach is reasonable for this use case"
- ✅ "The approach handles X by doing Y; it does not handle Z"

The human reviewer decides what is acceptable. You state what IS.

## Consequence vs Judgment

Every observation should have a consequence. Consequences are factual:

**Factual consequences (allowed):**
- "Callers cannot match on error variants"
- "This value can be zero at runtime"
- "The function panics if the slice is empty"
- "State transitions are not enforced by the type system"

**Judgments (not allowed):**
- "This is a minor issue"
- "This trade-off is worth it"
- "This is probably fine"
- "Acceptable for internal tooling"

## On Abstractions

Code is cheap. Abstractions are cheap to change—until customers depend on them.

- Not load-bearing: refactor aggressively for correctness and readability
- Load-bearing: requires migration paths, deprecation, careful consideration

When you identify refactoring opportunities, **surface them as proposals** for user decision. Don't assume they're worthwhile.

## Applying Principles

These principles generate questions. Some questions are always relevant (requirements, invariants, coherence). Others depend on context—ask them when the code touches their domain.

Document your questions and answers explicitly. They form the audit trail that lets others follow your reasoning.

# Feature Proposal Tutorial

**Keywords:** spec-driven, feature-proposal, concept, requirements, design, test-plan, implementation, EARS, critical-constraints

## The Philosophy: Spec-Driven Development

Building features with AI agents works best when you think of development as a conversation that progressively narrows from "what problem are we solving?" down to "what exact code changes do we make?"
Each phase of this process produces a document that serves as a contract—both for the AI implementing the feature and for you reviewing its work.

The key insight is that **gaps in specifications lead to gaps in implementation**.
When an AI agent encounters ambiguity, it makes assumptions.
Sometimes those assumptions are right; often they're subtly wrong in ways that only surface later.
By front-loading clarity through structured documents, you give the agent guardrails that keep it on track.

This isn't about bureaucracy or ceremony: it's about having the hard conversations early—about edge cases, error handling, performance constraints—when changing direction is cheap.

## The Development Lifecycle

The process flows through five phases, each building on the last:

```
┌─────────────┐     ┌──────────────┐     ┌────────┐     ┌───────────┐     ┌────────────────┐
│   Concept   │ ──▶ │ Requirements │ ──▶ │ Design │ ──▶ │ Test Plan │ ──▶ │ Implementation │
│             │     │              │     │        │     │           │     │      Plan      │
│  "why" and  │     │   "what"     │     │ "how"  │     │ "verify"  │     │    "build"     │
│   "what"    │     │  precisely   │     │        │     │           │     │                │
└─────────────┘     └──────────────┘     └────────┘     └───────────┘     └────────────────┘
```

You don't always need every phase.
A small bug fix needs none of this; a new CLI command might only need a concept and some requirements; a major architectural change benefits from the full treatment.
Use judgment about where to enter the process and how deep to go.

## Phase 1: Concept

The concept document answers two questions: what problem exists, and what would life look like if we solved it?

This phase is deliberately narrative.
You're telling a story about a user's pain and how the feature alleviates it.
The goal is alignment—making sure everyone (including the AI) understands *why* this feature matters before diving into specifics.

A concept document walks through the problem, the proposed solution, how a user would experience the feature, and what benefits it provides.
Technical details are kept minimal; this isn't the place for architecture diagrams or API signatures.

**When reviewing a concept**, ask yourself: does this clearly explain why someone would want this feature?
Could a new team member read this and understand the motivation?
Are there unstated assumptions about how users work that might not hold?

Watch for gaps in the problem statement.
If the concept says "users find X frustrating" but doesn't explain *why* it's frustrating or *when* they encounter it, the AI will fill in those blanks with guesses.
Make the pain concrete.

## Phase 2: Requirements

Requirements translate the narrative concept into precise, testable statements using EARS notation (Easy Approach to Requirements Syntax).
Each requirement follows a pattern:

> **WHILE** [some condition holds]  
> **WHEN** [something happens]  
> **THEN** the system **SHALL** [do something specific]

This structure forces clarity.
You can't write "the system should be fast"—you have to say "WHILE processing a batch, WHEN the batch exceeds 1000 items, THEN the system SHALL complete within 5 seconds."

Requirements come in three flavors: functional (what the system does), non-functional (how well it does it), and error handling (what happens when things go wrong).
The error handling requirements are often where gaps hide—it's easy to describe the happy path and forget the failure modes.

**When reviewing requirements**, trace each one back to the concept.
Does every requirement serve the stated problem?
Are there aspects of the concept that don't have corresponding requirements?

More importantly, look for missing requirements.
Think about edge cases: what happens with empty input?
What about malformed input?
What if the user cancels mid-operation?
What if the disk is full?
The AI will only handle cases you specify; unspecified cases get unspecified behavior.

## Phase 3: Design

The design document bridges requirements and code.
It describes the architecture, the key types and their relationships, the module structure, and—critically—the constraints that must not be violated.

The most important section is **Critical Constraints**.
These are the "you must do it this way" rules that prevent subtle bugs.
Each constraint includes an anti-pattern: the wrong approach that a naive implementation might take.

For example, a constraint might say: "Memory usage SHALL remain constant regardless of input size. Anti-pattern: loading the entire file into memory before processing."
This tells the implementer (human or AI) exactly what pitfall to avoid.

**When reviewing a design**, focus on the critical constraints.
Are they specific enough to catch mistakes?
Are there performance or correctness requirements from the previous phase that don't have corresponding constraints?

Also consider what's *not* in the design.
If the requirements mention error handling, does the design explain how errors propagate?
If there's a non-functional requirement about latency, does the design address how that latency is achieved?

## Phase 4: Test Plan

The test plan maps requirements and critical constraints to specific tests.
Every requirement gets a test; every constraint gets verification.

This phase serves two purposes.
First, it ensures coverage—you can see at a glance whether any requirement lacks verification.
Second, it forces you to think about *how* you'll know the feature works.
Some requirements are easy to test; others reveal their ambiguity only when you try to write a test for them.

Tests fall into categories: unit tests for internal logic, integration tests for real-world behavior (actually running CLI commands, touching the filesystem), and occasionally "not testable" for things that require human judgment.

**When reviewing a test plan**, check the coverage table.
Is every requirement ID accounted for?
Is every critical constraint verified?

Look for requirements marked "not testable" and ask whether that's truly the case or whether the requirement itself is too vague.
A requirement that can't be tested often can't be implemented consistently either.

## Phase 5: Implementation Plan

The implementation plan breaks the feature into commits that each tell a complete story.
Each commit is buildable (the project compiles), tested (new code has tests), and reviewable (small enough to understand in one sitting).

The key principle is **narrative atomicity**: commits should be organized around capabilities and impact, not around code artifacts.
A commit that "adds the config module with loading, validation, and tests" is easier to review than three commits that separately add types, then implementation, then tests.
The reviewer can see what the types are *for* because the implementation is right there.

This also enables test-driven development *within* each commit—you write the test, then the implementation, then refine.
Splitting tests into a separate commit breaks this workflow.

The plan assigns critical constraints to specific commits, making them explicit review checkpoints.
When reviewing a commit, you know which constraints it must satisfy.

**When reviewing an implementation plan**, ask: does each commit tell a coherent story?
Could a reviewer understand the commit's purpose without reading the commits before and after it?
Are types, implementation, and tests kept together?

Watch for the anti-pattern of splitting by artifact: "add types" → "add impl" → "add tests."
This fragments the story and makes review harder.
If a commit seems too large, split by capability (e.g., "config loading" vs "config validation"), not by artifact type.

## Putting It Together

The phases build on each other, with each document referencing the ones before it.
Requirements trace to the concept; design traces to requirements; tests trace to both requirements and constraints; implementation traces to everything.

This traceability is your safety net.
When something goes wrong—and something always goes wrong—you can follow the chain backward.
Did the implementation miss a constraint?
Did the design miss a requirement?
Did the requirements miss part of the problem?

The goal isn't perfect documents.
It's documents good enough that an AI agent can implement the feature correctly, and clear enough that you can review its work efficiently.
Iterate on the documents when you find gaps; that's cheaper than iterating on code.

## Getting Started

Note: It is recommended to use a new, "clean" AI agent context window between each document in this series (with the exception of idea-honing and the concept document.)

To begin a new feature, use the `propose-feature-concept` skill.
It will guide you through creating the concept document and offer to run an idea-honing session if your thoughts aren't fully formed yet.

From there, each subsequent skill picks up where the last left off.
The skills know about the document structure and will prompt you for the information needed at each phase.

Remember: the documents are for the AI as much as for you.

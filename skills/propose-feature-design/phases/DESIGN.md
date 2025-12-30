# Design Phase

You are executing the design phase for creating a feature design document.

## Your Goal

Fill in the design template with architecture, critical constraints, domain model, and implementation guidance.

## Inputs

From context_files:
- `{{feature_dir}}/concept.md`: Feature concept
- `{{feature_dir}}/requirements.md`: Feature requirements
- `{{feature_dir}}/design.md`: Template to fill in
- `planning/{{feature_number}}-{{feature_name}}/idea-honing.md` (if exists): Design insights

From context_data:
- `feature_dir`: Path to feature directory
- `idea_honing_exists`: Boolean indicating if idea-honing.md was found

## Procedure

### 1. Fill in Overview

Write a high-level description of the architecture and design approach. Reference the concept and requirements.

### 2. Identify Critical Constraints

For each key operation, ask:
- What would be a WRONG way to implement this?
- What performance characteristics are required?
- What invariants must hold?

Fill in the Critical Constraints table with:
- **ID**: CC-1, CC-2, etc.
- **Constraint**: What MUST be done a specific way
- **Rationale**: Why (performance, correctness, security)
- **Anti-pattern**: The wrong approach to reject in review

This is the most important section for preventing implementation mistakes.
Be specific—vague constraints don't help.

### 3. Define Architecture

Describe the overall structure:
- Component relationships
- Data flow
- Layer responsibilities
- Integration points

Use simple diagrams with ASCII art if helpful.

### 4. Define Domain Model

Specify the core types and operations:

**Types**
- Purpose and role
- Key properties
- Validation rules
- Relationships to other types

**Operations**
- Function signatures (conceptual)
- What they do
- Key behaviors
- Invariants (what must always be true)

### 5. Define Module Structure

Show how code should be organized:
- Directory structure
- Module responsibilities
- File organization
- Separation of concerns

### 6. Document Design Patterns

Describe relevant patterns and why they apply:
- Which patterns to use
- How they fit the problem
- Implementation guidance

### 7. Document Design Decisions

For significant choices between alternatives, add entries to the Design Decisions section:
- What was decided
- What alternatives were considered
- Why this option was chosen
- What it implies for implementation

This creates a record that helps implementors understand the reasoning.

### 8. Add Implementation Guidance

Key considerations for implementors:
- Reference Critical Constraints by ID
- Performance considerations
- Testing approach
- Edge cases to handle

### 9. Keep Code Minimal

Remember:
- Use minimal illustrative code
- Focus on architecture and decisions
- Guide implementors, don't implement
- Reference requirements by ID when relevant

## Output

Call `respond_to_leader()` with:
- status: "success"
- response: Brief summary of what was documented

## Example Response

```
Design document completed for {{feature_name}}.

Key sections filled:
- Overview: High-level architecture approach
- Critical Constraints: 5 constraints identified (CC-1 through CC-5)
- Architecture: Component relationships and data flow
- Domain Model: Core types and operations defined
- Module Structure: Directory layout and responsibilities
- Design Patterns: Builder pattern for configuration
- Design Decisions: 3 key decisions documented
- Implementation Guidance: References to critical constraints

The design provides clear guidance for implementation without prescribing specific code.
```

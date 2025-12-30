# Bottlerocket Forest Skills

Claude Code skills for common Bottlerocket development workflows.

## ⚠️ IMPORTANT: Always Use Skills When Available

**Skills are the preferred way to accomplish tasks in the forest.** They provide:
- Tested, reliable workflows
- Consistent results across sessions
- Complete procedures with validation
- Known solutions to common issues

**Before starting any task, check if a skill exists for it.** Using skills ensures quality and saves time.

## Skill Usage Protocol

When you identify that a skill should be used:

1. **Announce it clearly:**
   ```
   USING SKILL "skill-name"
   ```

2. **Load the skill file:**
   ```
   Read skills/skill-name/SKILL.md
   ```

3. **Follow the procedure exactly as documented** in the SKILL.md file

**💡 TIP: If you have todolist functionality, use it to track skill steps.** Multi-step skills benefit from explicit progress tracking.

## Skill Classification

Skills fall into four categories that determine their structure and usage:

### 1. Workflow/Process Skills
Multi-step procedures that must execute reliably in sequence. **SHOULD use script-driven-skill pattern** for reliability and resumability.

Examples: build-kit-locally, build-variant-from-local-kits, update-twoliter, add-custom-settings, add-settings-to-variant, create-settings-model, test-settings-locally, deep-research, fact-find

### 2. Documentation Skills
Provide guidance, style guides, or educational content. No execution required—just explain concepts.

Examples: test-local-twoliter

### 3. Tool-Providing Skills
Provide scripts/tools for interacting with systems. Agent chooses what to use based on context.

Examples: local-registry

### 4. Conversational/Interactive Skills
Require back-and-forth with user. **Root orchestrator handles directly**—do NOT delegate to subagents (users can't interact with subagents).

Examples: idea-honing

**Note:** Skills can span multiple categories. The propose-feature-* skills combine workflow and documentation aspects.

### When to Use Script-Driven Pattern

| Skill Type | Use script-driven-skill? | Why |
|------------|-------------------------|-----|
| Workflow/Process | ✅ Yes | Steps must execute reliably, skipping is failure |
| Documentation | ❌ No | Just explains concepts, no execution |
| Tool-Providing | ❌ No | Provides scripts/info, agent chooses what to use |
| Conversational | ❌ No | Root orchestrator handles, needs user interaction |

## Available Skills

- **fact-find** (Workflow) - Quick lookup of specific facts with citations. Use for concrete questions with definitive answers (e.g., "What partition scheme does Bottlerocket use?")
- **deep-research** (Workflow) - Create educational documents that build understanding progressively. Use for in-depth explanations of systems or features (e.g., "Explain how Bottlerocket's update system works")
- **script-driven-skill** (Meta) - Meta-skill for building reliable multi-step skills using a state-machine pattern with dumb orchestrator and smart phases
- **local-registry** (Tool-Providing) - Start and manage a local OCI registry for development
- **build-kit-locally** (Workflow) - Build a kit and publish it to a locally hosted registry for development testing
- **build-variant-from-local-kits** (Workflow) - Build a variant using locally published kits for development validation
- **test-local-twoliter** (Documentation) - Build and test local changes to twoliter before releasing
- **update-twoliter** (Workflow) - Update all repositories to a new Twoliter version
- **idea-honing** (Conversational) - Clarify feature ideas through iterative Q&A, recording insights to guide concept development
- **propose-feature-concept** (Workflow + Documentation) - Create a new feature concept document to pitch the idea and explain the problem/solution
- **propose-feature-requirements** (Workflow + Documentation) - Create or update feature requirements specification using EARS notation with examples and appendices
- **propose-feature-design** (Workflow + Documentation) - Create or update feature technical design document with architecture and implementation guidance
- **propose-feature-test-plan** (Workflow + Documentation) - Create a test plan mapping requirements and constraints to unit/integration tests
- **propose-implementation-plan** (Workflow + Documentation) - Create an implementation plan with atomic commits that build toward a complete feature
- **create-settings-model** (Workflow) - Define a new Bottlerocket settings model with SettingsModel trait implementation
- **add-settings-to-variant** (Workflow) - Wire an existing settings model into a Bottlerocket variant via settings-plugins
- **test-settings-locally** (Workflow) - Build and test settings SDK changes using local registry and kit builds
- **add-custom-settings** (Workflow) - Full workflow for adding custom settings: create model, wire to variant, test locally

## Skill Format

Each skill is a directory containing a `SKILL.md` file with:

1. **YAML frontmatter** - Metadata (name, description)
2. **Instructions** - Step-by-step procedures
3. **Optional helpers** - Scripts, templates, or resources

### SKILL.md Structure

```markdown
---
name: skill-name
description: Brief description of what the skill does
---

# Detailed instructions here
```

The frontmatter requires:
- `name`: lowercase letters, numbers, hyphens only (max 64 chars)
- `description`: concise summary (max 256 chars)

### Content Sections

- **Purpose** - What the skill does and why
- **When to Use** - Applicable scenarios
- **Prerequisites** - Required setup
- **Procedure** - Step-by-step instructions
- **Validation** - Success verification
- **Common Issues** - Known problems and solutions

## Creating New Skills

1. Create a directory with a descriptive name
2. Add `SKILL.md` with YAML frontmatter
3. Include concrete commands and examples
4. Add validation steps
5. Document common failure modes

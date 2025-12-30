# Setup Phase

You are executing the setup phase of creating an implementation plan.

## Your Goal

Verify prerequisites exist and prepare the workspace for planning.

## Inputs

- Workspace: `{{workspace}}` (from context_data)
- Feature number: Extract from workspace path (e.g., `planning/0042-feature-name` → `0042`)

## Procedure

### 1. Extract Feature Information

```bash
# Extract feature number from workspace path
FEATURE_NUM=$(basename {{workspace}} | grep -oE '^[0-9]+')
FEATURE_NAME=$(basename {{workspace}} | sed "s/^${FEATURE_NUM}-//")
```

### 2. Verify Design Document Exists

```bash
DESIGN_PATH="docs/features/${FEATURE_NUM}-${FEATURE_NAME}/design.md"
if [ ! -f "$DESIGN_PATH" ]; then
    echo "ERROR: Design document not found at $DESIGN_PATH"
    echo "Run 'propose-feature-design' skill first"
    exit 1
fi
```

### 3. Verify Test Plan Exists

```bash
TEST_PLAN_PATH="docs/features/${FEATURE_NUM}-${FEATURE_NAME}/test-plan.md"
if [ ! -f "$TEST_PLAN_PATH" ]; then
    echo "ERROR: Test plan not found at $TEST_PLAN_PATH"
    echo "Run 'propose-feature-test-plan' skill first"
    exit 1
fi
```

### 4. Create Planning Directory

```bash
mkdir -p {{workspace}}
```

### 5. Copy Template

```bash
cp docs/features/0000-templates/implementation-plan.md {{workspace}}/
```

### 6. Record Paths

Write to `{{workspace}}/00-setup.md`:

```markdown
# Setup Complete

## Feature Information
- Number: ${FEATURE_NUM}
- Name: ${FEATURE_NAME}

## Verified Files
- Design: ${DESIGN_PATH}
- Test Plan: ${TEST_PLAN_PATH}
- Template: {{workspace}}/implementation-plan.md

## Next Steps
Proceed to analyze phase to study design and extract constraints.
```

## Completion

Call `respond_to_leader("success", "<setup output>")` with the content you wrote to 00-setup.md.

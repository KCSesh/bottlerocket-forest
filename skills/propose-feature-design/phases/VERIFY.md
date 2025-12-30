# Verify Prerequisites Phase

You are executing the verify phase for creating a feature design document.

## Your Goal

Confirm that prerequisite documents (concept.md and requirements.md) exist before proceeding.

## Inputs

From context_data:
- `feature_dir`: Path to feature directory (e.g., docs/features/NNNN-feature-name)

## Procedure

1. Check that concept.md exists:
   ```bash
   ls {{feature_dir}}/concept.md
   ```

2. Check that requirements.md exists:
   ```bash
   ls {{feature_dir}}/requirements.md
   ```

3. If either is missing, report which prerequisites are missing.

4. If both exist, confirm success.

## Output

Call `respond_to_leader()` with:
- status: "success" if both files exist
- status: "failed" if either is missing
- response: Brief confirmation or list of missing files

## Example Response

Success:
```
Prerequisites verified:
- concept.md exists
- requirements.md exists
```

Failure:
```
Missing prerequisites:
- requirements.md not found

Complete the requirements document before creating the design.
```

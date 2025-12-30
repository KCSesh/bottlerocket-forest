# Update Lock File Phase

You are executing the lock file update phase.

## Your Goal

Run twoliter update to regenerate the lock file with local kit references.

## Inputs

- Workspace: Read from context_data["workspace"]

## Procedure

1. Navigate to bottlerocket directory and run update:
   ```bash
   cd $FOREST_ROOT/bottlerocket
   ./tools/twoliter/twoliter update
   ```

2. Verify lock file was updated:
   ```bash
   ls -l Twoliter.lock
   ```

3. Write completion marker to `{{workspace}}/02-lock-updated.json`:
   ```json
   {
     "phase": "update_lock",
     "status": "complete",
     "lock_file": "Twoliter.lock"
   }
   ```

## Common Issues

**Lock file update fails:**
- Check that kits exist in local registry: `(cd $FOREST_ROOT && brdev registry list)`
- Verify Twoliter.toml syntax is correct

## Completion

Call `respond_to_leader("success", "Lock file updated")` when done.

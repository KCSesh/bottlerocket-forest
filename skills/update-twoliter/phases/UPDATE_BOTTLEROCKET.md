# Update Bottlerocket Phase

You are executing the update-bottlerocket phase of the update-twoliter skill.

## Your Goal

Update bottlerocket/Makefile.toml with the new Twoliter version and checksums, then commit.

## Inputs

- Workspace: Read from context_data["workspace"]
- Checksums: Read from `<workspace>/checksums.json`
- Worktree root: Read from context_data["worktree_root"]

## Procedure

1. Read checksums:
   ```python
   import json
   data = json.loads(fs_read("Line", f"{workspace}/checksums.json", 1, 10))
   version = data["version"]
   x86 = data["x86_64"]
   aarch64 = data["aarch64"]
   ```

2. Update `bottlerocket/Makefile.toml`:
   - Find lines with TWOLITER_VERSION, TWOLITER_SHA256_AARCH64, TWOLITER_SHA256_X86_64
   - Replace with new values (note: WITH 'v' prefix for bottlerocket)
   - Use str_replace for each line

3. Commit:
   ```python
   bash(f'git add Makefile.toml && git commit -m "chore: bump to twoliter {version}"', cwd=f"{worktree_root}/bottlerocket", on_error="raise")
   ```

4. Write confirmation to `<workspace>/bottlerocket-updated.txt`.

## Important Notes

- Bottlerocket uses `"vX.Y.Z"` format (WITH 'v' prefix)
- Single commit for bottlerocket
- Commit message: `chore: bump to twoliter X.Y.Z`

## Completion

Call `respond_to_leader("success", "Bottlerocket updated")` when done.

# Update Kits Phase

You are executing the update-kits phase of the update-twoliter skill.

## Your Goal

Update all kit Makefiles with the new Twoliter version and checksums, then commit.

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

2. Find all kits:
   ```python
   kits = bash("find kits -maxdepth 1 -type d -name 'bottlerocket-*-kit'", on_error="raise", cwd=worktree_root).strip().split("\n")
   ```

3. For each kit, update Makefile:
   - Find lines with TWOLITER_VERSION, TWOLITER_SHA256_AARCH64, TWOLITER_SHA256_X86_64
   - Replace with new values (note: NO 'v' prefix in kit Makefiles)
   - Use str_replace for each line

4. Commit each kit:
   ```python
   bash(f'git add Makefile && git commit -m "chore: bump to twoliter {version}"', cwd=f"{worktree_root}/{kit}", on_error="raise")
   ```

5. Write summary to `<workspace>/kits-updated.txt` with list of updated kits.

## Important Notes

- Kits use `"X.Y.Z"` format (NO 'v' prefix)
- Each kit gets its own commit
- Commit message: `chore: bump to twoliter X.Y.Z`

## Completion

Call `respond_to_leader("success", "Kits updated: <list>")` when done.

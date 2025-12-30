# Validate Phase

You are executing the validate phase of the update-twoliter skill.

## Your Goal

Verify that commits were created correctly in all repositories.

## Inputs

- Workspace: Read from context_data["workspace"]
- Worktree root: Read from context_data["worktree_root"]
- Kits list: Read from `<workspace>/kits-updated.txt`

## Procedure

1. Read list of updated kits from workspace.

2. For each kit, verify commit:
   ```python
   result = bash(f"git -C {worktree_root}/{kit} show HEAD --stat", on_error="raise")
   # Check for: commit message, 1 file changed (Makefile), 3 insertions, 3 deletions
   ```

3. Verify bottlerocket commit:
   ```python
   result = bash(f"git -C {worktree_root}/bottlerocket show HEAD --stat", on_error="raise")
   # Check for: commit message, 1 file changed (Makefile.toml), 3 insertions, 3 deletions
   ```

4. Write validation report to `<workspace>/FINAL.md`:
   ```markdown
   # Update Twoliter: Validation Report
   
   ## Version
   X.Y.Z
   
   ## Updated Repositories
   - bottlerocket-core-kit: ✓
   - bottlerocket-kernel-kit: ✓
   - bottlerocket: ✓
   
   ## Commits Created
   All repositories have commits with message: `chore: bump to twoliter X.Y.Z`
   
   ## Next Steps
   - Push commits to remote branches
   - Create pull requests for CI testing
   ```

## Completion

Call `respond_to_leader("success", "<validation summary>")` when done.

# Fetch Phase: Get SHA256 Checksums

You are executing the fetch phase of the update-twoliter skill.

## Your Goal

Fetch SHA256 checksums for the specified Twoliter version from GitHub releases.

## Inputs

- Workspace: Read from context_data["workspace"]
- Version: Read from `<workspace>/version.txt`

## Procedure

1. Read the target version:
   ```python
   version = fs_read("Line", f"{workspace}/version.txt", 1, 1).strip()
   ```

2. Fetch SHA256 for x86_64:
   ```python
   x86 = bash(f'curl -sSL "https://github.com/bottlerocket-os/twoliter/releases/download/v{version}/twoliter-x86_64-unknown-linux-musl.tar.xz.sha256"', on_error="raise").strip()
   ```

3. Fetch SHA256 for aarch64:
   ```python
   aarch64 = bash(f'curl -sSL "https://github.com/bottlerocket-os/twoliter/releases/download/v{version}/twoliter-aarch64-unknown-linux-musl.tar.xz.sha256"', on_error="raise").strip()
   ```

4. Write checksums to `<workspace>/checksums.json`:
   ```json
   {
     "version": "X.Y.Z",
     "x86_64": "sha256...",
     "aarch64": "sha256..."
   }
   ```

## Error Handling

If curl fails, the release may not exist. Report the error clearly.

## Completion

Call `respond_to_leader("success", "Checksums fetched")` when done.

---
name: prepare-core-kit-release
description: Prepare bottlerocket-core-kit for release by bumping version and updating changelog
---

# Prepare Core Kit Release

Prepare the bottlerocket-core-kit repository for a new release by bumping the version in Twoliter.toml and updating the CHANGELOG.md.

## Purpose

Automate the release preparation workflow for bottlerocket-core-kit:
- Bump the release version in Twoliter.toml
- Generate changelog entries from git history
- Create properly formatted commits for the release

## When to Use

- Preparing a new release of bottlerocket-core-kit
- Need to update changelog with recent changes
- Creating release commits with proper formatting

## Prerequisites

- Working in a grove with bottlerocket-core-kit checked out
- Git history available with merge commits and PR references
- Knowledge of the target release version

## Procedure

### Release Workflow

**IMPORTANT: The release process requires TWO commits in strict order:**

#### Commit 1: Version Bump (MUST come first)
Update `Twoliter.toml` to set the new release version:
```toml
# In Twoliter.toml, update:
release-version = "X.Y.Z"
```

Commit command:
```bash
git add Twoliter.toml && git commit -s -S -m "twoliter: bump kit version to vX.Y.Z"
```

#### Commit 2: Changelog Update (MUST come second)
Update `CHANGELOG.md` with the new version's changes.

Commit command:
```bash
git add CHANGELOG.md && git commit -s -S -m "changelog: update for vX.Y.Z"
```

### 1. Identify New Commits
- Find commits since the last changelog entry using: `git log --oneline <last_version_tag>..HEAD`
- If no tags exist, use: `git log --oneline --since="<last_release_date>"`

### 2. Find PR References
- Analyze merge commits to find PR numbers: `git log --merges --oneline <last_version_tag>..HEAD`
- Look for patterns like "Merge pull request #XXX" in commit messages
- Cross-reference commit hashes with PR merge commits

### 3. CRITICAL: Verify PR Contents
**ALWAYS verify what packages/changes are in each PR by checking the actual files modified:**
```bash
git show <merge_commit_hash> --stat
```

**DO NOT rely on:**
- Commit message summaries alone
- PR branch names
- Assumptions about what a PR contains

**WATCH FOR MIXED PRs:** A single PR may contain BOTH first-party changes AND third-party package updates. You must create separate changelog entries for each type of change from the same PR.

**Example of a mixed PR (PR #760):**
```bash
git show 70609d6a --stat
# Shows changes to:
#   sources/host-ctr/...          <- First-party change (host-ctr resolver)
#   packages/amazon-vpc-cni-plugins/...  <- Third-party update
#   packages/ecs-agent/...        <- Third-party update
```
This PR requires TWO changelog entries:
1. OS Changes: "Replace `amazon-ecr-containerd-resolver` with Docker resolver in `host-ctr` ([#760])"
2. Third Party Package Updates: "Update `amazon-vpc-cni-plugins`, `ecs-agent` ([#760])"

**Example of correct verification:**
```bash
# For merge commit 9656d841 (PR #795):
git show 9656d841 --stat
# Shows: libcap, liburcu, libxcrypt, readline changes
# NOT what the branch name or summary might suggest
```

### 4. Changelog Style Guidelines

#### Version Header Format
```
# v<VERSION> (<YYYY-MM-DD>)
```

#### Section Hierarchy
1. **OS Changes** - Core system and first-party package updates
   - First-party package updates (with detailed sub-bullets)
   - **Third Party Package Updates** subsection for external packages
2. **Build Changes** - Build system, tooling, and infrastructure changes
3. **Orchestrator Changes** - Container orchestration specific changes
   - **Kubernetes** subsection when applicable

#### Entry Format
- Use bullet points with PR references: `- Description ([#XXX])`
- For first-party packages, include detailed sub-bullets when appropriate
- **For third-party package updates: DO NOT include version numbers**
  - CORRECT: `- Update \`docker-cli-29\`, \`docker-engine-29\` ([#785])`
  - WRONG: `- Update \`docker-cli-29\` to v29.0.4, \`docker-engine-29\` to v29.0.4 ([#785])`
- For patches/fixes, use "Patch" instead of "Update":
  - CORRECT: `- Patch \`containerd-2.1\` to update GRPC ([#801])`

#### Categorization Rules
- **First-party packages**: Bottlerocket-specific code and internal dependencies
- **Third-party packages**: External libraries and tools (glibc, docker-engine, etc.)
- **Build changes**: Anything affecting the build system, CI/CD, or development tools
- **Orchestrator changes**: Kubernetes, ECS, or other container orchestration features

### 5. Quality Checks
- Ensure all entries have PR references in format `([#XXX])`
- Verify consistent formatting and indentation
- Group related changes to avoid repetitive entries
- Maintain chronological order within sections
- Check that subsection headers match existing patterns
- **VERIFY each PR's actual content before writing the entry**

## Validation

### Validation Commands
- Verify git log range: `git log --oneline <range>`
- Check merge commits: `git log --merges --grep="pull request" <range>`
- **Check PR contents: `git show <merge_hash> --stat`**
- Validate PR numbers exist in the repository

A successful release preparation includes:
- Two commits in correct order (version bump first, changelog second)
- Twoliter.toml has updated release-version
- CHANGELOG.md has new version section with correct date
- All entries have PR references in `([#XXX])` format
- Third-party package updates do NOT include version numbers
- Commits are signed (`-s -S` flags)

## Common Issues

### Missing PR References
If merge commits don't show PR numbers, check:
```bash
git log --merges --oneline | head -20
```
Look for "Merge pull request #XXX" patterns.

### Incorrect Package Categorization
Always verify with `git show <hash> --stat` to see actual files changed before categorizing as first-party vs third-party.

### Version Number in Third-Party Updates
Remember: third-party package update entries should NOT include version numbers, only package names.

## Example Output Structure
```markdown
# v12.3.0 (2026-01-20)

## OS Changes
- Replace `amazon-ecr-containerd-resolver` with Docker resolver in `host-ctr` ([#760])
- Add MPS control daemon support to `nvidia-k8s-device-plugin` ([#789])

### Third Party Package Updates
- Update `docker-cli-29`, `docker-engine-29` ([#785])
- Patch `containerd-2.1` to update GRPC ([#801])
- Update `libnvme`, `xfsprogs`, `nvme-cli`, `makedumpfile`, `keyutils`, `e2fsprogs` ([#794])
- Update `readline`, `libxcrypt`, `liburcu`, `libcap` ([#795])

[#760]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/760
[#785]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/785
[#789]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/789
[#794]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/794
[#795]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/795
[#801]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/801
```

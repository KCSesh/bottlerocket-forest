# Setup Phase

Verify prerequisites and start local registry.

## Your Goal

Ensure environment is ready for local builds.

## Inputs

- Workspace: Available in context_data["workspace"]

## Procedure

### 1. Verify Docker

```bash
docker --version
docker ps
```

If Docker not running, report failure.

### 2. Start local registry

```bash
(cd $FOREST_ROOT && brdev registry start)
```

### 3. Verify registry

```bash
(cd $FOREST_ROOT && brdev registry status)
curl http://localhost:5000/v2/_catalog
```

### 4. Check core-kit exists

```bash
ls -d kits/bottlerocket-core-kit
```

### 5. Check bottlerocket repo exists

```bash
ls -d bottlerocket
```

## Output Format

Write to `<workspace>/00-setup.md`:

```markdown
# Setup Phase - Complete

## Registry Status
- Running: yes/no
- URL: localhost:5000

## Prerequisites
- Docker: ✓/✗
- Core-kit: ✓/✗
- Bottlerocket repo: ✓/✗

## Ready
yes/no
```

## Completion

Call `respond_to_leader("success", "<output>")` with the markdown output.

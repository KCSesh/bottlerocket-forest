# Build Kit Phase

Build core-kit with settings changes and publish to local registry.

## Your Goal

Build and publish core-kit to localhost:5000.

## Inputs

- Workspace: Available in context_data["workspace"]

## Procedure

### 1. Navigate to core-kit

```bash
cd kits/bottlerocket-core-kit
```

### 2. Ensure Infra.toml configured

Check if `Infra.toml` exists with local vendor:

```bash
cat Infra.toml
```

If missing or incorrect, create:

```bash
cat > Infra.toml << 'EOF'
[vendor.local]
registry = "localhost:5000"
EOF
```

### 3. Build the kit

```bash
make build
```

### 4. Publish to local registry

```bash
make publish VENDOR=local
```

### 5. Verify publication

```bash
curl http://localhost:5000/v2/_catalog
curl http://localhost:5000/v2/bottlerocket-core-kit/tags/list
```

### 6. Extract version

Read the kit version from the build output or Cargo.toml.

## Output Format

Write to `<workspace>/01-build-kit.md`:

```markdown
# Build Kit Phase - Complete

## Kit Details
- Name: bottlerocket-core-kit
- Version: <version>
- Registry: localhost:5000

## Verification
- Published: yes/no
- Available tags: <list>

## Next Step
Update variant Twoliter.toml to use:
```toml
[[kit]]
name = "bottlerocket-core-kit"
version = "<version>"
vendor = "local"
```
```

## Completion

Call `respond_to_leader("success", "<output>")` with the markdown output.

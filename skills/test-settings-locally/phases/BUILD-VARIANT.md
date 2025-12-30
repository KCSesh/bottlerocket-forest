# Build Variant Phase

Configure variant to use local kit and build.

## Your Goal

Build a variant using the locally published core-kit.

## Inputs

- Workspace: Available in context_data["workspace"]
- Kit build results: Available in context_files (01-build-kit.md)

## Procedure

### 1. Extract kit version

Read kit version from 01-build-kit.md.

### 2. Update Twoliter.toml

Edit `bottlerocket/Twoliter.toml`:

```toml
[[kit]]
name = "bottlerocket-core-kit"
version = "<version-from-kit>"
vendor = "local"

[[kit]]
name = "bottlerocket-kernel-kit"
version = "<existing-version>"
vendor = "<existing-vendor>"
```

### 3. Ensure Infra.toml configured

Check `bottlerocket/Infra.toml`:

```toml
[vendor.local]
registry = "localhost:5000"
```

### 4. Update lock file

```bash
cd bottlerocket
./tools/twoliter/twoliter update
```

### 5. Build variant

```bash
cargo make -e BUILDSYS_VARIANT=aws-k8s-1.31
```

Or use default variant:
```bash
cargo make
```

### 6. Locate built image

```bash
ls -lh build/images/*.img
```

## Output Format

Write to `<workspace>/02-build-variant.md`:

```markdown
# Build Variant Phase - Complete

## Configuration
- Core-kit version: <version>
- Core-kit vendor: local
- Variant: <variant-name>

## Build Result
- Image location: build/images/<filename>.img
- Image size: <size>

## Testing
The variant image is ready for testing. Deploy to test environment or use with QEMU/EC2.

## Cleanup
When done:
```bash
(cd $FOREST_ROOT && brdev registry stop)
```
```

## Completion

Call `respond_to_leader("success", "<output>")` with the markdown output.

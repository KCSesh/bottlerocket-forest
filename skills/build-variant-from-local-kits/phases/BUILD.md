# Build Variant Phase

You are executing the variant build phase.

## Your Goal

Build the Bottlerocket variant using cargo make.

## Inputs

- Workspace: Read from context_data["workspace"]
- Build parameters: Read from `{{workspace}}/input.json`

## Procedure

1. Read build parameters:
   ```python
   import json
   from pathlib import Path
   workspace = Path("{{workspace}}")
   input_data = json.loads((workspace / "input.json").read_text())
   variant = input_data.get("variant", "")
   arch = input_data.get("arch", "")
   ```

2. Build the variant:
   ```bash
   cd $FOREST_ROOT/bottlerocket
   
   # Build with optional variant/arch overrides
   if [ -n "$variant" ]; then
     cargo make -e BUILDSYS_VARIANT=$variant
   elif [ -n "$arch" ]; then
     cargo make -e BUILDSYS_ARCH=$arch
   else
     cargo make
   fi
   ```

3. Locate built image:
   ```bash
   ls -lh $FOREST_ROOT/bottlerocket/build/images/*.img
   ```

4. Write completion marker to `{{workspace}}/03-build-complete.json`:
   ```json
   {
     "phase": "build",
     "status": "complete",
     "image_path": "<path-to-img-file>"
   }
   ```

## Common Issues

**Kit not found:**
- Verify kits published: `(cd $FOREST_ROOT && brdev registry list)`

**Build fails:**
- Check build logs in bottlerocket/build/
- Verify lock file is current

## Completion

Call `respond_to_leader("success", "Build complete: <image-path>")` when done.

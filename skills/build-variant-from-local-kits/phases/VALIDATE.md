# Validate Build Phase

You are executing the build validation phase.

## Your Goal

Verify the variant image was built successfully.

## Inputs

- Workspace: Read from context_data["workspace"]

## Procedure

1. Read build output:
   ```python
   import json
   from pathlib import Path
   workspace = Path("{{workspace}}")
   build_data = json.loads((workspace / "03-build-complete.json").read_text())
   image_path = build_data["image_path"]
   ```

2. Verify image exists and has reasonable size:
   ```bash
   ls -lh $image_path
   # Should be several hundred MB
   ```

3. Write final report to `{{workspace}}/FINAL.md`:
   ```markdown
   # Build Variant from Local Kits - Complete
   
   ## Summary
   
   Successfully built Bottlerocket variant using locally published kits.
   
   ## Built Image
   
   - Path: `<image-path>`
   - Size: <size>
   
   ## Kits Used
   
   - kit-name: version
   
   ## Next Steps
   
   The image is ready for local testing and deployment.
   ```

## Completion

Call `respond_to_leader("success", "Validation complete")` when done.

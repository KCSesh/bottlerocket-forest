#!/usr/bin/env python3
import json
import sys
from pathlib import Path

def main():
    workspace = Path(sys.argv[1])
    progress_file = workspace / "progress.json"
    
    if progress_file.exists():
        state = json.loads(progress_file.read_text())
    else:
        state = {"phase": "setup", "completed": []}
    
    phase = state["phase"]
    
    # Phase: setup
    if phase == "setup":
        if "setup" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the setup phase to verify prerequisites and start local registry.",
                "context_files": ["skills/test-settings-locally/phases/SETUP.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "00-setup.md"
            }))
            return
        if not (workspace / "00-setup.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Setup output missing"}))
            return
        state["phase"] = "build-kit"
        state["completed"].append("setup")
    
    # Phase: build-kit
    elif phase == "build-kit":
        if "build-kit" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the build-kit phase to build core-kit with settings changes.",
                "context_files": ["skills/test-settings-locally/phases/BUILD-KIT.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "01-build-kit.md"
            }))
            return
        if not (workspace / "01-build-kit.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Build-kit output missing"}))
            return
        state["phase"] = "build-variant"
        state["completed"].append("build-kit")
    
    # Phase: build-variant
    elif phase == "build-variant":
        if "build-variant" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the build-variant phase to build variant with local kit.",
                "context_files": [
                    "skills/test-settings-locally/phases/BUILD-VARIANT.md",
                    f"{workspace}/01-build-kit.md"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "02-build-variant.md"
            }))
            return
        if not (workspace / "02-build-variant.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Build-variant output missing"}))
            return
        state["phase"] = "done"
        state["completed"].append("build-variant")
    
    # Done
    elif phase == "done":
        if not (workspace / "FINAL.md").exists():
            final = f"""# Test Settings Locally - Complete

All phases completed successfully.

## Results

- Setup: {workspace}/00-setup.md
- Kit Build: {workspace}/01-build-kit.md
- Variant Build: {workspace}/02-build-variant.md

## Next Steps

1. Test the built variant image (see 02-build-variant.md for location)
2. Deploy to test environment if needed
3. Iterate on settings changes as needed

## Cleanup

When done testing:
```bash
(cd $FOREST_ROOT && brdev registry stop)
```
"""
            (workspace / "FINAL.md").write_text(final)
        print(json.dumps({"type": "done"}))
        return
    
    progress_file.write_text(json.dumps(state, indent=2))
    main()

if __name__ == "__main__":
    main()

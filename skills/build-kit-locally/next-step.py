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
    
    if phase == "setup":
        if "setup" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the setup phase for building a kit locally.",
                "context_files": ["skills/build-kit-locally/phases/SETUP.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "01-setup.md"
            }))
            return
        if not (workspace / "01-setup.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Setup output missing"}))
            return
        state["phase"] = "build"
        state["completed"].append("setup")
    
    if phase == "build":
        if "build" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the build phase for building a kit locally.",
                "context_files": ["skills/build-kit-locally/phases/BUILD.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "02-build.md"
            }))
            return
        if not (workspace / "02-build.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Build output missing"}))
            return
        state["phase"] = "publish"
        state["completed"].append("build")
    
    if phase == "publish":
        if "publish" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the publish phase for building a kit locally.",
                "context_files": ["skills/build-kit-locally/phases/PUBLISH.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "03-publish.md"
            }))
            return
        if not (workspace / "03-publish.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Publish output missing"}))
            return
        state["phase"] = "verify"
        state["completed"].append("publish")
    
    if phase == "verify":
        if "verify" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the verify phase for building a kit locally.",
                "context_files": ["skills/build-kit-locally/phases/VERIFY.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "04-verify.md"
            }))
            return
        if not (workspace / "04-verify.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Verify output missing"}))
            return
        state["phase"] = "done"
        state["completed"].append("verify")
    
    if phase == "done":
        print(json.dumps({"type": "done"}))
        return
    
    progress_file.write_text(json.dumps(state, indent=2))
    main()

if __name__ == "__main__":
    main()

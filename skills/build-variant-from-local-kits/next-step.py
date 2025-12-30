#!/usr/bin/env python3
import json
import sys
from pathlib import Path

def main():
    if len(sys.argv) < 2:
        print(json.dumps({"type": "error", "reason": "Usage: next-step.py <workspace>"}))
        sys.exit(1)
    
    workspace = Path(sys.argv[1])
    progress_file = workspace / "progress.json"
    
    if progress_file.exists():
        state = json.loads(progress_file.read_text())
    else:
        state = {"phase": "update_config", "completed": []}
    
    phase = state["phase"]
    
    # Phase: update_config
    if phase == "update_config":
        if "update_config" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the configuration update phase for building a variant from local kits.",
                "context_files": ["skills/build-variant-from-local-kits/phases/UPDATE_CONFIG.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "01-config-updated.json"
            }))
            return
        
        if not (workspace / "01-config-updated.json").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Configuration update output missing"}))
            return
        
        state["phase"] = "update_lock"
        state["completed"].append("update_config")
    
    # Phase: update_lock
    elif phase == "update_lock":
        if "update_lock" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the lock file update phase.",
                "context_files": ["skills/build-variant-from-local-kits/phases/UPDATE_LOCK.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "02-lock-updated.json"
            }))
            return
        
        if not (workspace / "02-lock-updated.json").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Lock file update output missing"}))
            return
        
        state["phase"] = "build"
        state["completed"].append("update_lock")
    
    # Phase: build
    elif phase == "build":
        if "build" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the variant build phase.",
                "context_files": ["skills/build-variant-from-local-kits/phases/BUILD.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "03-build-complete.json"
            }))
            return
        
        if not (workspace / "03-build-complete.json").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Build output missing"}))
            return
        
        state["phase"] = "validate"
        state["completed"].append("build")
    
    # Phase: validate
    elif phase == "validate":
        if "validate" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the build validation phase.",
                "context_files": ["skills/build-variant-from-local-kits/phases/VALIDATE.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "FINAL.md"
            }))
            return
        
        if not (workspace / "FINAL.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Validation output missing"}))
            return
        
        state["phase"] = "done"
        state["completed"].append("validate")
    
    # Done
    elif phase == "done":
        print(json.dumps({"type": "done"}))
        return
    
    progress_file.write_text(json.dumps(state, indent=2))
    main()

if __name__ == "__main__":
    main()

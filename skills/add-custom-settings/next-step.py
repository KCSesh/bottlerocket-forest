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
        state = {"phase": "plan", "completed": []}
    
    phase = state["phase"]
    
    # Phase 1: PLAN
    if phase == "plan":
        if "plan" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the planning phase to gather requirements for adding custom settings.",
                "context_files": ["skills/add-custom-settings/phases/PLAN.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "requirements.json"
            }))
            return
        
        if not (workspace / "requirements.json").exists():
            print(json.dumps({"type": "gate_failed", "reason": "requirements.json missing"}))
            return
        
        state["phase"] = "create-model"
        state["completed"].append("plan")
    
    # Phase 2: CREATE-MODEL
    elif phase == "create-model":
        if "create-model" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the create-settings-model skill using the gathered requirements.",
                "context_files": [
                    "skills/add-custom-settings/phases/CREATE-MODEL.md",
                    f"{workspace}/requirements.json"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "01-model.md"
            }))
            return
        
        if not (workspace / "01-model.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "01-model.md missing"}))
            return
        
        state["phase"] = "wire-variant"
        state["completed"].append("create-model")
    
    # Phase 3: WIRE-VARIANT
    elif phase == "wire-variant":
        if "wire-variant" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the add-settings-to-variant skill using the created model.",
                "context_files": [
                    "skills/add-custom-settings/phases/WIRE-VARIANT.md",
                    f"{workspace}/requirements.json",
                    f"{workspace}/01-model.md"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "02-variant.md"
            }))
            return
        
        if not (workspace / "02-variant.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "02-variant.md missing"}))
            return
        
        state["phase"] = "test"
        state["completed"].append("wire-variant")
    
    # Phase 4: TEST
    elif phase == "test":
        if "test" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the test-settings-locally skill to validate the implementation.",
                "context_files": [
                    "skills/add-custom-settings/phases/TEST.md",
                    f"{workspace}/requirements.json",
                    f"{workspace}/01-model.md",
                    f"{workspace}/02-variant.md"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "03-test.md"
            }))
            return
        
        if not (workspace / "03-test.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "03-test.md missing"}))
            return
        
        state["phase"] = "finalize"
        state["completed"].append("test")
    
    # Phase 5: FINALIZE
    elif phase == "finalize":
        if not (workspace / "FINAL.md").exists():
            print(json.dumps({
                "type": "spawn",
                "prompt": "Create final summary of the add-custom-settings workflow.",
                "context_files": [
                    f"{workspace}/requirements.json",
                    f"{workspace}/01-model.md",
                    f"{workspace}/02-variant.md",
                    f"{workspace}/03-test.md"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "FINAL.md"
            }))
            return
        
        state["phase"] = "done"
        state["completed"].append("finalize")
    
    # Done
    elif phase == "done":
        print(json.dumps({"type": "done"}))
        return
    
    progress_file.write_text(json.dumps(state, indent=2))
    main()

if __name__ == "__main__":
    main()

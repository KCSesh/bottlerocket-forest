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
        state = {"phase": "scaffold", "completed": []}
    
    phase = state["phase"]
    
    # Phase: scaffold
    if phase == "scaffold":
        if "scaffold" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the scaffold phase to create the settings model directory structure and basic files.",
                "context_files": ["skills/create-settings-model/phases/SCAFFOLD.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "00-scaffold.md"
            }))
            return
        
        # Gate: verify scaffold output exists
        if not (workspace / "00-scaffold.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Scaffold output missing"}))
            return
        
        state["phase"] = "implement"
        state["completed"].append("scaffold")
        progress_file.write_text(json.dumps(state, indent=2))
    
    # Phase: implement
    if phase == "implement":
        if "implement" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the implement phase to add SettingsModel trait implementation.",
                "context_files": [
                    "skills/create-settings-model/phases/IMPLEMENT.md",
                    f"{workspace}/00-scaffold.md"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "01-implement.md"
            }))
            return
        
        # Gate: verify implementation output exists
        if not (workspace / "01-implement.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Implementation output missing"}))
            return
        
        state["phase"] = "validate"
        state["completed"].append("implement")
        progress_file.write_text(json.dumps(state, indent=2))
    
    # Phase: validate
    if phase == "validate":
        if "validate" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the validate phase to verify the settings model builds correctly.",
                "context_files": [
                    "skills/create-settings-model/phases/VALIDATE.md",
                    f"{workspace}/00-scaffold.md",
                    f"{workspace}/01-implement.md"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "FINAL.md"
            }))
            return
        
        # Gate: verify final output exists
        if not (workspace / "FINAL.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Validation output missing"}))
            return
        
        state["phase"] = "done"
        state["completed"].append("validate")
        progress_file.write_text(json.dumps(state, indent=2))
    
    # Done
    if phase == "done":
        print(json.dumps({"type": "done"}))
        return
    
    # Continue to next action
    main()

if __name__ == "__main__":
    main()

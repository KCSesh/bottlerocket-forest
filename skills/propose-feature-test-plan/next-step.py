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
        state = {"phase": "verify", "completed": []}
    
    phase = state["phase"]
    
    if phase == "verify":
        if "verify" not in state["completed"]:
            input_data = json.loads((workspace / "input.json").read_text())
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the verify phase to check prerequisites exist.",
                "context_files": ["skills/propose-feature-test-plan/phases/VERIFY.md"],
                "context_data": {
                    "workspace": str(workspace),
                    "feature_number": input_data["feature_number"],
                    "feature_name": input_data["feature_name"]
                },
                "output_file": "verify.json"
            }))
            return
        
        verify_result = json.loads((workspace / "verify.json").read_text())
        if not verify_result.get("all_present", False):
            print(json.dumps({
                "type": "gate_failed",
                "reason": "Prerequisites missing. Complete concept, requirements, and design first."
            }))
            return
        
        state["phase"] = "extract"
        state["completed"].append("verify")
        progress_file.write_text(json.dumps(state, indent=2))
        main()
        return
    
    if phase == "extract":
        if "extract" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the extract phase to pull requirements and constraints.",
                "context_files": ["skills/propose-feature-test-plan/phases/EXTRACT.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "extract.json"
            }))
            return
        
        if not (workspace / "extract.json").exists():
            print(json.dumps({
                "type": "gate_failed",
                "reason": "Extract output missing"
            }))
            return
        
        state["phase"] = "plan"
        state["completed"].append("extract")
        progress_file.write_text(json.dumps(state, indent=2))
        main()
        return
    
    if phase == "plan":
        if "plan" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the plan phase to create the test plan document.",
                "context_files": ["skills/propose-feature-test-plan/phases/PLAN.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "plan-summary.md"
            }))
            return
        
        if not (workspace / "plan-summary.md").exists():
            print(json.dumps({
                "type": "gate_failed",
                "reason": "Plan output missing"
            }))
            return
        
        state["phase"] = "done"
        state["completed"].append("plan")
        progress_file.write_text(json.dumps(state, indent=2))
        main()
        return
    
    if phase == "done":
        print(json.dumps({"type": "done"}))
        return

if __name__ == "__main__":
    main()

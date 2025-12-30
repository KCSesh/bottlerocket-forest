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
        state = {"phase": "setup", "completed": []}
    
    phase = state["phase"]
    
    if phase == "setup":
        if "setup" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the setup phase for creating an implementation plan.",
                "context_files": ["skills/propose-implementation-plan/phases/SETUP.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "00-setup.md"
            }))
            return
        if not (workspace / "00-setup.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Setup output missing"}))
            return
        state["phase"] = "analyze"
        state["completed"].append("setup")
    
    elif phase == "analyze":
        if "analyze" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the analyze phase for studying the design and extracting constraints.",
                "context_files": ["skills/propose-implementation-plan/phases/ANALYZE.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "01-analyze.md"
            }))
            return
        if not (workspace / "01-analyze.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Analyze output missing"}))
            return
        state["phase"] = "plan"
        state["completed"].append("analyze")
    
    elif phase == "plan":
        if "plan" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the plan phase for breaking the feature into atomic commits.",
                "context_files": [
                    "skills/propose-implementation-plan/phases/PLAN.md",
                    f"{workspace}/01-analyze.md"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "02-plan.md"
            }))
            return
        if not (workspace / "02-plan.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Plan output missing"}))
            return
        state["phase"] = "finalize"
        state["completed"].append("plan")
    
    elif phase == "finalize":
        if "finalize" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the finalize phase to validate and write the final implementation plan.",
                "context_files": [
                    "skills/propose-implementation-plan/phases/FINALIZE.md",
                    f"{workspace}/01-analyze.md",
                    f"{workspace}/02-plan.md"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "implementation-plan.md"
            }))
            return
        if not (workspace / "implementation-plan.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Final implementation plan missing"}))
            return
        state["phase"] = "done"
        state["completed"].append("finalize")
    
    elif phase == "done":
        print(json.dumps({"type": "done"}))
        return
    
    progress_file.write_text(json.dumps(state, indent=2))
    main()

if __name__ == "__main__":
    main()

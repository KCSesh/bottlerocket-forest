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
    
    # Load or initialize state
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
                "prompt": "Execute the setup phase for this feature concept.",
                "context_files": ["skills/propose-feature-concept/phases/SETUP.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "setup.json"
            }))
            return
        
        # Gate: setup.json must exist and be valid
        setup_file = workspace / "setup.json"
        if not setup_file.exists():
            print(json.dumps({"type": "gate_failed", "reason": "Setup output missing"}))
            return
        
        try:
            setup_data = json.loads(setup_file.read_text())
            required = ["feature_number", "feature_name", "feature_dir"]
            if not all(k in setup_data for k in required):
                print(json.dumps({"type": "gate_failed", "reason": "Setup output incomplete"}))
                return
        except json.JSONDecodeError:
            print(json.dumps({"type": "gate_failed", "reason": "Setup output invalid JSON"}))
            return
        
        state["phase"] = "draft"
        state["completed"].append("setup")
        progress_file.write_text(json.dumps(state, indent=2))
        # Recurse to get next action
        main()
        return
    
    # Phase: draft
    if phase == "draft":
        if "draft" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the draft phase for this feature concept.",
                "context_files": [
                    "skills/propose-feature-concept/phases/DRAFT.md",
                    f"{workspace}/setup.json"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "draft-complete.txt"
            }))
            return
        
        # Gate: concept.md must exist in feature directory
        setup_file = workspace / "setup.json"
        setup_data = json.loads(setup_file.read_text())
        concept_file = Path(setup_data["feature_dir"]) / "concept.md"
        
        if not concept_file.exists():
            print(json.dumps({"type": "gate_failed", "reason": "Concept document not created"}))
            return
        
        state["phase"] = "review"
        state["completed"].append("draft")
        progress_file.write_text(json.dumps(state, indent=2))
        # Recurse to get next action
        main()
        return
    
    # Phase: review
    if phase == "review":
        if "review" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the review phase for this feature concept.",
                "context_files": [
                    "skills/propose-feature-concept/phases/REVIEW.md",
                    f"{workspace}/setup.json"
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "review.md"
            }))
            return
        
        # Gate: review.md must exist
        if not (workspace / "review.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Review output missing"}))
            return
        
        state["phase"] = "done"
        state["completed"].append("review")
        progress_file.write_text(json.dumps(state, indent=2))
        print(json.dumps({"type": "done"}))
        return
    
    # Done
    if phase == "done":
        print(json.dumps({"type": "done"}))
        return
    
    # Unknown phase
    print(json.dumps({"type": "error", "reason": f"Unknown phase: {phase}"}))

if __name__ == "__main__":
    main()

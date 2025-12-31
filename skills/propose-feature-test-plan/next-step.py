#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["verify", "extract", "plan", "done"]

def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("workspace")
    p.add_argument("--phase-result", choices=["success", "failure"])
    return p.parse_args()

def load_state(workspace):
    progress = workspace / "progress.json"
    if progress.exists():
        return json.loads(progress.read_text())
    return {"phase": PHASES[0], "completed": [], "retries": 0}

def save_state(workspace, state):
    (workspace / "progress.json").write_text(json.dumps(state, indent=2))

def next_phase(current):
    idx = PHASES.index(current)
    return PHASES[idx + 1] if idx + 1 < len(PHASES) else "done"

def main():
    args = parse_args()
    workspace = Path(args.workspace)
    state = load_state(workspace)

    if args.phase_result == "success":
        state["completed"].append(state["phase"])
        state["phase"] = next_phase(state["phase"])
        state["retries"] = 0
        save_state(workspace, state)
    elif args.phase_result == "failure":
        state["retries"] = state.get("retries", 0) + 1
        if state["retries"] >= 3:
            print(json.dumps({"type": "blocked", "reason": f"Phase {state['phase']} failed 3 times"}))
            return
        save_state(workspace, state)

    phase = state["phase"]

    if phase == "done":
        print(json.dumps({"type": "done"}))
        return

    if phase == "verify":
        input_file = workspace / "input.json"
        if input_file.exists():
            input_data = json.loads(input_file.read_text())
        else:
            input_data = {"feature_number": "0000", "feature_name": "unknown"}
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the verify phase to check prerequisites exist.",
            "context_files": ["skills/propose-feature-test-plan/phases/VERIFY.md"],
            "context_data": {
                "workspace": str(workspace),
                "feature_number": input_data.get("feature_number", "0000"),
                "feature_name": input_data.get("feature_name", "unknown")
            },
            "output_file": "verify.json"
        }))
        return

    if phase == "extract":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the extract phase to pull requirements and constraints.",
            "context_files": ["skills/propose-feature-test-plan/phases/EXTRACT.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "extract.json"
        }))
        return

    if phase == "plan":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the plan phase to create the test plan document.",
            "context_files": ["skills/propose-feature-test-plan/phases/PLAN.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
        return

if __name__ == "__main__":
    main()

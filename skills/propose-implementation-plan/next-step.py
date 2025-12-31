#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["setup", "analyze", "plan", "finalize", "done"]

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

    if phase == "setup":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the setup phase for creating an implementation plan.",
            "context_files": ["skills/propose-implementation-plan/phases/SETUP.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "00-setup.md"
        }))
        return

    if phase == "analyze":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the analyze phase for studying the design and extracting constraints.",
            "context_files": ["skills/propose-implementation-plan/phases/ANALYZE.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "01-analyze.md"
        }))
        return

    if phase == "plan":
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

    if phase == "finalize":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the finalize phase to validate and write the final implementation plan.",
            "context_files": [
                "skills/propose-implementation-plan/phases/FINALIZE.md",
                f"{workspace}/01-analyze.md",
                f"{workspace}/02-plan.md"
            ],
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
        return

if __name__ == "__main__":
    main()

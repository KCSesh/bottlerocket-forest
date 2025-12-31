#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["setup", "draft", "review", "done"]

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
            "prompt": "Execute the setup phase for this feature concept.",
            "context_files": ["skills/propose-feature-concept/phases/SETUP.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "setup.json"
        }))
        return

    if phase == "draft":
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

    if phase == "review":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the review phase for this feature concept.",
            "context_files": [
                "skills/propose-feature-concept/phases/REVIEW.md",
                f"{workspace}/setup.json"
            ],
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
        return

if __name__ == "__main__":
    main()

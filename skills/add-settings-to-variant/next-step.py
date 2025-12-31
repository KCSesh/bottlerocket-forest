#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["locate", "integrate", "verify", "done"]

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

    if phase == "locate":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the locate phase to find the variant's settings-plugins crate.",
            "context_files": ["skills/add-settings-to-variant/phases/LOCATE.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "01-locate.md"
        }))
        return

    if phase == "integrate":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the integrate phase to add the settings model to the plugin.",
            "context_files": [
                "skills/add-settings-to-variant/phases/INTEGRATE.md",
                f"{workspace}/01-locate.md"
            ],
            "context_data": {"workspace": str(workspace)},
            "output_file": "02-integrate.md"
        }))
        return

    if phase == "verify":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the verify phase to build and confirm the integration.",
            "context_files": [
                "skills/add-settings-to-variant/phases/VERIFY.md",
                f"{workspace}/01-locate.md",
                f"{workspace}/02-integrate.md"
            ],
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
        return

if __name__ == "__main__":
    main()

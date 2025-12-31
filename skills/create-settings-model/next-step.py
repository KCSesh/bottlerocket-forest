#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["scaffold", "implement", "validate", "done"]

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

    if phase == "scaffold":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the scaffold phase to create the settings model directory structure and basic files.",
            "context_files": ["skills/create-settings-model/phases/SCAFFOLD.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "00-scaffold.md"
        }))
        return

    if phase == "implement":
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

    if phase == "validate":
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

if __name__ == "__main__":
    main()

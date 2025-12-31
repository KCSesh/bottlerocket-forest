#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["plan", "create-model", "wire-variant", "test", "finalize", "done"]

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

    if phase == "plan":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the planning phase to gather requirements for adding custom settings.",
            "context_files": ["skills/add-custom-settings/phases/PLAN.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "requirements.json"
        }))
        return

    if phase == "create-model":
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

    if phase == "wire-variant":
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

    if phase == "test":
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

    if phase == "finalize":
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

if __name__ == "__main__":
    main()

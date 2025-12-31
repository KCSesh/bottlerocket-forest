#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["setup", "build-kit", "build-variant", "done"]

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
            "prompt": "Execute the setup phase to verify prerequisites and start local registry.",
            "context_files": ["skills/test-settings-locally/phases/SETUP.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "00-setup.md"
        }))
        return

    if phase == "build-kit":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the build-kit phase to build core-kit with settings changes.",
            "context_files": ["skills/test-settings-locally/phases/BUILD-KIT.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "01-build-kit.md"
        }))
        return

    if phase == "build-variant":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the build-variant phase to build variant with local kit.",
            "context_files": [
                "skills/test-settings-locally/phases/BUILD-VARIANT.md",
                f"{workspace}/01-build-kit.md"
            ],
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
        return

if __name__ == "__main__":
    main()

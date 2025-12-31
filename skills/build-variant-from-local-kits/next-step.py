#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["update_config", "update_lock", "build", "validate", "done"]

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

    if phase == "update_config":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the configuration update phase for building a variant from local kits.",
            "context_files": ["skills/build-variant-from-local-kits/phases/UPDATE_CONFIG.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "01-config-updated.json"
        }))
        return

    if phase == "update_lock":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the lock file update phase.",
            "context_files": ["skills/build-variant-from-local-kits/phases/UPDATE_LOCK.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "02-lock-updated.json"
        }))
        return

    if phase == "build":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the variant build phase.",
            "context_files": ["skills/build-variant-from-local-kits/phases/BUILD.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "03-build-complete.json"
        }))
        return

    if phase == "validate":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the build validation phase.",
            "context_files": ["skills/build-variant-from-local-kits/phases/VALIDATE.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
        return

if __name__ == "__main__":
    main()

#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["fetch", "update_kits", "update_bottlerocket", "validate", "done"]

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

    if phase == "fetch":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the fetch phase to get SHA256 checksums.",
            "context_files": ["skills/update-twoliter/phases/FETCH.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "checksums.json"
        }))
        return

    if phase == "update_kits":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the update-kits phase to update all kit Makefiles.",
            "context_files": ["skills/update-twoliter/phases/UPDATE_KITS.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "kits-updated.txt"
        }))
        return

    if phase == "update_bottlerocket":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the update-bottlerocket phase to update Makefile.toml.",
            "context_files": ["skills/update-twoliter/phases/UPDATE_BOTTLEROCKET.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "bottlerocket-updated.txt"
        }))
        return

    if phase == "validate":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the validate phase to verify all commits.",
            "context_files": ["skills/update-twoliter/phases/VALIDATE.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
        return

if __name__ == "__main__":
    main()

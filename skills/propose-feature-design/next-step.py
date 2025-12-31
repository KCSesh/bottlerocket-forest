#!/usr/bin/env python3
import argparse
import json
from pathlib import Path

PHASES = ["verify", "prepare", "design", "done"]

def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("workspace")
    p.add_argument("--phase-result", choices=["success", "failure"])
    return p.parse_args()

def load_state(workspace):
    progress = workspace / ".design-progress.json"
    if progress.exists():
        return json.loads(progress.read_text())
    return {"phase": PHASES[0], "completed": [], "retries": 0}

def save_state(workspace, state):
    (workspace / ".design-progress.json").write_text(json.dumps(state, indent=2))

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
    feature_name = workspace.name
    parts = feature_name.split("-", 1)
    feature_number = parts[0] if len(parts) > 1 else "0000"
    feature_slug = parts[1] if len(parts) > 1 else feature_name

    if phase == "done":
        print(json.dumps({"type": "done"}))
        return

    if phase == "verify":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the verify phase to check prerequisites exist.",
            "context_files": ["skills/propose-feature-design/phases/VERIFY.md"],
            "context_data": {"feature_dir": str(workspace)},
            "output_file": ".verify-result.txt"
        }))
        return

    if phase == "prepare":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the prepare phase to copy template and gather context.",
            "context_files": ["skills/propose-feature-design/phases/PREPARE.md"],
            "context_data": {
                "feature_dir": str(workspace),
                "feature_number": feature_number,
                "feature_name": feature_slug
            },
            "output_file": ".prepare-result.txt"
        }))
        return

    if phase == "design":
        forest_root = Path.cwd()
        planning_dir = forest_root / "planning" / f"{feature_number}-{feature_slug}"
        idea_honing = planning_dir / "idea-honing.md"
        ctx_files = [
            "skills/propose-feature-design/phases/DESIGN.md",
            f"{workspace}/concept.md",
            f"{workspace}/requirements.md",
            f"{workspace}/design.md"
        ]
        if idea_honing.exists():
            ctx_files.append(str(idea_honing))
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the design phase to fill in the design document.",
            "context_files": ctx_files,
            "context_data": {
                "feature_dir": str(workspace),
                "feature_number": feature_number,
                "feature_name": feature_slug,
                "idea_honing_exists": idea_honing.exists()
            },
            "output_file": ".design-result.txt"
        }))
        return

if __name__ == "__main__":
    main()

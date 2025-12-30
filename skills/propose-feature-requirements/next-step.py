#!/usr/bin/env python3
import json
import sys
from pathlib import Path

def main():
  workspace = Path(sys.argv[1])
  progress_file = workspace / "progress.json"
  
  if progress_file.exists():
    state = json.loads(progress_file.read_text())
  else:
    state = {"phase": "verify", "completed": []}
  
  phase = state["phase"]
  
  if phase == "verify":
    if "verify" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the verify phase for this requirements specification task.",
        "context_files": ["skills/propose-feature-requirements/phases/VERIFY.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "00-verify.md"
      }))
      return
    if not (workspace / "00-verify.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Verify output missing"}))
      return
    state["phase"] = "setup"
    state["completed"].append("verify")
  
  if phase == "setup":
    if "setup" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the setup phase for this requirements specification task.",
        "context_files": ["skills/propose-feature-requirements/phases/SETUP.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "01-setup.md"
      }))
      return
    if not (workspace / "01-setup.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Setup output missing"}))
      return
    state["phase"] = "write"
    state["completed"].append("setup")
  
  if phase == "write":
    if "write" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the write phase for this requirements specification task.",
        "context_files": ["skills/propose-feature-requirements/phases/WRITE.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "02-write.md"
      }))
      return
    if not (workspace / "02-write.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Write output missing"}))
      return
    state["phase"] = "validate"
    state["completed"].append("write")
  
  if phase == "validate":
    if "validate" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the validate phase for this requirements specification task.",
        "context_files": ["skills/propose-feature-requirements/phases/VALIDATE.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "03-validate.md"
      }))
      return
    if not (workspace / "03-validate.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Validate output missing"}))
      return
    state["phase"] = "done"
    state["completed"].append("validate")
  
  if phase == "done":
    print(json.dumps({"type": "done"}))
    return
  
  progress_file.write_text(json.dumps(state, indent=2))
  main()

if __name__ == "__main__":
  main()

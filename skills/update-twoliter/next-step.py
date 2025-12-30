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
    state = {"phase": "fetch", "completed": []}
  
  phase = state["phase"]
  
  if phase == "fetch":
    if "fetch" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the fetch phase to get SHA256 checksums.",
        "context_files": ["skills/update-twoliter/phases/FETCH.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "checksums.json"
      }))
      return
    if not (workspace / "checksums.json").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Checksums file missing"}))
      return
    state["phase"] = "update_kits"
    state["completed"].append("fetch")
  
  if phase == "update_kits":
    if "update_kits" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the update-kits phase to update all kit Makefiles.",
        "context_files": ["skills/update-twoliter/phases/UPDATE_KITS.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "kits-updated.txt"
      }))
      return
    if not (workspace / "kits-updated.txt").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Kits update confirmation missing"}))
      return
    state["phase"] = "update_bottlerocket"
    state["completed"].append("update_kits")
  
  if phase == "update_bottlerocket":
    if "update_bottlerocket" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the update-bottlerocket phase to update Makefile.toml.",
        "context_files": ["skills/update-twoliter/phases/UPDATE_BOTTLEROCKET.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "bottlerocket-updated.txt"
      }))
      return
    if not (workspace / "bottlerocket-updated.txt").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Bottlerocket update confirmation missing"}))
      return
    state["phase"] = "validate"
    state["completed"].append("update_bottlerocket")
  
  if phase == "validate":
    if "validate" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the validate phase to verify all commits.",
        "context_files": ["skills/update-twoliter/phases/VALIDATE.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "FINAL.md"
      }))
      return
    if not (workspace / "FINAL.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Validation report missing"}))
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

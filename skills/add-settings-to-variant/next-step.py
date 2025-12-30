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
    state = {"phase": "locate", "completed": []}
  
  phase = state["phase"]
  
  # Phase: locate
  if phase == "locate":
    if "locate" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the locate phase to find the variant's settings-plugins crate.",
        "context_files": ["skills/add-settings-to-variant/phases/LOCATE.md"],
        "context_data": {"workspace": str(workspace)},
        "output_file": "01-locate.md"
      }))
      return
    
    if not (workspace / "01-locate.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Locate output missing"}))
      return
    
    state["phase"] = "integrate"
    state["completed"].append("locate")
    progress_file.write_text(json.dumps(state, indent=2))
    main()
    return
  
  # Phase: integrate
  if phase == "integrate":
    if "integrate" not in state["completed"]:
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
    
    if not (workspace / "02-integrate.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Integrate output missing"}))
      return
    
    state["phase"] = "verify"
    state["completed"].append("integrate")
    progress_file.write_text(json.dumps(state, indent=2))
    main()
    return
  
  # Phase: verify
  if phase == "verify":
    if "verify" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the verify phase to build and confirm the integration.",
        "context_files": [
          "skills/add-settings-to-variant/phases/VERIFY.md",
          f"{workspace}/01-locate.md",
          f"{workspace}/02-integrate.md"
        ],
        "context_data": {"workspace": str(workspace)},
        "output_file": "03-verify.md"
      }))
      return
    
    if not (workspace / "03-verify.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Verify output missing"}))
      return
    
    state["phase"] = "done"
    state["completed"].append("verify")
    progress_file.write_text(json.dumps(state, indent=2))
    
    # Create FINAL.md
    final = f"""# Add Settings to Variant: Complete

Settings model successfully integrated into variant.

## Summary

{(workspace / "01-locate.md").read_text()}

---

{(workspace / "02-integrate.md").read_text()}

---

{(workspace / "03-verify.md").read_text()}
"""
    (workspace / "FINAL.md").write_text(final)
    main()
    return
  
  # Done
  if phase == "done":
    print(json.dumps({"type": "done"}))
    return

if __name__ == "__main__":
  main()

#!/usr/bin/env python3
import json
import sys
from pathlib import Path

def main():
  if len(sys.argv) < 2:
    print(json.dumps({"type": "error", "reason": "Usage: next-step.py <feature_dir>"}))
    return
  
  feature_dir = Path(sys.argv[1])
  progress_file = feature_dir / ".design-progress.json"
  
  if progress_file.exists():
    state = json.loads(progress_file.read_text())
  else:
    state = {"phase": "verify", "completed": []}
  
  phase = state["phase"]
  forest_root = Path.cwd()
  
  if phase == "verify":
    if "verify" not in state["completed"]:
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the verify phase to check prerequisites exist.",
        "context_files": ["skills/propose-feature-design/phases/VERIFY.md"],
        "context_data": {"feature_dir": str(feature_dir)},
        "output_file": ".verify-result.txt"
      }))
      return
    
    verify_result = feature_dir / ".verify-result.txt"
    if not verify_result.exists():
      print(json.dumps({"type": "gate_failed", "reason": "Verify phase did not produce output"}))
      return
    
    if not (feature_dir / "concept.md").exists() or not (feature_dir / "requirements.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Prerequisites missing (concept.md or requirements.md)"}))
      return
    
    state["phase"] = "prepare"
    state["completed"].append("verify")
    progress_file.write_text(json.dumps(state, indent=2))
    main()
    return
  
  if phase == "prepare":
    if "prepare" not in state["completed"]:
      feature_name = feature_dir.name
      parts = feature_name.split("-", 1)
      feature_number = parts[0] if len(parts) > 1 else "0000"
      feature_slug = parts[1] if len(parts) > 1 else feature_name
      
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the prepare phase to copy template and gather context.",
        "context_files": ["skills/propose-feature-design/phases/PREPARE.md"],
        "context_data": {
          "feature_dir": str(feature_dir),
          "feature_number": feature_number,
          "feature_name": feature_slug
        },
        "output_file": ".prepare-result.txt"
      }))
      return
    
    prepare_result = feature_dir / ".prepare-result.txt"
    if not prepare_result.exists():
      print(json.dumps({"type": "gate_failed", "reason": "Prepare phase did not produce output"}))
      return
    
    if not (feature_dir / "design.md").exists():
      print(json.dumps({"type": "gate_failed", "reason": "Template not copied (design.md missing)"}))
      return
    
    state["phase"] = "design"
    state["completed"].append("prepare")
    progress_file.write_text(json.dumps(state, indent=2))
    main()
    return
  
  if phase == "design":
    if "design" not in state["completed"]:
      feature_name = feature_dir.name
      parts = feature_name.split("-", 1)
      feature_number = parts[0] if len(parts) > 1 else "0000"
      feature_slug = parts[1] if len(parts) > 1 else feature_name
      
      planning_dir = forest_root / "planning" / f"{feature_number}-{feature_slug}"
      idea_honing = planning_dir / "idea-honing.md"
      
      ctx_files = [
        "skills/propose-feature-design/phases/DESIGN.md",
        f"{feature_dir}/concept.md",
        f"{feature_dir}/requirements.md",
        f"{feature_dir}/design.md"
      ]
      
      if idea_honing.exists():
        ctx_files.append(str(idea_honing))
      
      print(json.dumps({
        "type": "spawn",
        "prompt": "Execute the design phase to fill in the design document.",
        "context_files": ctx_files,
        "context_data": {
          "feature_dir": str(feature_dir),
          "feature_number": feature_number,
          "feature_name": feature_slug,
          "idea_honing_exists": idea_honing.exists()
        },
        "output_file": ".design-result.txt"
      }))
      return
    
    design_result = feature_dir / ".design-result.txt"
    if not design_result.exists():
      print(json.dumps({"type": "gate_failed", "reason": "Design phase did not produce output"}))
      return
    
    design_file = feature_dir / "design.md"
    if not design_file.exists():
      print(json.dumps({"type": "gate_failed", "reason": "Design document not created"}))
      return
    
    content = design_file.read_text()
    if len(content) < 500:
      print(json.dumps({"type": "gate_failed", "reason": "Design document appears incomplete (too short)"}))
      return
    
    state["phase"] = "done"
    state["completed"].append("design")
    progress_file.write_text(json.dumps(state, indent=2))
    main()
    return
  
  if phase == "done":
    print(json.dumps({"type": "done"}))
    return
  
  print(json.dumps({"type": "error", "reason": f"Unknown phase: {phase}"}))

if __name__ == "__main__":
  main()

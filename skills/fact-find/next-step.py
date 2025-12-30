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
        state = {"phase": "search", "completed": []}
    
    phase = state["phase"]
    
    if phase == "search":
        if "search" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the search phase to find relevant files for this question.",
                "context_files": ["skills/fact-find/phases/SEARCH.md"],
                "context_data": {"workspace": str(workspace)},
                "output_file": "00-search.md"
            }))
            return
        if not (workspace / "00-search.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Search output missing"}))
            return
        state["phase"] = "answer"
        state["completed"].append("search")
        progress_file.write_text(json.dumps(state, indent=2))
    
    if phase == "answer":
        if "answer" not in state["completed"]:
            print(json.dumps({
                "type": "spawn",
                "prompt": "Execute the answer phase to formulate the response with citations.",
                "context_files": [
                    "skills/fact-find/phases/ANSWER.md",
                    str(workspace / "00-search.md")
                ],
                "context_data": {"workspace": str(workspace)},
                "output_file": "FINAL.md"
            }))
            return
        if not (workspace / "FINAL.md").exists():
            print(json.dumps({"type": "gate_failed", "reason": "Final answer missing"}))
            return
        state["phase"] = "done"
        state["completed"].append("answer")
        progress_file.write_text(json.dumps(state, indent=2))
    
    if phase == "done":
        print(json.dumps({"type": "done"}))
        return

if __name__ == "__main__":
    main()

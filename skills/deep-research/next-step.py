#!/usr/bin/env python3
import argparse
import json
import re
from pathlib import Path

PHASES = ["scout", "research", "assemble", "verify", "done"]

def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("workspace")
    p.add_argument("--phase-result", choices=["success", "failure"])
    return p.parse_args()

def load_state(workspace):
    progress = workspace / "progress.json"
    if progress.exists():
        return json.loads(progress.read_text())
    return {"phase": PHASES[0], "completed": [], "retries": 0, "researched": [], "verified": []}

def save_state(workspace, state):
    (workspace / "progress.json").write_text(json.dumps(state, indent=2))

def next_phase(current):
    idx = PHASES.index(current)
    return PHASES[idx + 1] if idx + 1 < len(PHASES) else "done"

def parse_scout_subquestions(scout_path):
    if not scout_path.exists():
        return []
    content = scout_path.read_text()
    questions = []
    current = {}
    for line in content.split('\n'):
        if line.startswith('### ') and re.match(r'### \d+\.', line):
            if current:
                questions.append(current)
            current = {'text': re.sub(r'^### \d+\.\s*', '', line)}
        elif current and line.startswith('- **Type:**'):
            current['type'] = line.split(':', 1)[1].strip().lower()
        elif current and line.startswith('- **Key files:**'):
            current['files'] = line.split(':', 1)[1].strip()
    if current:
        questions.append(current)
    return questions

def count_research_files(workspace):
    return len([f for f in workspace.glob('[0-9][0-9]-*.md') if f.name != '00-scout.md'])

def extract_citations(final_path):
    if not final_path.exists():
        return []
    content = final_path.read_text()
    return sorted(set(re.findall(r'<sup>\[(\d+)\]</sup>', content)), key=int)

def main():
    args = parse_args()
    workspace = Path(args.workspace)
    state = load_state(workspace)

    if args.phase_result == "success":
        if state["phase"] == "research":
            pass
        elif state["phase"] == "verify":
            pass
        else:
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

    if phase == "scout":
        print(json.dumps({
            "type": "spawn",
            "prompt": "Execute the scout phase for this research task.",
            "context_files": ["skills/deep-research/phases/SCOUT.md"],
            "context_data": {"workspace": str(workspace)},
            "output_file": "00-scout.md"
        }))
        return

    if phase == "research":
        scout_file = workspace / "00-scout.md"
        subqs = parse_scout_subquestions(scout_file)
        if not subqs:
            print(json.dumps({"type": "blocked", "reason": "Scout produced no sub-questions"}))
            return
        researched = state.get("researched", [])
        for i, sq in enumerate(subqs):
            if i in researched:
                continue
            file_num = str(i + 1).zfill(2)
            slug = re.sub(r'[^a-z0-9]+', '-', sq['text'].lower()[:40]).strip('-')
            output_file = f'{file_num}-{slug}.md'
            print(json.dumps({
                "type": "spawn",
                "prompt": f"Research this sub-question: {sq['text']}",
                "context_files": ["skills/deep-research/phases/RESEARCH.md", f"{workspace}/00-scout.md"],
                "context_data": {
                    "workspace": str(workspace),
                    "subquestion": sq["text"],
                    "subquestion_type": sq.get("type", "fact-find"),
                    "key_files": sq.get("files", ""),
                    "output_file": output_file
                },
                "output_file": output_file
            }))
            state["researched"].append(i)
            save_state(workspace, state)
            return
        state["completed"].append("research")
        state["phase"] = "assemble"
        save_state(workspace, state)
        main()
        return

    if phase == "assemble":
        research_files = sorted([str(f) for f in workspace.glob('[0-9][0-9]-*.md')])
        print(json.dumps({
            "type": "spawn",
            "prompt": "Assemble the research into a final document.",
            "context_files": ["skills/deep-research/phases/ASSEMBLE.md"] + research_files,
            "context_data": {"workspace": str(workspace)},
            "output_file": "FINAL.md"
        }))
        return

    if phase == "verify":
        final_file = workspace / "FINAL.md"
        citations = extract_citations(final_file)
        verified = state.get("verified", [])
        final_content = final_file.read_text() if final_file.exists() else ""
        for cit in citations:
            if cit in verified:
                continue
            pattern = rf'([^.]*<sup>\[{cit}\]</sup>[^.]*\.)'
            match = re.search(pattern, final_content)
            claim = match.group(1) if match else f"Citation [{cit}]"
            print(json.dumps({
                "type": "spawn",
                "prompt": f"Verify citation [{cit}]",
                "context_files": ["skills/deep-research/phases/VERIFY.md", f"{workspace}/FINAL.md"],
                "context_data": {
                    "workspace": str(workspace),
                    "citation_num": cit,
                    "claim": claim
                },
                "output_file": f"verify-{cit}.txt"
            }))
            state["verified"].append(cit)
            save_state(workspace, state)
            return
        state["completed"].append("verify")
        state["phase"] = "done"
        save_state(workspace, state)
        print(json.dumps({"type": "done"}))
        return

if __name__ == "__main__":
    main()

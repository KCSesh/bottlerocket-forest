#!/usr/bin/env python3
"""State machine driver for implement-commit skill.

Orchestrator calls: init -> (next -> report)* until done or exit 1.
Driver runs gates/bash directly. Only spawns need orchestrator.
"""

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass, field, asdict
from enum import Enum
from pathlib import Path


# ═══════════════════════════════════════════════════════════════════════════════
# CONSTANTS & CONFIGURATION
# ═══════════════════════════════════════════════════════════════════════════════

MAX_GATE_RETRIES = 2
MAX_STYLE_CYCLES = 3


# ═══════════════════════════════════════════════════════════════════════════════
# STATE DEFINITIONS
# ═══════════════════════════════════════════════════════════════════════════════

class State(str, Enum):
    EXPLORE_GATES = "explore_gates"
    EXPLORE_FORMATTERS = "explore_formatters"
    DESIGNER_PHASE = "designer_phase"
    DESIGNER_GATE = "designer_gate"
    DESIGNER_STYLE = "designer_style"
    TESTER_PHASE = "tester_phase"
    TESTER_GATE = "tester_gate"
    TESTER_STYLE = "tester_style"
    IMPLEMENTOR_PHASE = "implementor_phase"
    IMPLEMENTOR_GATE = "implementor_gate"
    IMPLEMENTOR_STYLE = "implementor_style"
    SCOPE_REVIEW = "scope_review"
    FORMAT = "format"
    COMMIT = "commit"
    DONE = "done"
    NEEDS_LLM = "needs_llm"


# ═══════════════════════════════════════════════════════════════════════════════
# COMMIT STATE
# ═══════════════════════════════════════════════════════════════════════════════

@dataclass
class CommitState:
    plan_path: str
    commit_id: int
    workspace: str
    state: str = State.EXPLORE_GATES.value
    # Discovered commands
    designer_gate: str = ""
    test_gate: str = ""
    fmt_cmd: str = ""
    lint_cmd: str = ""
    # Commit info (loaded from plan)
    commit_title: str = ""
    commit_message: str = ""
    commit_files: list[str] = field(default_factory=list)
    commit_tests: list[str] = field(default_factory=list)
    # Tracking
    files_touched: list[str] = field(default_factory=list)
    gate_retries: int = 0
    style_cycles: int = 0
    last_gate_error: str = ""
    last_style_violations: list[str] = field(default_factory=list)
    escalation_context: str = ""

    def to_json(self) -> str:
        return json.dumps(asdict(self), indent=2)

    @classmethod
    def from_json(cls, data: str) -> "CommitState":
        return cls(**json.loads(data))


# ═══════════════════════════════════════════════════════════════════════════════
# PROMPT TEMPLATES
# ═══════════════════════════════════════════════════════════════════════════════

PROMPTS = {
    "explore_gates": lambda st: f"""Determine build and test gate commands for this project.

Workspace: {st.workspace}
Files to be modified: {st.commit_files}

Return JSON: {{"designer_gate": "<cargo check or equivalent>", "test_gate": "<cargo test or equivalent>"}}

Gates VALIDATE code (compile, run tests) - they never modify files.
Scope commands to affected crates/packages when possible.
Use --release if needed for ARM compatibility.""",

    "explore_formatters": lambda st: f"""Determine format and lint-fix commands for this project.

Workspace: {st.workspace}
Files to be modified: {st.commit_files}

Return JSON: {{"fmt_cmd": "<cargo fmt or equivalent>", "lint_cmd": "<cargo clippy --fix or equivalent>"}}

Formatters MODIFY code to fix style - they change files.""",

    "phase": lambda phase, st, retry="": f"""Execute {phase} phase for commit {st.commit_id}: {st.commit_title}

Workspace: {st.workspace}
Files: {st.commit_files}
{f"Tests to write: {st.commit_tests}" if phase == "tester" and st.commit_tests else ""}

Follow the {phase}.md agent instructions and style guide.{f'''

PREVIOUS ATTEMPT FAILED:
{retry}''' if retry else ''}""",

    "style_review": lambda phase, st: f"""Review {phase} phase style compliance.

Files to review: {st.files_touched}
Phase: {phase}

Return JSON: {{"status": "accept" | "violations", "violations": [...]}}""",

    "style_fix": lambda phase, st: f"""Fix style violations for {phase} phase of commit {st.commit_id}.

Violations: {st.last_style_violations}
Files: {st.files_touched}

Fix the violations while maintaining correctness.""",

    "scope_review": lambda st: f"""Review scope compliance for commit {st.commit_id}: {st.commit_title}

Allowed files: {st.commit_files}
Actual files touched: {st.files_touched}

Return ACCEPT or VIOLATIONS: [...]""",

    "arbiter": lambda st: f"""Style review exceeded {MAX_STYLE_CYCLES} cycles.

Remaining violations: {st.last_style_violations}

Decide: proceed with noted style debt, or reject?
Return JSON: {{"proceed": true|false, "reasoning": "..."}}""",
}


# ═══════════════════════════════════════════════════════════════════════════════
# UTILITIES
# ═══════════════════════════════════════════════════════════════════════════════

def load_plan(path: str) -> dict:
    try:
        import tomllib
        with open(path, "rb") as f:
            return tomllib.load(f)
    except ImportError:
        import toml
        with open(path, "r") as f:
            return toml.load(f)


def run_cmd(cmd: str, cwd: str) -> tuple[bool, str]:
    """Run command, return (success, output)."""
    try:
        r = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True, timeout=300)
        return r.returncode == 0, r.stdout + r.stderr
    except subprocess.TimeoutExpired:
        return False, "Command timed out"
    except Exception as e:
        return False, str(e)


def action(name: str, **kwargs) -> dict:
    return {"action": name, **kwargs}


def escalate(st: CommitState, reason: str, state_path: str) -> None:
    """Exit 1 with state for LLM intervention."""
    st.state = State.NEEDS_LLM.value
    st.escalation_context = reason
    Path(state_path).write_text(st.to_json())
    print(json.dumps({"error": reason, "state_path": state_path}), file=sys.stderr)
    sys.exit(1)


def advance_from_style(st: CommitState) -> None:
    """Move to next phase after style acceptance."""
    st.style_cycles = 0
    transitions = {
        State.DESIGNER_STYLE: State.TESTER_PHASE,
        State.TESTER_STYLE: State.IMPLEMENTOR_PHASE,
        State.IMPLEMENTOR_STYLE: State.SCOPE_REVIEW,
    }
    st.state = transitions[State(st.state)].value


def handle_gate_failure(st: CommitState, error: str, retry_state: State, state_path: str) -> dict:
    st.last_gate_error = error
    st.gate_retries += 1
    if st.gate_retries > MAX_GATE_RETRIES:
        escalate(st, f"Gate failed after {MAX_GATE_RETRIES} retries: {error}", state_path)
    st.state = retry_state.value
    return next_action(st, state_path)


# ═══════════════════════════════════════════════════════════════════════════════
# STATE HANDLERS (next_action dispatch)
# ═══════════════════════════════════════════════════════════════════════════════

def handle_explore_gates(st, state_path):
    return action("spawn", prompt=PROMPTS["explore_gates"](st), response_schema="GateDiscovery")


def handle_explore_formatters(st, state_path):
    return action("spawn", prompt=PROMPTS["explore_formatters"](st), response_schema="FormatterDiscovery")


def handle_phase(phase: str):
    def handler(st, state_path):
        retry = st.last_gate_error if st.gate_retries > 0 else ""
        return action("spawn", prompt=PROMPTS["phase"](phase, st, retry), phase=phase)
    return handler


def handle_designer_gate(st, state_path):
    ok, out = run_cmd(st.designer_gate, st.workspace)
    if ok:
        st.state = State.DESIGNER_STYLE.value
        st.gate_retries = 0
        return next_action(st, state_path)
    return handle_gate_failure(st, out, State.DESIGNER_PHASE, state_path)


def handle_tester_gate(st, state_path):
    if not st.commit_tests:
        st.state = State.TESTER_STYLE.value
        return next_action(st, state_path)
    ok, out = run_cmd(st.test_gate, st.workspace)
    if not ok:
        st.state = State.TESTER_STYLE.value
        st.gate_retries = 0
        return next_action(st, state_path)
    escalate(st, "Tester gate passed but should fail (TDD red). Tests may be vacuous.", state_path)


def handle_implementor_gate(st, state_path):
    ok, out = run_cmd(st.test_gate, st.workspace)
    if ok:
        st.state = State.IMPLEMENTOR_STYLE.value
        st.gate_retries = 0
        return next_action(st, state_path)
    return handle_gate_failure(st, out, State.IMPLEMENTOR_PHASE, state_path)


def handle_style(phase: str):
    def handler(st, state_path):
        if st.style_cycles >= MAX_STYLE_CYCLES:
            return action("spawn", prompt=PROMPTS["arbiter"](st), arbiter=True)
        return action("spawn", prompt=PROMPTS["style_review"](phase, st), style_review=True, phase=phase)
    return handler


def handle_scope_review(st, state_path):
    return action("spawn", prompt=PROMPTS["scope_review"](st), scope_review=True)


def handle_format(st, state_path):
    run_cmd(st.fmt_cmd, st.workspace)
    run_cmd(st.lint_cmd, st.workspace)
    st.state = State.COMMIT.value
    return next_action(st, state_path)


def handle_commit(st, state_path):
    msg = st.commit_message.replace("'", "'\''")
    ok, out = run_cmd(f"git add -A && git commit -m '{msg}'", st.workspace)
    if ok:
        st.state = State.DONE.value
        return action("done", commit_id=st.commit_id)
    escalate(st, f"Commit failed: {out}", state_path)


def handle_done(st, state_path):
    return action("done", commit_id=st.commit_id)


STATE_HANDLERS = {
    State.EXPLORE_GATES: handle_explore_gates,
    State.EXPLORE_FORMATTERS: handle_explore_formatters,
    State.DESIGNER_PHASE: handle_phase("designer"),
    State.DESIGNER_GATE: handle_designer_gate,
    State.DESIGNER_STYLE: handle_style("designer"),
    State.TESTER_PHASE: handle_phase("tester"),
    State.TESTER_GATE: handle_tester_gate,
    State.TESTER_STYLE: handle_style("tester"),
    State.IMPLEMENTOR_PHASE: handle_phase("implementor"),
    State.IMPLEMENTOR_GATE: handle_implementor_gate,
    State.IMPLEMENTOR_STYLE: handle_style("implementor"),
    State.SCOPE_REVIEW: handle_scope_review,
    State.FORMAT: handle_format,
    State.COMMIT: handle_commit,
    State.DONE: handle_done,
}


def next_action(st: CommitState, state_path: str) -> dict:
    """Return next action for orchestrator, or execute directly."""
    s = State(st.state)
    handler = STATE_HANDLERS.get(s)
    if handler:
        return handler(st, state_path)
    escalate(st, f"Unknown state: {s}", state_path)


# ═══════════════════════════════════════════════════════════════════════════════
# REPORT HANDLERS (process_report dispatch)
# ═══════════════════════════════════════════════════════════════════════════════

def report_explore_gates(st, result):
    st.designer_gate = result.get("designer_gate", "cargo check")
    st.test_gate = result.get("test_gate", "cargo test")
    st.state = State.EXPLORE_FORMATTERS.value


def report_explore_formatters(st, result):
    st.fmt_cmd = result.get("fmt_cmd", "cargo fmt")
    st.lint_cmd = result.get("lint_cmd", "cargo clippy --fix --allow-dirty --allow-staged")
    st.state = State.DESIGNER_PHASE.value


def report_phase(next_state: State):
    def handler(st, result):
        st.files_touched.extend(result.get("files_created", []))
        st.files_touched.extend(result.get("files_modified", []))
        st.state = next_state.value
    return handler


def report_style(st, result):
    if result.get("arbiter"):
        if result.get("proceed"):
            advance_from_style(st)
        else:
            st.escalation_context = f"Arbiter rejected: {result.get('reasoning')}"
            st.state = State.NEEDS_LLM.value
    elif result.get("status") == "accept":
        advance_from_style(st)
    else:
        st.last_style_violations = result.get("violations", [])
        st.style_cycles += 1


def report_scope_review(st, result):
    response = str(result.get("response", ""))
    if "VIOLATIONS" in response.upper():
        st.escalation_context = f"Scope violations: {response}"
        st.state = State.NEEDS_LLM.value
    else:
        st.state = State.FORMAT.value


REPORT_HANDLERS = {
    State.EXPLORE_GATES: report_explore_gates,
    State.EXPLORE_FORMATTERS: report_explore_formatters,
    State.DESIGNER_PHASE: report_phase(State.DESIGNER_GATE),
    State.TESTER_PHASE: report_phase(State.TESTER_GATE),
    State.IMPLEMENTOR_PHASE: report_phase(State.IMPLEMENTOR_GATE),
    State.DESIGNER_STYLE: report_style,
    State.TESTER_STYLE: report_style,
    State.IMPLEMENTOR_STYLE: report_style,
    State.SCOPE_REVIEW: report_scope_review,
}


def process_report(st: CommitState, result: dict) -> None:
    """Update state based on spawn result."""
    handler = REPORT_HANDLERS.get(State(st.state))
    if handler:
        handler(st, result)


# ═══════════════════════════════════════════════════════════════════════════════
# CLI COMMANDS
# ═══════════════════════════════════════════════════════════════════════════════

def cmd_init(args):
    plan = load_plan(args.plan)
    meta = plan["meta"]
    commits = {c["id"]: c for c in plan["commits"]}
    commit = commits[args.commit]

    st = CommitState(
        plan_path=args.plan,
        commit_id=args.commit,
        workspace=meta["workspace"],
        commit_title=commit["title"],
        commit_message=commit["message"],
        commit_files=commit["files"],
        commit_tests=commit.get("tests", []),
    )

    Path(args.state).parent.mkdir(parents=True, exist_ok=True)
    Path(args.state).write_text(st.to_json())
    print(json.dumps({"initialized": args.state}))


def cmd_next(args):
    st = CommitState.from_json(Path(args.state).read_text())
    act = next_action(st, args.state)
    Path(args.state).write_text(st.to_json())
    print(json.dumps(act))


def cmd_report(args):
    st = CommitState.from_json(Path(args.state).read_text())
    result = json.loads(args.result)
    process_report(st, result)
    Path(args.state).write_text(st.to_json())
    print(json.dumps({"state": st.state}))


def cmd_resume(args):
    st = CommitState.from_json(Path(args.state).read_text())
    if st.state == State.NEEDS_LLM.value:
        ctx = st.escalation_context.lower()
        if "designer" in ctx:
            st.state = State.DESIGNER_PHASE.value
        elif "tester" in ctx:
            st.state = State.TESTER_PHASE.value
        elif "implementor" in ctx:
            st.state = State.IMPLEMENTOR_PHASE.value
        elif "scope" in ctx:
            st.state = State.SCOPE_REVIEW.value
        else:
            st.state = State.DESIGNER_PHASE.value
        st.escalation_context = ""
        st.gate_retries = 0
    Path(args.state).write_text(st.to_json())
    cmd_next(args)


def cmd_status(args):
    st = CommitState.from_json(Path(args.state).read_text())
    print(st.to_json())


# ═══════════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════════

def main():
    p = argparse.ArgumentParser(description="implement-commit state machine driver")
    sub = p.add_subparsers(dest="cmd", required=True)

    init = sub.add_parser("init", help="Initialize commit state")
    init.add_argument("--plan", required=True, help="Path to implementation-plan.toml")
    init.add_argument("--commit", type=int, required=True, help="Commit ID to process")
    init.add_argument("--state", required=True, help="Path to state file")

    nxt = sub.add_parser("next", help="Get next action")
    nxt.add_argument("--state", required=True, help="Path to state file")

    rep = sub.add_parser("report", help="Report spawn result")
    rep.add_argument("--state", required=True, help="Path to state file")
    rep.add_argument("--result", required=True, help="JSON result from spawn")

    res = sub.add_parser("resume", help="Resume after LLM intervention")
    res.add_argument("--state", required=True, help="Path to state file")

    stat = sub.add_parser("status", help="Show current state")
    stat.add_argument("--state", required=True, help="Path to state file")

    args = p.parse_args()
    {"init": cmd_init, "next": cmd_next, "report": cmd_report, "resume": cmd_resume, "status": cmd_status}[args.cmd](args)


if __name__ == "__main__":
    main()

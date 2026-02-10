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


MAX_GATE_RETRIES = 2
MAX_STYLE_CYCLES = 3


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


# --- Prompt Templates ---

def prompt_explore_gates(st: CommitState) -> str:
    return f"""Determine build and test gate commands for this project.

Workspace: {st.workspace}
Files to be modified: {st.commit_files}

Return JSON: {{"designer_gate": "<cargo check or equivalent>", "test_gate": "<cargo test or equivalent>"}}

Gates VALIDATE code (compile, run tests) - they never modify files.
Scope commands to affected crates/packages when possible.
Use --release if needed for ARM compatibility."""


def prompt_explore_formatters(st: CommitState) -> str:
    return f"""Determine format and lint-fix commands for this project.

Workspace: {st.workspace}
Files to be modified: {st.commit_files}

Return JSON: {{"fmt_cmd": "<cargo fmt or equivalent>", "lint_cmd": "<cargo clippy --fix or equivalent>"}}

Formatters MODIFY code to fix style - they change files."""


def prompt_phase(phase: str, st: CommitState, retry_context: str = "") -> str:
    tests_info = f"Tests to write: {st.commit_tests}" if phase == "tester" and st.commit_tests else ""
    base = f"""Execute {phase} phase for commit {st.commit_id}: {st.commit_title}

Workspace: {st.workspace}
Files: {st.commit_files}
{tests_info}

Follow the {phase}.md agent instructions and style guide."""
    if retry_context:
        base += f"""

PREVIOUS ATTEMPT FAILED:
{retry_context}"""
    return base


def prompt_style_review(phase: str, st: CommitState) -> str:
    return f"""Review {phase} phase style compliance.

Files to review: {st.files_touched}
Phase: {phase}

Return JSON: {{"status": "accept" | "violations", "violations": [...]}}"""


def prompt_style_fix(phase: str, st: CommitState) -> str:
    return f"""Fix style violations for {phase} phase of commit {st.commit_id}.

Violations: {st.last_style_violations}
Files: {st.files_touched}

Fix the violations while maintaining correctness."""


def prompt_scope_review(st: CommitState) -> str:
    return f"""Review scope compliance for commit {st.commit_id}: {st.commit_title}

Allowed files: {st.commit_files}
Actual files touched: {st.files_touched}

Return ACCEPT or VIOLATIONS: [...]"""


def prompt_arbiter(st: CommitState) -> str:
    return f"""Style review exceeded {MAX_STYLE_CYCLES} cycles.

Remaining violations: {st.last_style_violations}

Decide: proceed with noted style debt, or reject?
Return JSON: {{"proceed": true|false, "reasoning": "..."}}"""


# --- State Transitions ---

def next_action(st: CommitState, state_path: str) -> dict:
    """Return next action for orchestrator, or execute directly."""
    s = State(st.state)

    if s == State.EXPLORE_GATES:
        return action("spawn", prompt=prompt_explore_gates(st), response_schema="GateDiscovery")

    if s == State.EXPLORE_FORMATTERS:
        return action("spawn", prompt=prompt_explore_formatters(st), response_schema="FormatterDiscovery")

    if s in (State.DESIGNER_PHASE, State.TESTER_PHASE, State.IMPLEMENTOR_PHASE):
        phase = s.value.replace("_phase", "")
        retry = st.last_gate_error if st.gate_retries > 0 else ""
        return action("spawn", prompt=prompt_phase(phase, st, retry), phase=phase)

    if s == State.DESIGNER_GATE:
        ok, out = run_cmd(st.designer_gate, st.workspace)
        if ok:
            st.state = State.DESIGNER_STYLE.value
            st.gate_retries = 0
            return next_action(st, state_path)
        return handle_gate_failure(st, out, State.DESIGNER_PHASE, state_path)

    if s == State.TESTER_GATE:
        # Tester gate EXPECTS failure (TDD red) - but only if there are tests
        if not st.commit_tests:
            # No tests for this commit, skip to style
            st.state = State.TESTER_STYLE.value
            return next_action(st, state_path)
        ok, out = run_cmd(st.test_gate, st.workspace)
        if not ok:
            st.state = State.TESTER_STYLE.value
            st.gate_retries = 0
            return next_action(st, state_path)
        # Tests passed = bad (vacuous tests or designer over-implemented)
        escalate(st, "Tester gate passed but should fail (TDD red). Tests may be vacuous.", state_path)

    if s == State.IMPLEMENTOR_GATE:
        ok, out = run_cmd(st.test_gate, st.workspace)
        if ok:
            st.state = State.IMPLEMENTOR_STYLE.value
            st.gate_retries = 0
            return next_action(st, state_path)
        return handle_gate_failure(st, out, State.IMPLEMENTOR_PHASE, state_path)

    if s in (State.DESIGNER_STYLE, State.TESTER_STYLE, State.IMPLEMENTOR_STYLE):
        phase = s.value.replace("_style", "")
        if st.style_cycles >= MAX_STYLE_CYCLES:
            return action("spawn", prompt=prompt_arbiter(st), arbiter=True)
        return action("spawn", prompt=prompt_style_review(phase, st), style_review=True, phase=phase)

    if s == State.SCOPE_REVIEW:
        return action("spawn", prompt=prompt_scope_review(st), scope_review=True)

    if s == State.FORMAT:
        run_cmd(st.fmt_cmd, st.workspace)
        run_cmd(st.lint_cmd, st.workspace)
        st.state = State.COMMIT.value
        return next_action(st, state_path)

    if s == State.COMMIT:
        # Escape single quotes for shell
        msg = st.commit_message.replace("'", "'\''")
        ok, out = run_cmd(f"git add -A && git commit -m '{msg}'", st.workspace)
        if ok:
            st.state = State.DONE.value
            return action("done", commit_id=st.commit_id)
        escalate(st, f"Commit failed: {out}", state_path)

    if s == State.DONE:
        return action("done", commit_id=st.commit_id)

    escalate(st, f"Unknown state: {s}", state_path)


def handle_gate_failure(st: CommitState, error: str, retry_state: State, state_path: str) -> dict:
    st.last_gate_error = error
    st.gate_retries += 1
    if st.gate_retries > MAX_GATE_RETRIES:
        escalate(st, f"Gate failed after {MAX_GATE_RETRIES} retries: {error}", state_path)
    st.state = retry_state.value
    return next_action(st, state_path)


def process_report(st: CommitState, result: dict) -> None:
    """Update state based on spawn result."""
    s = State(st.state)

    if s == State.EXPLORE_GATES:
        st.designer_gate = result.get("designer_gate", "cargo check")
        st.test_gate = result.get("test_gate", "cargo test")
        st.state = State.EXPLORE_FORMATTERS.value

    elif s == State.EXPLORE_FORMATTERS:
        st.fmt_cmd = result.get("fmt_cmd", "cargo fmt")
        st.lint_cmd = result.get("lint_cmd", "cargo clippy --fix --allow-dirty --allow-staged")
        st.state = State.DESIGNER_PHASE.value

    elif s == State.DESIGNER_PHASE:
        st.files_touched.extend(result.get("files_created", []))
        st.files_touched.extend(result.get("files_modified", []))
        st.state = State.DESIGNER_GATE.value

    elif s == State.TESTER_PHASE:
        st.files_touched.extend(result.get("files_created", []))
        st.files_touched.extend(result.get("files_modified", []))
        st.state = State.TESTER_GATE.value

    elif s == State.IMPLEMENTOR_PHASE:
        st.files_touched.extend(result.get("files_created", []))
        st.files_touched.extend(result.get("files_modified", []))
        st.state = State.IMPLEMENTOR_GATE.value

    elif s in (State.DESIGNER_STYLE, State.TESTER_STYLE, State.IMPLEMENTOR_STYLE):
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
            # Stay in same state for style fix

    elif s == State.SCOPE_REVIEW:
        response = str(result.get("response", ""))
        if "VIOLATIONS" in response.upper():
            st.escalation_context = f"Scope violations: {response}"
            st.state = State.NEEDS_LLM.value
        else:
            st.state = State.FORMAT.value


def advance_from_style(st: CommitState) -> None:
    """Move to next phase after style acceptance."""
    s = State(st.state)
    st.style_cycles = 0
    if s == State.DESIGNER_STYLE:
        st.state = State.TESTER_PHASE.value
    elif s == State.TESTER_STYLE:
        st.state = State.IMPLEMENTOR_PHASE.value
    elif s == State.IMPLEMENTOR_STYLE:
        st.state = State.SCOPE_REVIEW.value


# --- CLI ---

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
        # LLM fixed the issue, retry from appropriate state based on context
        # The orchestrator should have fixed the issue, we re-run from last phase
        if "designer" in st.escalation_context.lower():
            st.state = State.DESIGNER_PHASE.value
        elif "tester" in st.escalation_context.lower():
            st.state = State.TESTER_PHASE.value
        elif "implementor" in st.escalation_context.lower():
            st.state = State.IMPLEMENTOR_PHASE.value
        elif "scope" in st.escalation_context.lower():
            st.state = State.SCOPE_REVIEW.value
        else:
            st.state = State.DESIGNER_PHASE.value  # Default restart
        st.escalation_context = ""
        st.gate_retries = 0
    Path(args.state).write_text(st.to_json())
    cmd_next(args)


def cmd_status(args):
    st = CommitState.from_json(Path(args.state).read_text())
    print(st.to_json())


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

#!/usr/bin/env python3
"""Compose file-local JEI with one explicitly requested bounded history scope."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
from typing import Any

MAX_BUDGET = 16 * 1024 * 1024
MAX_HISTORY_LIMIT = 200
MAX_REQUIRED_SUBJECTS = 16
MAX_SUBJECT_BYTES = 1024


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description="Pilot explicit scope history beside a file-local JEI packet")
    result.add_argument("--packet-exe", required=True, type=Path)
    result.add_argument("--repo", required=True, type=Path)
    result.add_argument("--target", required=True)
    result.add_argument("--scope", required=True)
    result.add_argument("--budget", required=True, action="append", type=int)
    result.add_argument("--control-budget", required=True, type=int)
    result.add_argument("--acceptance-budget", required=True, type=int)
    result.add_argument("--history-limit", type=int, default=20)
    result.add_argument("--required-subject", required=True, action="append")
    result.add_argument("--repository-label", required=True)
    result.add_argument("--source", required=True)
    result.add_argument("--output", required=True, type=Path)
    return result


def canonical_relative(value: str, field: str, *, allow_root: bool = False) -> str:
    if allow_root and value == ".":
        return value
    if not value or "\\" in value or value.startswith("./") or value.endswith("/"):
        raise ValueError(f"{field} must be a canonical repository-relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError(f"{field} must be a canonical repository-relative path")
    return path.as_posix()


def validate_budget(value: int, field: str) -> int:
    if value <= 0 or value > MAX_BUDGET:
        raise ValueError(f"{field} must be between 1 and {MAX_BUDGET}")
    return value


def git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def resolve_inputs(repo: Path, target: str, scope: str) -> tuple[Path, Path, Path]:
    repo = repo.resolve(strict=True)
    root = Path(git(repo, "rev-parse", "--show-toplevel")).resolve(strict=True)
    if repo != root:
        raise ValueError("--repo must resolve to the Git repository root")

    target_path = (root / target).resolve(strict=True)
    scope_path = root if scope == "." else (root / scope).resolve(strict=True)
    if not target_path.is_file():
        raise ValueError("target must resolve to an existing file")
    if not scope_path.is_dir():
        raise ValueError("scope must resolve to an existing directory")
    try:
        target_path.relative_to(scope_path)
    except ValueError as error:
        raise ValueError("scope must contain the target") from error
    try:
        target_path.relative_to(root)
        scope_path.relative_to(root)
    except ValueError as error:
        raise ValueError("target and scope must stay inside the repository") from error
    return root, target_path, scope_path


def parse_history(output: str) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for line in output.splitlines():
        fields = line.split("\x1f", 2)
        if len(fields) != 3 or not all(fields):
            raise ValueError("git history emitted an invalid bounded record")
        rows.append({"sha": fields[0], "date": fields[1], "subject": fields[2]})
    return rows


def scope_history(repo: Path, scope: str, limit: int) -> tuple[list[dict[str, str]], bool]:
    output = git(
        repo,
        "-c",
        "core.quotepath=false",
        "log",
        "--no-merges",
        "--format=%H%x1f%cI%x1f%s",
        "-n",
        str(limit + 1),
        "--",
        scope,
    )
    rows = parse_history(output)
    truncated = len(rows) > limit
    return rows[:limit], truncated


def run_packet(packet_exe: Path, repo: Path, target: Path, budget: int) -> dict[str, Any]:
    env = os.environ.copy()
    env["CARGO_CULTIST_PACKET_MAX_BYTES"] = str(budget)
    completed = subprocess.run(
        [str(packet_exe), str(target)],
        cwd=repo,
        env=env,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise ValueError(f"file-local packet failed at budget {budget}")
    packet = json.loads(completed.stdout)
    if not isinstance(packet, dict):
        raise ValueError("file-local packet must decode to an object")
    if not isinstance(packet.get("recent_history"), list):
        raise ValueError("file-local packet is missing recent_history")
    return packet


def render_with_exact_size(envelope: dict[str, Any]) -> str:
    envelope["combined_serialized_bytes"] = 0
    while True:
        rendered = json.dumps(envelope, indent=2, sort_keys=True) + "\n"
        size = len(rendered.encode("utf-8"))
        if envelope["combined_serialized_bytes"] == size:
            return rendered
        envelope["combined_serialized_bytes"] = size


def compose(
    packet: dict[str, Any],
    repository: str,
    revision: str,
    target: str,
    scope: str,
    raw_scope_history: list[dict[str, str]],
    scope_truncated: bool,
    budget: int,
    history_limit: int,
    required: list[str],
    source: str,
) -> dict[str, Any]:
    target_shas = {
        row.get("sha")
        for row in packet["recent_history"]
        if isinstance(row, dict) and isinstance(row.get("sha"), str)
    }
    additional_scope_history = [row for row in raw_scope_history if row["sha"] not in target_shas]
    presence = {
        expected: any(expected in row["subject"] for row in additional_scope_history)
        for expected in required
    }
    scope_history_bytes = len(
        (json.dumps(additional_scope_history, separators=(",", ":"), sort_keys=True) + "\n").encode("utf-8")
    )
    envelope: dict[str, Any] = {
        "schema_version": 1,
        "analysis": "jei_explicit_scope_history_pilot",
        "repository": repository,
        "revision": revision,
        "target": target,
        "scope": scope,
        "source": source,
        "budget": {
            "max_serialized_bytes": budget,
            "max_scope_history_commits": history_limit,
        },
        "file_packet": packet,
        "scope_recent_history": additional_scope_history,
        "scope_history_truncated": scope_truncated,
        "scope_history_serialized_bytes": scope_history_bytes,
        "combined_serialized_bytes": 0,
        "lesson_presence": presence,
        "preserves_all_required": all(presence.values()),
        "unknowns": [
            "Explicit scope chronology does not by itself prove that a scope-history commit is semantically relevant to the target."
        ],
    }
    rendered = render_with_exact_size(envelope)
    envelope["combined_fits_budget"] = len(rendered.encode("utf-8")) <= budget
    rendered = render_with_exact_size(envelope)
    envelope["combined_fits_budget"] = len(rendered.encode("utf-8")) <= budget
    render_with_exact_size(envelope)
    return envelope


def main() -> int:
    args = parser().parse_args()
    try:
        packet_exe = args.packet_exe.resolve(strict=True)
        target = canonical_relative(args.target, "target")
        scope = canonical_relative(args.scope, "scope", allow_root=True)
        if args.history_limit <= 0 or args.history_limit > MAX_HISTORY_LIMIT:
            raise ValueError(f"history-limit must be between 1 and {MAX_HISTORY_LIMIT}")
        budgets = [validate_budget(value, "budget") for value in args.budget]
        if len(budgets) != len(set(budgets)):
            raise ValueError("budgets must be unique")
        control_budget = validate_budget(args.control_budget, "control-budget")
        acceptance_budget = validate_budget(args.acceptance_budget, "acceptance-budget")
        if control_budget not in budgets or acceptance_budget not in budgets:
            raise ValueError("control and acceptance budgets must also appear in --budget")

        required = args.required_subject
        if not required or len(required) > MAX_REQUIRED_SUBJECTS:
            raise ValueError(f"required subjects must contain 1..{MAX_REQUIRED_SUBJECTS} entries")
        if len(required) != len(set(required)):
            raise ValueError("required subjects must be unique")
        if any(not value or len(value.encode("utf-8")) > MAX_SUBJECT_BYTES for value in required):
            raise ValueError("required subject exceeds the admitted boundary")

        repo, target_path, _scope_path = resolve_inputs(args.repo, target, scope)
        revision = git(repo, "rev-parse", "HEAD")
        raw_scope_history, scope_truncated = scope_history(repo, scope, args.history_limit)

        results = []
        for budget in budgets:
            packet = run_packet(packet_exe, repo, target_path, budget)
            envelope = compose(
                packet,
                args.repository_label,
                revision,
                target,
                scope,
                raw_scope_history,
                scope_truncated,
                budget,
                args.history_limit,
                required,
                args.source,
            )
            results.append(
                {
                    "budget": budget,
                    "file_packet_serialized_bytes": packet.get("serialized_bytes"),
                    "file_packet_semantic_evictions": packet.get("semantic_evictions", []),
                    "scope_history_serialized_bytes": envelope["scope_history_serialized_bytes"],
                    "combined_serialized_bytes": envelope["combined_serialized_bytes"],
                    "combined_fits_budget": envelope["combined_fits_budget"],
                    "scope_history_truncated": envelope["scope_history_truncated"],
                    "scope_recent_history": envelope["scope_recent_history"],
                    "lesson_presence": envelope["lesson_presence"],
                    "preserves_all_required": envelope["preserves_all_required"],
                }
            )

        by_budget = {row["budget"]: row for row in results}
        control = by_budget[control_budget]
        acceptance = by_budget[acceptance_budget]
        passed = (
            control["combined_fits_budget"]
            and control["preserves_all_required"]
            and acceptance["combined_fits_budget"]
            and acceptance["preserves_all_required"]
        )
        receipt = {
            "schema_version": 1,
            "repository": args.repository_label,
            "revision": revision,
            "target": target,
            "scope": scope,
            "source": args.source,
            "required_history_subjects": required,
            "control_budget": control_budget,
            "acceptance_budget": acceptance_budget,
            "results": results,
            "acceptance": {
                "control_recovers_all_required": control["preserves_all_required"],
                "control_fits_budget": control["combined_fits_budget"],
                "acceptance_budget_recovers_all_required": acceptance["preserves_all_required"],
                "acceptance_budget_fits": acceptance["combined_fits_budget"],
                "passed": passed,
            },
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps(receipt, indent=2, sort_keys=True))
        return 0 if passed else 1
    except (OSError, ValueError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"JEI scope-history pilot error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

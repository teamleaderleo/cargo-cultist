#!/usr/bin/env python3
"""Replay one historical agent-context packet across semantic byte budgets."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
from typing import Any

MAX_BUDGET = 16 * 1024 * 1024
MAX_REQUIRED_SUBJECTS = 16
MAX_SUBJECT_BYTES = 1024


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description="Replay a JEI history packet across byte budgets")
    result.add_argument("--packet-exe", required=True, type=Path)
    result.add_argument("--repo", required=True, type=Path)
    result.add_argument("--target", required=True)
    result.add_argument("--budget", required=True, action="append", type=int)
    result.add_argument("--control-budget", required=True, type=int)
    result.add_argument("--acceptance-budget", required=True, type=int)
    result.add_argument("--required-subject", required=True, action="append")
    result.add_argument("--repository-label", required=True)
    result.add_argument("--source", required=True)
    result.add_argument("--output", required=True, type=Path)
    return result


def canonical_target(value: str) -> str:
    if not value or "\\" in value or value.startswith("./"):
        raise ValueError("target must be a canonical repository-relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError("target must be a canonical repository-relative path")
    return path.as_posix()


def validate_budget(value: int, field: str) -> int:
    if value <= 0 or value > MAX_BUDGET:
        raise ValueError(f"{field} must be between 1 and {MAX_BUDGET}")
    return value


def git_head(repo: Path) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def run_packet(packet_exe: Path, repo: Path, target: str, budget: int, required: list[str]) -> dict[str, Any]:
    env = os.environ.copy()
    env["CARGO_CULTIST_PACKET_MAX_BYTES"] = str(budget)
    completed = subprocess.run(
        [str(packet_exe), str(repo / target)],
        cwd=repo,
        env=env,
        capture_output=True,
        text=True,
    )

    if completed.returncode != 0:
        stderr = completed.stderr
        if "protected packet evidence requires" in stderr:
            return {
                "budget": budget,
                "status": "protected_core_too_large",
                "preserves_all_required": False,
                "lesson_presence": {subject: False for subject in required},
            }
        return {
            "budget": budget,
            "status": "packet_error",
            "returncode": completed.returncode,
            "preserves_all_required": False,
            "lesson_presence": {subject: False for subject in required},
        }

    packet = json.loads(completed.stdout)
    if not isinstance(packet, dict):
        raise ValueError("agent context packet must decode to an object")
    history = packet.get("recent_history")
    if not isinstance(history, list):
        raise ValueError("agent context packet is missing recent_history")
    subjects = []
    for row in history:
        if not isinstance(row, dict) or not isinstance(row.get("subject"), str):
            raise ValueError("recent_history contains an invalid row")
        subjects.append(row["subject"])

    presence = {
        expected: any(expected in observed for observed in subjects)
        for expected in required
    }
    return {
        "budget": budget,
        "status": "success",
        "candidate_serialized_bytes": packet.get("candidate_serialized_bytes"),
        "serialized_bytes": packet.get("serialized_bytes"),
        "semantic_evictions": packet.get("semantic_evictions", []),
        "recent_history_subjects": subjects,
        "lesson_presence": presence,
        "preserves_all_required": all(presence.values()),
    }


def main() -> int:
    args = parser().parse_args()
    try:
        packet_exe = args.packet_exe.resolve(strict=True)
        repo = args.repo.resolve(strict=True)
        target = canonical_target(args.target)
        if not (repo / target).is_file():
            raise ValueError(f"target does not exist in pinned repository: {target}")

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
        if any(not value or len(value.encode("utf-8")) > MAX_SUBJECT_BYTES for value in required):
            raise ValueError("required subject exceeds the admitted boundary")
        if len(required) != len(set(required)):
            raise ValueError("required subjects must be unique")

        revision = git_head(repo)
        results = [run_packet(packet_exe, repo, target, budget, required) for budget in budgets]
        by_budget = {result["budget"]: result for result in results}
        control_ok = by_budget[control_budget]["status"] == "success" and by_budget[control_budget]["preserves_all_required"]
        acceptance_ok = by_budget[acceptance_budget]["status"] == "success" and by_budget[acceptance_budget]["preserves_all_required"]
        unexpected_errors = [result["budget"] for result in results if result["status"] == "packet_error"]

        preserving = [result["budget"] for result in results if result["status"] == "success" and result["preserves_all_required"]]
        below_acceptance = [result for result in results if result["budget"] < acceptance_budget]
        first_loss = next(
            (
                result["budget"]
                for result in below_acceptance
                if result["status"] != "success" or not result["preserves_all_required"]
            ),
            None,
        )

        receipt = {
            "schema_version": 1,
            "repository": args.repository_label,
            "revision": revision,
            "target": target,
            "source": args.source,
            "required_history_subjects": required,
            "control_budget": control_budget,
            "acceptance_budget": acceptance_budget,
            "minimum_tested_preserving_budget": min(preserving) if preserving else None,
            "first_tested_loss_below_acceptance": first_loss,
            "results": results,
            "acceptance": {
                "control_preserves_required_history": control_ok,
                "acceptance_budget_preserves_required_history": acceptance_ok,
                "unexpected_packet_error_budgets": unexpected_errors,
                "passed": control_ok and acceptance_ok and not unexpected_errors,
            },
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps(receipt, indent=2, sort_keys=True))
        return 0 if receipt["acceptance"]["passed"] else 1
    except (OSError, ValueError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"JEI history pilot error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

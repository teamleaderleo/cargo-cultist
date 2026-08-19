#!/usr/bin/env python3
"""Measure whether automatic target-parent scope adds required JEI evidence."""

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
EXPECTATIONS = {"adds_missing", "already_sufficient"}


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description="Compare file-local and parent-scoped JEI packets")
    result.add_argument("--packet-exe", required=True, type=Path)
    result.add_argument("--scoped-exe", required=True, type=Path)
    result.add_argument("--repo", required=True, type=Path)
    result.add_argument("--target", required=True)
    result.add_argument("--budget", required=True, type=int)
    result.add_argument("--required-subject", required=True, action="append")
    result.add_argument("--expectation", required=True, choices=sorted(EXPECTATIONS))
    result.add_argument("--repository-label", required=True)
    result.add_argument("--source", required=True)
    result.add_argument("--output", required=True, type=Path)
    return result


def canonical_target(value: str) -> str:
    if not value or "\\" in value or value.startswith("./") or value.endswith("/"):
        raise ValueError("target must be a canonical repository-relative file path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError("target must be a canonical repository-relative file path")
    if len(path.parts) < 2:
        raise ValueError("target must have a repository-relative parent directory")
    return path.as_posix()


def validate_budget(value: int) -> int:
    if value <= 0 or value > MAX_BUDGET:
        raise ValueError(f"budget must be between 1 and {MAX_BUDGET}")
    return value


def git_head(repo: Path) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def run_json(executable: Path, args: list[str], repo: Path, budget: int) -> dict[str, Any]:
    env = os.environ.copy()
    env["CARGO_CULTIST_PACKET_MAX_BYTES"] = str(budget)
    completed = subprocess.run(
        [str(executable), *args],
        cwd=repo,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    decoded = json.loads(completed.stdout)
    if not isinstance(decoded, dict):
        raise ValueError("packet executable must emit one JSON object")
    return decoded


def subjects(rows: Any, field: str) -> list[str]:
    if not isinstance(rows, list):
        raise ValueError(f"{field} must be an array")
    output = []
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("subject"), str):
            raise ValueError(f"{field} contains an invalid history row")
        output.append(row["subject"])
    return output


def presence(required: list[str], observed: list[str]) -> dict[str, bool]:
    return {
        expected: any(expected in subject for subject in observed)
        for expected in required
    }


def main() -> int:
    args = parser().parse_args()
    try:
        packet_exe = args.packet_exe.resolve(strict=True)
        scoped_exe = args.scoped_exe.resolve(strict=True)
        repo = args.repo.resolve(strict=True)
        target = canonical_target(args.target)
        target_path = (repo / target).resolve(strict=True)
        if not target_path.is_file():
            raise ValueError("target must resolve to an existing file")
        try:
            target_path.relative_to(repo)
        except ValueError as error:
            raise ValueError("target must stay inside the repository") from error

        parent = PurePosixPath(target).parent.as_posix()
        budget = validate_budget(args.budget)
        required = args.required_subject
        if not required or len(required) > MAX_REQUIRED_SUBJECTS:
            raise ValueError(f"required subjects must contain 1..{MAX_REQUIRED_SUBJECTS} entries")
        if len(required) != len(set(required)):
            raise ValueError("required subjects must be unique")
        if any(not item or len(item.encode("utf-8")) > MAX_SUBJECT_BYTES for item in required):
            raise ValueError("required subject exceeds the admitted boundary")

        file_packet = run_json(packet_exe, [str(target_path)], repo, budget)
        scoped_packet = run_json(scoped_exe, [str(target_path), "--scope", parent], repo, budget)

        file_subjects = subjects(file_packet.get("recent_history"), "recent_history")
        nested_subjects = subjects(
            scoped_packet.get("file_packet", {}).get("recent_history")
            if isinstance(scoped_packet.get("file_packet"), dict)
            else None,
            "file_packet.recent_history",
        )
        scope_subjects = subjects(scoped_packet.get("scope_recent_history"), "scope_recent_history")
        if file_subjects != nested_subjects:
            raise ValueError("scoped packet changed target-local recent history before scope comparison")

        target_presence = presence(required, file_subjects)
        scope_presence = presence(required, scope_subjects)
        novel_presence = {
            item: (not target_presence[item]) and scope_presence[item]
            for item in required
        }

        file_bytes = file_packet.get("serialized_bytes")
        scoped_bytes = scoped_packet.get("serialized_bytes")
        candidate_bytes = scoped_packet.get("candidate_serialized_bytes")
        if not all(isinstance(value, int) for value in [file_bytes, scoped_bytes, candidate_bytes]):
            raise ValueError("packets are missing byte measurements")
        if scoped_bytes > budget:
            raise ValueError("scoped packet exceeded the declared budget")

        if args.expectation == "adds_missing":
            expectation_passed = not any(target_presence.values()) and all(novel_presence.values())
        elif args.expectation == "already_sufficient":
            expectation_passed = all(target_presence.values()) and not any(novel_presence.values())
        else:
            raise ValueError(f"unsupported expectation: {args.expectation}")

        receipt = {
            "schema_version": 1,
            "repository": args.repository_label,
            "revision": git_head(repo),
            "target": target,
            "inferred_parent_scope": parent,
            "source": args.source,
            "expectation": args.expectation,
            "budget": budget,
            "required_history_subjects": required,
            "file_packet_serialized_bytes": file_bytes,
            "scoped_candidate_serialized_bytes": candidate_bytes,
            "scoped_selected_serialized_bytes": scoped_bytes,
            "selected_byte_overhead": scoped_bytes - file_bytes,
            "scope_history_count": len(scope_subjects),
            "scope_history_truncated": scoped_packet.get("scope_history_truncated"),
            "scope_semantic_evictions": scoped_packet.get("semantic_evictions", []),
            "file_packet_semantic_evictions": scoped_packet.get("file_packet", {}).get("semantic_evictions", []),
            "target_history_presence": target_presence,
            "scope_history_presence": scope_presence,
            "novel_required_presence": novel_presence,
            "novel_required_count": sum(novel_presence.values()),
            "acceptance": {
                "expectation_passed": expectation_passed,
                "scoped_packet_fits_budget": scoped_bytes <= budget,
                "passed": expectation_passed and scoped_bytes <= budget,
            },
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps(receipt, indent=2, sort_keys=True))
        return 0 if receipt["acceptance"]["passed"] else 1
    except (OSError, ValueError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"JEI parent-scope pilot error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

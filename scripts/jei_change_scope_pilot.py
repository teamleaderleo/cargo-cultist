#!/usr/bin/env python3
"""Pilot deterministic scope expansion from an explicit changed-path set."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
from typing import Any

MAX_BUDGET = 16 * 1024 * 1024
MAX_TASK_PATHS = 64
MAX_REQUIRED_SUBJECTS = 16
MAX_SUBJECT_BYTES = 1024
EXPECTATIONS = {"expand_adds_missing", "single_path_file_local"}


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description="Pilot change-set-driven JEI scope expansion")
    result.add_argument("--packet-exe", required=True, type=Path)
    result.add_argument("--scoped-exe", required=True, type=Path)
    result.add_argument("--repo", required=True, type=Path)
    result.add_argument("--primary-target", required=True)
    result.add_argument("--task-path", required=True, action="append")
    result.add_argument("--budget", required=True, type=int)
    result.add_argument("--required-subject", required=True, action="append")
    result.add_argument("--expectation", required=True, choices=sorted(EXPECTATIONS))
    result.add_argument("--repository-label", required=True)
    result.add_argument("--source", required=True)
    result.add_argument("--output", required=True, type=Path)
    return result


def canonical_path(value: str, field: str) -> str:
    if not value or "\\" in value or value.startswith("./") or value.endswith("/"):
        raise ValueError(f"{field} must be a canonical repository-relative file path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError(f"{field} must be a canonical repository-relative file path")
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


def common_directory(paths: list[str]) -> str | None:
    if len(paths) < 2:
        return None
    parent_parts = [PurePosixPath(path).parent.parts for path in paths]
    shared: list[str] = []
    for components in zip(*parent_parts):
        if len(set(components)) != 1:
            break
        shared.append(components[0])
    if not shared:
        return None
    scope = PurePosixPath(*shared).as_posix()
    return None if scope == "." else scope


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


def history_subjects(rows: Any, field: str) -> list[str]:
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
        budget = validate_budget(args.budget)
        primary = canonical_path(args.primary_target, "primary-target")

        task_paths = [canonical_path(value, "task-path") for value in args.task_path]
        if not task_paths or len(task_paths) > MAX_TASK_PATHS:
            raise ValueError(f"task paths must contain 1..{MAX_TASK_PATHS} entries")
        if len(task_paths) != len(set(task_paths)):
            raise ValueError("task paths must be unique")
        if primary not in task_paths:
            raise ValueError("primary target must be included in task paths")

        for value in task_paths:
            candidate = (repo / value).resolve(strict=True)
            if not candidate.is_file():
                raise ValueError(f"task path is not an existing file: {value}")
            try:
                candidate.relative_to(repo)
            except ValueError as error:
                raise ValueError(f"task path escapes repository: {value}") from error

        required = args.required_subject
        if not required or len(required) > MAX_REQUIRED_SUBJECTS:
            raise ValueError(f"required subjects must contain 1..{MAX_REQUIRED_SUBJECTS} entries")
        if len(required) != len(set(required)):
            raise ValueError("required subjects must be unique")
        if any(not item or len(item.encode("utf-8")) > MAX_SUBJECT_BYTES for item in required):
            raise ValueError("required subject exceeds admitted boundary")

        primary_path = (repo / primary).resolve(strict=True)
        file_packet = run_json(packet_exe, [str(primary_path)], repo, budget)
        file_subjects = history_subjects(file_packet.get("recent_history"), "recent_history")
        target_presence = presence(required, file_subjects)

        scope = common_directory(task_paths)
        if len(task_paths) == 1:
            decision = "file_local"
        elif scope is None:
            decision = "unsupported"
        else:
            decision = "explicit_common_scope"

        scope_subjects: list[str] = []
        scoped_packet: dict[str, Any] | None = None
        scope_presence = {item: False for item in required}
        novel_presence = {item: False for item in required}
        if decision == "explicit_common_scope":
            scoped_packet = run_json(scoped_exe, [str(primary_path), "--scope", scope], repo, budget)
            nested = scoped_packet.get("file_packet")
            if not isinstance(nested, dict):
                raise ValueError("scoped packet is missing nested file packet")
            nested_subjects = history_subjects(nested.get("recent_history"), "file_packet.recent_history")
            if nested_subjects != file_subjects:
                raise ValueError("scoped packet changed target-local recent history")
            scope_subjects = history_subjects(scoped_packet.get("scope_recent_history"), "scope_recent_history")
            scope_presence = presence(required, scope_subjects)
            novel_presence = {
                item: (not target_presence[item]) and scope_presence[item]
                for item in required
            }

        if args.expectation == "expand_adds_missing":
            expectation_passed = (
                decision == "explicit_common_scope"
                and not any(target_presence.values())
                and all(novel_presence.values())
            )
        else:
            expectation_passed = decision == "file_local" and all(target_presence.values())

        scoped_bytes = scoped_packet.get("serialized_bytes") if scoped_packet else None
        file_bytes = file_packet.get("serialized_bytes")
        if not isinstance(file_bytes, int) or (scoped_packet and not isinstance(scoped_bytes, int)):
            raise ValueError("packet byte measurement missing")

        receipt = {
            "schema_version": 1,
            "repository": args.repository_label,
            "revision": git_head(repo),
            "primary_target": primary,
            "task_paths": task_paths,
            "source": args.source,
            "budget": budget,
            "decision": decision,
            "common_scope": scope,
            "required_history_subjects": required,
            "file_packet_serialized_bytes": file_bytes,
            "scoped_packet_serialized_bytes": scoped_bytes,
            "selected_byte_overhead": (scoped_bytes - file_bytes) if isinstance(scoped_bytes, int) else 0,
            "scope_history_count": len(scope_subjects),
            "scope_history_truncated": scoped_packet.get("scope_history_truncated") if scoped_packet else None,
            "scope_semantic_evictions": scoped_packet.get("semantic_evictions", []) if scoped_packet else [],
            "target_history_presence": target_presence,
            "scope_history_presence": scope_presence,
            "novel_required_presence": novel_presence,
            "novel_required_count": sum(novel_presence.values()),
            "acceptance": {
                "expectation": args.expectation,
                "expectation_passed": expectation_passed,
                "passed": expectation_passed,
            },
        }
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps(receipt, indent=2, sort_keys=True))
        return 0 if expectation_passed else 1
    except (OSError, ValueError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"JEI change-scope pilot error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

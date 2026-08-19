#!/usr/bin/env python3
"""Resolve strict pinned external-dogfood cases for local or CI replay."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any

MAX_REGISTRY_BYTES = 1024 * 1024
MAX_CASES = 128
MAX_HISTORY_COMMITS = 1000
MAX_FETCH_DEPTH = 100000
CASE_ID_RE = re.compile(r"^[a-z0-9][a-z0-9-]{0,79}$")
REPOSITORY_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
HEX40_RE = re.compile(r"^[0-9a-f]{40}$")
STATUSES = {"replayable", "adapter_gap", "needs_pin"}
CASE_KEYS = {
    "id",
    "status",
    "repository",
    "ref",
    "fetch_depth",
    "history_file",
    "history_max",
    "base",
    "repeat_scan",
    "source",
    "question",
    "reason",
}


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description="Resolve a pinned external dogfood case")
    result.add_argument(
        "--registry",
        type=Path,
        default=Path("research/external-dogfood-cases.json"),
    )
    action = result.add_mutually_exclusive_group(required=True)
    action.add_argument("--list", action="store_true")
    action.add_argument("--case")
    result.add_argument(
        "--github-output",
        type=Path,
        help="write replayable case fields using GitHub Actions output syntax",
    )
    return result


def canonical_relative_path(value: object, field: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise ValueError(f"{field} must be null or a non-empty string")
    normalized = value.replace("\\", "/")
    path = Path(normalized)
    if path.is_absolute() or ".." in path.parts or normalized.startswith("./"):
        raise ValueError(f"{field} must be a canonical repository-relative path")
    if path.as_posix() != normalized:
        raise ValueError(f"{field} must use canonical `/` separators")
    return normalized


def bounded_string(value: object, field: str, limit: int = 4096) -> str:
    if not isinstance(value, str) or not value or len(value) > limit:
        raise ValueError(f"{field} must be a non-empty string up to {limit} characters")
    if "\n" in value or "\r" in value:
        raise ValueError(f"{field} must be single-line")
    return value


def validate_case(raw: object) -> dict[str, Any]:
    if not isinstance(raw, dict):
        raise ValueError("each case must be a JSON object")
    unknown = set(raw) - CASE_KEYS
    if unknown:
        raise ValueError(f"unknown case field(s): {', '.join(sorted(unknown))}")

    case_id = bounded_string(raw.get("id"), "id", 80)
    if CASE_ID_RE.fullmatch(case_id) is None:
        raise ValueError(f"invalid case id: {case_id}")

    status = bounded_string(raw.get("status"), f"{case_id}.status", 32)
    if status not in STATUSES:
        raise ValueError(f"unsupported status for {case_id}: {status}")

    repository = bounded_string(raw.get("repository"), f"{case_id}.repository", 200)
    if REPOSITORY_RE.fullmatch(repository) is None:
        raise ValueError(f"invalid repository for {case_id}: {repository}")

    ref = raw.get("ref")
    if ref is not None:
        ref = bounded_string(ref, f"{case_id}.ref", 200)
        if HEX40_RE.fullmatch(ref) is None:
            raise ValueError(f"registry ref for {case_id} must be an exact 40-hex commit")

    base = raw.get("base")
    if base is not None:
        base = bounded_string(base, f"{case_id}.base", 200)
        if HEX40_RE.fullmatch(base) is None:
            raise ValueError(f"registry base for {case_id} must be an exact 40-hex commit")

    fetch_depth = raw.get("fetch_depth")
    if not isinstance(fetch_depth, int) or isinstance(fetch_depth, bool):
        raise ValueError(f"{case_id}.fetch_depth must be an integer")
    if fetch_depth < 0 or fetch_depth > MAX_FETCH_DEPTH:
        raise ValueError(f"{case_id}.fetch_depth is outside the admitted range")

    history_max = raw.get("history_max")
    if not isinstance(history_max, int) or isinstance(history_max, bool):
        raise ValueError(f"{case_id}.history_max must be an integer")
    if history_max < 1 or history_max > MAX_HISTORY_COMMITS:
        raise ValueError(f"{case_id}.history_max is outside the admitted range")

    repeat_scan = raw.get("repeat_scan")
    if not isinstance(repeat_scan, bool):
        raise ValueError(f"{case_id}.repeat_scan must be boolean")

    history_file = canonical_relative_path(raw.get("history_file"), f"{case_id}.history_file")
    source = bounded_string(raw.get("source"), f"{case_id}.source", 512)
    question = bounded_string(raw.get("question"), f"{case_id}.question", 4096)

    reason = raw.get("reason")
    if reason is not None:
        reason = bounded_string(reason, f"{case_id}.reason", 4096)

    if status == "replayable" and ref is None:
        raise ValueError(f"replayable case {case_id} requires an exact ref")
    if status != "replayable" and reason is None:
        raise ValueError(f"non-replayable case {case_id} requires a reason")

    return {
        "id": case_id,
        "status": status,
        "repository": repository,
        "ref": ref,
        "fetch_depth": fetch_depth,
        "history_file": history_file,
        "history_max": history_max,
        "base": base,
        "repeat_scan": repeat_scan,
        "source": source,
        "question": question,
        "reason": reason,
    }


def load_registry(path: Path) -> list[dict[str, Any]]:
    data = path.read_bytes()
    if len(data) > MAX_REGISTRY_BYTES:
        raise ValueError("external dogfood registry exceeds 1 MiB")
    parsed = json.loads(data)
    if not isinstance(parsed, dict) or set(parsed) != {"schema_version", "cases"}:
        raise ValueError("registry must contain exactly schema_version and cases")
    if parsed.get("schema_version") != 1:
        raise ValueError("unsupported external dogfood registry schema")
    raw_cases = parsed.get("cases")
    if not isinstance(raw_cases, list) or len(raw_cases) > MAX_CASES:
        raise ValueError(f"registry cases must be a list of at most {MAX_CASES} entries")

    cases = [validate_case(raw) for raw in raw_cases]
    ids = [case["id"] for case in cases]
    if len(ids) != len(set(ids)):
        raise ValueError("external dogfood case ids must be unique")
    return cases


def write_github_output(path: Path, case: dict[str, Any]) -> None:
    fields = {
        "case_id": case["id"],
        "repository": case["repository"],
        "ref": case["ref"] or "",
        "fetch_depth": str(case["fetch_depth"]),
        "history_file": case["history_file"] or "",
        "history_max": str(case["history_max"]),
        "base": case["base"] or "",
        "repeat_scan": "true" if case["repeat_scan"] else "false",
        "source": case["source"],
        "question": case["question"],
    }
    with path.open("a", encoding="utf-8") as output:
        for key, value in fields.items():
            if "\n" in value or "\r" in value:
                raise ValueError(f"GitHub output field {key} contains a newline")
            output.write(f"{key}={value}\n")


def main() -> int:
    args = parser().parse_args()
    try:
        cases = load_registry(args.registry)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"external dogfood registry error: {error}", file=sys.stderr)
        return 1

    if args.list:
        print(json.dumps(cases, indent=2, sort_keys=True))
        return 0

    case = next((case for case in cases if case["id"] == args.case), None)
    if case is None:
        print(f"unknown external dogfood case: {args.case}", file=sys.stderr)
        return 1
    if case["status"] != "replayable":
        print(
            f"case {case['id']} is {case['status']}: {case['reason']}",
            file=sys.stderr,
        )
        return 2

    if args.github_output is not None:
        try:
            write_github_output(args.github_output, case)
        except (OSError, ValueError) as error:
            print(f"could not write GitHub outputs: {error}", file=sys.stderr)
            return 1

    print(json.dumps(case, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

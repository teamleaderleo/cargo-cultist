#!/usr/bin/env python3
"""Replay pinned real-repository Cargo Cultist cases.

This harness is intentionally outside required CI. It uses network access to clone
public repositories, runs the local cargo-cultist binary, and checks semantic JSON
facts instead of golden terminal output.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CASES = ROOT / "fieldwork" / "cases.json"


class FieldworkFailure(RuntimeError):
    pass


def command(args: list[str], *, cwd: Path | None = None, capture: bool = False) -> str:
    result = subprocess.run(
        args,
        cwd=cwd,
        check=False,
        text=True,
        capture_output=capture,
    )
    if result.returncode != 0:
        stdout = result.stdout.strip() if capture else ""
        stderr = result.stderr.strip() if capture else ""
        detail = "\n".join(part for part in [stdout, stderr] if part)
        suffix = f"\n{detail}" if detail else ""
        raise FieldworkFailure(f"command failed ({result.returncode}): {' '.join(args)}{suffix}")
    return result.stdout if capture else ""


def load_cases(path: Path) -> list[dict[str, Any]]:
    payload = json.loads(path.read_text())
    if payload.get("schema_version") != 1:
        raise FieldworkFailure(f"unsupported Fieldwork schema: {payload.get('schema_version')!r}")
    cases = payload.get("cases")
    if not isinstance(cases, list):
        raise FieldworkFailure("Fieldwork file must contain a `cases` array")
    return cases


def build_binary() -> Path:
    command(["cargo", "build", "--quiet"], cwd=ROOT)
    suffix = ".exe" if os.name == "nt" else ""
    binary = ROOT / "target" / "debug" / f"cargo-cultist{suffix}"
    if not binary.is_file():
        raise FieldworkFailure(f"cargo build did not produce {binary}")
    return binary.resolve()


def prepare_repository(
    case: dict[str, Any],
    workdir: Path,
    cache: dict[tuple[str, str], Path],
) -> Path:
    repository = require_string(case, "repository")
    checkout = require_string(case, "checkout")
    key = (repository, checkout)
    if key in cache:
        return cache[key]

    destination = workdir / f"repo-{len(cache) + 1}"
    if destination.exists():
        shutil.rmtree(destination)

    command(
        [
            "git",
            "clone",
            "--quiet",
            "--filter=blob:none",
            "--no-checkout",
            repository,
            str(destination),
        ]
    )

    refs = [checkout]
    extra_refs = case.get("extra_refs", [])
    if not isinstance(extra_refs, list) or not all(isinstance(ref, str) for ref in extra_refs):
        raise FieldworkFailure(f"{case_id(case)}: `extra_refs` must be an array of strings")
    refs.extend(extra_refs)

    command(["git", "-C", str(destination), "fetch", "--quiet", "origin", *refs])
    command(["git", "-C", str(destination), "checkout", "--quiet", "--detach", checkout])
    cache[key] = destination
    return destination


def run_case(binary: Path, case: dict[str, Any], repository: Path) -> None:
    raw_command = case.get("command")
    if not isinstance(raw_command, list) or not all(isinstance(arg, str) for arg in raw_command):
        raise FieldworkFailure(f"{case_id(case)}: `command` must be an array of strings")

    stdout = command([str(binary), *raw_command], cwd=repository, capture=True)
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError as error:
        raise FieldworkFailure(
            f"{case_id(case)}: cargo-cultist did not return JSON: {error}\n{stdout}"
        ) from error

    check_expectations(case, payload)


def check_expectations(case: dict[str, Any], payload: dict[str, Any]) -> None:
    expected = case.get("expect")
    if not isinstance(expected, dict):
        raise FieldworkFailure(f"{case_id(case)}: missing `expect` object")

    analysis = expected.get("analysis")
    if analysis is not None and payload.get("analysis") != analysis:
        fail(case, f"expected analysis {analysis!r}, got {payload.get('analysis')!r}")

    required_kinds = expected.get("finding_kinds", [])
    if required_kinds:
        if not isinstance(required_kinds, list) or not all(
            isinstance(kind, str) for kind in required_kinds
        ):
            fail(case, "`finding_kinds` must be an array of strings")
        actual_kinds = {
            finding.get("kind")
            for finding in payload.get("findings", [])
            if isinstance(finding, dict)
        }
        missing = [kind for kind in required_kinds if kind not in actual_kinds]
        if missing:
            fail(case, f"missing finding kind(s): {', '.join(missing)}; got {sorted(actual_kinds)}")

    required_strings = expected.get("contains", [])
    if required_strings:
        if not isinstance(required_strings, list) or not all(
            isinstance(value, str) for value in required_strings
        ):
            fail(case, "`contains` must be an array of strings")
        all_strings = list(flatten_strings(payload))
        for needle in required_strings:
            if not any(needle in value for value in all_strings):
                fail(case, f"expected JSON evidence containing: {needle!r}")

    companion_expectations = expected.get("companions", [])
    if companion_expectations:
        actual_companions = {
            companion.get("path"): companion
            for companion in payload.get("companions", [])
            if isinstance(companion, dict) and isinstance(companion.get("path"), str)
        }
        for companion_expectation in companion_expectations:
            if not isinstance(companion_expectation, dict):
                fail(case, "companion expectations must be objects")
            path = companion_expectation.get("path")
            if not isinstance(path, str):
                fail(case, "companion expectation is missing string `path`")
            actual = actual_companions.get(path)
            if actual is None:
                fail(case, f"expected companion {path!r} was not reported")
            check_numeric_bound(case, path, actual, companion_expectation, "support", "min_support")
            check_numeric_bound(case, path, actual, companion_expectation, "support", "max_support")
            if "opportunities" in companion_expectation:
                wanted = companion_expectation["opportunities"]
                if actual.get("opportunities") != wanted:
                    fail(
                        case,
                        f"{path}: expected opportunities={wanted}, got {actual.get('opportunities')}",
                    )


def check_numeric_bound(
    case: dict[str, Any],
    path: str,
    actual: dict[str, Any],
    expected: dict[str, Any],
    actual_key: str,
    expected_key: str,
) -> None:
    if expected_key not in expected:
        return
    wanted = expected[expected_key]
    value = actual.get(actual_key)
    if not isinstance(wanted, (int, float)) or not isinstance(value, (int, float)):
        fail(case, f"{path}: numeric assertion {expected_key} has non-numeric data")

    if expected_key.startswith("min_") and value < wanted:
        fail(case, f"{path}: expected {actual_key}>={wanted}, got {value}")
    if expected_key.startswith("max_") and value > wanted:
        fail(case, f"{path}: expected {actual_key}<={wanted}, got {value}")


def flatten_strings(value: Any):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for nested in value.values():
            yield from flatten_strings(nested)
    elif isinstance(value, list):
        for nested in value:
            yield from flatten_strings(nested)


def require_string(case: dict[str, Any], key: str) -> str:
    value = case.get(key)
    if not isinstance(value, str):
        raise FieldworkFailure(f"{case_id(case)}: `{key}` must be a string")
    return value


def case_id(case: dict[str, Any]) -> str:
    value = case.get("id")
    return value if isinstance(value, str) else "<unnamed case>"


def fail(case: dict[str, Any], message: str) -> None:
    raise FieldworkFailure(f"{case_id(case)}: {message}")


def select_cases(cases: list[dict[str, Any]], requested: list[str]) -> list[dict[str, Any]]:
    if not requested:
        return cases
    by_id = {case_id(case): case for case in cases}
    unknown = [identifier for identifier in requested if identifier not in by_id]
    if unknown:
        raise FieldworkFailure(f"unknown Fieldwork case(s): {', '.join(unknown)}")
    return [by_id[identifier] for identifier in requested]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", type=Path, default=DEFAULT_CASES, help="Fieldwork case file")
    parser.add_argument("--case", action="append", default=[], help="run only this case (repeatable)")
    parser.add_argument("--list", action="store_true", help="list case IDs and exit")
    parser.add_argument(
        "--workdir",
        type=Path,
        help="persistent directory for cloned repositories (temporary by default)",
    )
    parser.add_argument("--binary", type=Path, help="use an existing cargo-cultist binary")
    args = parser.parse_args()

    try:
        cases = load_cases(args.cases)
        if args.list:
            for case in cases:
                print(f"{case_id(case)}\t{case.get('description', '')}")
            return 0

        selected = select_cases(cases, args.case)
        binary = args.binary.resolve() if args.binary else build_binary()
        if not binary.is_file():
            raise FieldworkFailure(f"cargo-cultist binary does not exist: {binary}")

        temporary = None
        if args.workdir:
            workdir = args.workdir.resolve()
            workdir.mkdir(parents=True, exist_ok=True)
        else:
            temporary = tempfile.TemporaryDirectory(prefix="cargo-cultist-fieldwork-")
            workdir = Path(temporary.name)

        cache: dict[tuple[str, str], Path] = {}
        failures: list[str] = []
        try:
            for case in selected:
                identifier = case_id(case)
                print(f"FIELDWORK {identifier}")
                print(f"  {case.get('description', '')}")
                try:
                    repository = prepare_repository(case, workdir, cache)
                    run_case(binary, case, repository)
                except (FieldworkFailure, OSError) as error:
                    failures.append(str(error))
                    print(f"  FAIL: {error}")
                else:
                    print("  PASS")
        finally:
            if temporary is not None:
                temporary.cleanup()

        if failures:
            print(f"\n{len(failures)} of {len(selected)} Fieldwork case(s) failed", file=sys.stderr)
            return 1

        print(f"\n{len(selected)} Fieldwork case(s) passed")
        return 0
    except (FieldworkFailure, OSError, json.JSONDecodeError) as error:
        print(f"fieldwork: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

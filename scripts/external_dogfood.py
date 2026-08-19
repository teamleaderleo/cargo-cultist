#!/usr/bin/env python3
"""Run bounded Cultist probes against a read-only repository checkout."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
from typing import Any

PERF_PREFIX = "CULTIST_PERF "
MAX_HISTORY_COMMITS = 1000
MAX_BASE_LENGTH = 256


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Run Cultist against another repository without executing that "
            "repository's build or test commands."
        )
    )
    result.add_argument("--cultist", required=True, type=Path)
    result.add_argument("--repo", required=True, type=Path)
    result.add_argument("--output-dir", required=True, type=Path)
    result.add_argument("--cache-dir", type=Path)
    result.add_argument(
        "--repeat-scan",
        action="store_true",
        help="run a second repository scan to measure warm reuse",
    )
    result.add_argument(
        "--history-file",
        help="repository-relative file to inspect with the bounded history probe",
    )
    result.add_argument(
        "--history-max",
        type=int,
        default=100,
        help=f"history commit bound (1..{MAX_HISTORY_COMMITS}; default 100)",
    )
    result.add_argument(
        "--base",
        help="optional Git revision for one diff probe against the checked-out target",
    )
    result.add_argument(
        "--checkout-depth",
        type=int,
        default=0,
        help="record the caller's checkout depth; 0 means full history",
    )
    return result


def canonical_history_file(repo: Path, raw: str | None) -> str | None:
    if raw is None or not raw.strip():
        return None
    value = raw.strip().replace("\\", "/")
    candidate = Path(value)
    if candidate.is_absolute() or ".." in candidate.parts:
        raise ValueError("history file must be a canonical repository-relative path")
    resolved = (repo / candidate).resolve()
    try:
        relative = resolved.relative_to(repo)
    except ValueError as error:
        raise ValueError("history file escapes the target repository") from error
    if not resolved.is_file():
        raise ValueError(f"history file does not exist: {relative.as_posix()}")
    return relative.as_posix()


def validate_base(raw: str | None) -> str | None:
    if raw is None or not raw.strip():
        return None
    value = raw.strip()
    if len(value) > MAX_BASE_LENGTH or "\n" in value or "\r" in value:
        raise ValueError("base revision is malformed or too long")
    return value


def git_head(repo: Path) -> str:
    output = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"],
        text=True,
    )
    return output.strip()


def repository_is_shallow(repo: Path) -> bool:
    output = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "--is-shallow-repository"],
        text=True,
    )
    return output.strip() == "true"


def split_stderr(stderr: str) -> tuple[dict[str, Any] | None, str]:
    perf: dict[str, Any] | None = None
    retained: list[str] = []
    for line in stderr.splitlines():
        if line.startswith(PERF_PREFIX):
            payload = line[len(PERF_PREFIX) :]
            parsed = json.loads(payload)
            if not isinstance(parsed, dict):
                raise ValueError("Cultist performance receipt was not a JSON object")
            if perf is not None:
                raise ValueError("Cultist emitted more than one performance receipt")
            perf = parsed
        else:
            retained.append(line)
    suffix = "\n" if retained else ""
    return perf, "\n".join(retained) + suffix


def run_probe(
    *,
    name: str,
    command: list[str],
    cwd: Path,
    output_dir: Path,
    environment: dict[str, str],
) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )

    perf: dict[str, Any] | None = None
    stderr = completed.stderr
    receipt_error: str | None = None
    try:
        perf, stderr = split_stderr(completed.stderr)
    except (ValueError, json.JSONDecodeError) as error:
        receipt_error = str(error)

    stdout_path = output_dir / f"{name}.json"
    stderr_path = output_dir / f"{name}.stderr.txt"
    stdout_path.write_text(completed.stdout, encoding="utf-8")
    stderr_path.write_text(stderr, encoding="utf-8")

    json_error: str | None = None
    report: object | None = None
    try:
        report = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        json_error = str(error)

    finding_count: int | None = None
    claim_count: int | None = None
    if isinstance(report, dict):
        findings = report.get("findings")
        claims = report.get("claims")
        if isinstance(findings, list):
            finding_count = len(findings)
        if isinstance(claims, list):
            claim_count = len(claims)

    errors = []
    if completed.returncode != 0:
        errors.append(f"exit status {completed.returncode}")
    if json_error is not None:
        errors.append(f"invalid JSON output: {json_error}")
    if receipt_error is not None:
        errors.append(f"invalid performance receipt: {receipt_error}")
    if perf is None:
        errors.append("missing performance receipt")

    return {
        "name": name,
        "command": command,
        "exit_code": completed.returncode,
        "finding_count": finding_count,
        "claim_count": claim_count,
        "performance": perf,
        "stdout": stdout_path.name,
        "stderr": stderr_path.name,
        "errors": errors,
    }


def main() -> int:
    args = parser().parse_args()

    cultist = args.cultist.resolve()
    repo = args.repo.resolve()
    output_dir = args.output_dir.resolve()

    if not cultist.is_file():
        raise SystemExit(f"Cultist binary does not exist: {cultist}")
    if not repo.is_dir():
        raise SystemExit(f"target repository does not exist: {repo}")
    if args.history_max < 1 or args.history_max > MAX_HISTORY_COMMITS:
        raise SystemExit(f"--history-max must be between 1 and {MAX_HISTORY_COMMITS}")
    if args.checkout_depth < 0:
        raise SystemExit("--checkout-depth must be zero or greater")

    try:
        history_file = canonical_history_file(repo, args.history_file)
        base = validate_base(args.base)
    except ValueError as error:
        raise SystemExit(str(error)) from error

    output_dir.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment["CARGO_CULTIST_PERF"] = "1"
    if args.cache_dir is not None:
        cache_dir = args.cache_dir.resolve()
        cache_dir.mkdir(parents=True, exist_ok=True)
        environment["CARGO_CULTIST_CACHE_DIR"] = str(cache_dir)

    target_head = git_head(repo)
    shallow = repository_is_shallow(repo)

    probes: list[dict[str, Any]] = []
    binary = str(cultist)
    root = str(repo)

    probes.append(
        run_probe(
            name="scan",
            command=[binary, "--format", "json", root],
            cwd=repo,
            output_dir=output_dir,
            environment=environment,
        )
    )

    if args.repeat_scan:
        probes.append(
            run_probe(
                name="scan-warm",
                command=[binary, "--format", "json", root],
                cwd=repo,
                output_dir=output_dir,
                environment=environment,
            )
        )

    probes.append(
        run_probe(
            name="ci-tests",
            command=[binary, "ci-tests", "--format", "json", root],
            cwd=repo,
            output_dir=output_dir,
            environment=environment,
        )
    )

    if history_file is not None:
        probes.append(
            run_probe(
                name="history",
                command=[
                    binary,
                    "history",
                    "--max-commits",
                    str(args.history_max),
                    "--format",
                    "json",
                    history_file,
                ],
                cwd=repo,
                output_dir=output_dir,
                environment=environment,
            )
        )

    if base is not None:
        probes.append(
            run_probe(
                name="diff",
                command=[binary, "diff", "--base", base, "--format", "json", root],
                cwd=repo,
                output_dir=output_dir,
                environment=environment,
            )
        )

    summary = {
        "schema_version": 1,
        "target": {
            "repository_path": str(repo),
            "head": target_head,
            "shallow": shallow,
            "checkout_depth": args.checkout_depth,
        },
        "bounds": {
            "history_file": history_file,
            "history_max_commits": args.history_max if history_file else None,
            "diff_base": base,
        },
        "safety": {
            "target_build_executed": False,
            "target_tests_executed": False,
            "target_commands_executed": False,
        },
        "probes": probes,
    }
    (output_dir / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    failed = [probe for probe in probes if probe["errors"]]
    print(json.dumps(summary, indent=2, sort_keys=True))
    if failed:
        print(
            "external dogfood probe failure: "
            + ", ".join(str(probe["name"]) for probe in failed),
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

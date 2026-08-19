#!/usr/bin/env python3
"""Reject new human-facing direct external github.com links.

The guard applies presentation hygiene only. Canonical provider/evidence URLs may be
retained with an explicit local escape marker, and code examples are ignored.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

ALLOW_MARKER = "<!-- cultist:allow-canonical-github-evidence -->"
DIRECT_GITHUB_URL_RE = re.compile(r"https://github\.com/[^\s<>()\[\]`\"']+")
INLINE_CODE_RE = re.compile(r"`[^`]*`")
FENCE_RE = re.compile(r"^\s*(```|~~~)")


class GuardError(RuntimeError):
    pass


@dataclass(frozen=True)
class Violation:
    source: str
    line: int
    url: str


def repository_from_url(url: str) -> str | None:
    parsed = urlsplit(url)
    if parsed.scheme != "https" or (parsed.hostname or "").lower() != "github.com":
        return None
    components = [component for component in parsed.path.split("/") if component]
    if len(components) < 2:
        return None
    return f"{components[0]}/{components[1]}"


def strip_inline_code(line: str) -> str:
    return INLINE_CODE_RE.sub("", line)


def scan_text(
    text: str,
    *,
    source: str,
    current_repository: str,
    eligible_lines: set[int] | None = None,
) -> list[Violation]:
    violations: list[Violation] = []
    in_fence = False
    allow_next_line = False

    for line_number, line in enumerate(text.splitlines(), start=1):
        fence = FENCE_RE.match(line)
        if fence:
            in_fence = not in_fence
            allow_next_line = False
            continue
        if in_fence:
            continue

        marker_on_line = ALLOW_MARKER in line
        if marker_on_line and line.strip() == ALLOW_MARKER:
            allow_next_line = True
            continue

        eligible = eligible_lines is None or line_number in eligible_lines
        sanitized = strip_inline_code(line)
        matches = list(DIRECT_GITHUB_URL_RE.finditer(sanitized))
        allow_this_line = marker_on_line or allow_next_line
        allow_next_line = False

        if not eligible or allow_this_line:
            continue

        for match in matches:
            url = match.group(0).rstrip(".,;:!?")
            repository = repository_from_url(url)
            if repository is None or repository == current_repository:
                continue
            violations.append(Violation(source=source, line=line_number, url=url))

    return violations


def git_diff(base: str, root: Path) -> str:
    completed = subprocess.run(
        [
            "git",
            "diff",
            "--unified=0",
            "--no-color",
            f"{base}...HEAD",
            "--",
            "*.md",
        ],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or "git diff failed"
        raise GuardError(f"could not diff changed Markdown from `{base}`: {detail}")
    return completed.stdout


def added_markdown_lines(diff: str) -> dict[str, set[int]]:
    result: dict[str, set[int]] = {}
    current_path: str | None = None
    new_line: int | None = None

    for raw_line in diff.splitlines():
        if raw_line.startswith("+++ "):
            value = raw_line[4:]
            if value == "/dev/null":
                current_path = None
            elif value.startswith("b/"):
                current_path = value[2:]
                result.setdefault(current_path, set())
            else:
                raise GuardError(f"unsupported git diff path line: {raw_line}")
            new_line = None
            continue

        if raw_line.startswith("@@ "):
            match = re.search(r"\+(\d+)(?:,(\d+))?", raw_line)
            if match is None:
                raise GuardError(f"could not parse git hunk header: {raw_line}")
            new_line = int(match.group(1))
            continue

        if current_path is None or new_line is None:
            continue
        if raw_line.startswith("+") and not raw_line.startswith("+++"):
            result[current_path].add(new_line)
            new_line += 1
        elif raw_line.startswith("-") and not raw_line.startswith("---"):
            continue
        else:
            new_line += 1

    return result


def scan_changed_markdown(
    *, base: str, root: Path, current_repository: str
) -> list[Violation]:
    diff = git_diff(base, root)
    changed = added_markdown_lines(diff)
    violations: list[Violation] = []
    for relative_path, eligible_lines in sorted(changed.items()):
        if not eligible_lines:
            continue
        path = root / relative_path
        if not path.is_file():
            continue
        violations.extend(
            scan_text(
                path.read_text(encoding="utf-8"),
                source=relative_path,
                current_repository=current_repository,
                eligible_lines=eligible_lines,
            )
        )
    return violations


def scan_pull_request_body(
    *, event_path: Path, current_repository: str
) -> list[Violation]:
    try:
        event = json.loads(event_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GuardError(f"could not read pull-request event JSON: {error}") from error
    pull_request = event.get("pull_request")
    if not isinstance(pull_request, dict):
        raise GuardError("event JSON does not contain a pull_request object")
    body = pull_request.get("body") or ""
    if not isinstance(body, str):
        raise GuardError("pull_request.body must be a string or null")
    return scan_text(
        body,
        source="pull_request.body",
        current_repository=current_repository,
    )


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description="Reject newly introduced human-facing direct external github.com links."
    )
    result.add_argument("--repository", required=True, help="Current owner/repository")
    result.add_argument("--base", help="Git base commit/ref for changed Markdown scanning")
    result.add_argument("--event", type=Path, help="GitHub pull_request event JSON to scan")
    result.add_argument("--root", type=Path, default=Path("."))
    return result


def main() -> int:
    args = parser().parse_args()
    if args.base is None and args.event is None:
        raise SystemExit("external-github-reference-guard: provide --base and/or --event")

    try:
        violations: list[Violation] = []
        if args.base is not None:
            violations.extend(
                scan_changed_markdown(
                    base=args.base,
                    root=args.root,
                    current_repository=args.repository,
                )
            )
        if args.event is not None:
            violations.extend(
                scan_pull_request_body(
                    event_path=args.event,
                    current_repository=args.repository,
                )
            )
    except GuardError as error:
        raise SystemExit(f"external-github-reference-guard: {error}") from error

    if violations:
        print("Direct external github.com references are not allowed in new human-facing prose:")
        for violation in violations:
            print(f"  {violation.source}:{violation.line}: {violation.url}")
        print(
            "Use https://redirect.github.com/... or add the local canonical-evidence marker "
            "when exact source text must remain canonical."
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

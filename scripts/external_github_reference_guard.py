#!/usr/bin/env python3
"""Preflight and detect unsafe third-party GitHub references."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

ALLOW_MARKER = "<!-- cultist:allow-canonical-github-evidence -->"
DIRECT_GITHUB_URL_RE = re.compile(r"https?://github\.com/[^\s<>()\[\]`\"']+")
SHORTHAND_RE = re.compile(
    r"(?<![A-Za-z0-9_.-])([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)#([0-9]+)\b"
)
INLINE_CODE_RE = re.compile(r"`[^`]*`")
FENCE_RE = re.compile(r"^\s*(```|~~~)")
DEFAULT_OWNED_OWNERS = {"teamleaderleo"}
MAX_INTERACTION_BYTES = 256 * 1024


class GuardError(RuntimeError):
    pass


@dataclass(frozen=True)
class Violation:
    source: str
    line: int
    url: str


def configured_owned_owners() -> set[str]:
    owners = set(DEFAULT_OWNED_OWNERS)
    configured = os.environ.get("CULTIST_OWNED_GITHUB_OWNERS", "")
    owners.update(
        owner.strip().lower()
        for owner in configured.split(",")
        if owner.strip()
    )
    return owners


def repository_from_url(url: str) -> str | None:
    parsed = urlsplit(url)
    if parsed.scheme not in {"http", "https"} or (parsed.hostname or "").lower() != "github.com":
        return None
    components = [component for component in parsed.path.split("/") if component]
    if len(components) < 2:
        return None
    return f"{components[0]}/{components[1]}".lower()


def is_owned_repository(
    repository: str,
    *,
    current_repository: str,
    owned_owners: set[str],
) -> bool:
    normalized = repository.lower()
    owner = normalized.split("/", 1)[0]
    return normalized == current_repository.lower() or owner in owned_owners


def strip_inline_code(line: str) -> str:
    return INLINE_CODE_RE.sub("", line)


def scan_text(
    text: str,
    *,
    source: str,
    current_repository: str,
    eligible_lines: set[int] | None = None,
    owned_owners: set[str] | None = None,
) -> list[Violation]:
    """Scan repository prose; code and one exact-evidence marker stay exempt."""

    violations: list[Violation] = []
    owners = configured_owned_owners() if owned_owners is None else owned_owners
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
        allow_this_line = marker_on_line or allow_next_line
        allow_next_line = False

        if not eligible or allow_this_line:
            continue

        for match in DIRECT_GITHUB_URL_RE.finditer(sanitized):
            url = match.group(0).rstrip(".,;:!?")
            repository = repository_from_url(url)
            if repository is None or is_owned_repository(
                repository,
                current_repository=current_repository,
                owned_owners=owners,
            ):
                continue
            violations.append(Violation(source=source, line=line_number, url=url))

        for match in SHORTHAND_RE.finditer(sanitized):
            repository = f"{match.group(1)}/{match.group(2)}".lower()
            if is_owned_repository(
                repository,
                current_repository=current_repository,
                owned_owners=owners,
            ):
                continue
            violations.append(
                Violation(
                    source=source,
                    line=line_number,
                    url=f"{match.group(1)}/{match.group(2)}#{match.group(3)}",
                )
            )

    return violations


def scan_interaction_text(
    text: str,
    *,
    source: str,
    current_repository: str,
    owned_owners: set[str] | None = None,
) -> list[Violation]:
    """Scan exact outbound GitHub interaction text before the write.

    Interaction preflight has no marker or Markdown-code exception. Automated text
    either uses redirect.github.com, uses non-linking wording, or stays within an
    owned repository.
    """

    encoded = text.encode("utf-8")
    if len(encoded) > MAX_INTERACTION_BYTES:
        raise GuardError(
            f"{source} exceeds the {MAX_INTERACTION_BYTES}-byte interaction preflight limit"
        )

    owners = configured_owned_owners() if owned_owners is None else owned_owners
    violations: list[Violation] = []
    for line_number, line in enumerate(text.splitlines(), start=1):
        for match in DIRECT_GITHUB_URL_RE.finditer(line):
            url = match.group(0).rstrip(".,;:!?")
            repository = repository_from_url(url)
            if repository is None or is_owned_repository(
                repository,
                current_repository=current_repository,
                owned_owners=owners,
            ):
                continue
            violations.append(Violation(source=source, line=line_number, url=url))

        for match in SHORTHAND_RE.finditer(line):
            repository = f"{match.group(1)}/{match.group(2)}".lower()
            if is_owned_repository(
                repository,
                current_repository=current_repository,
                owned_owners=owners,
            ):
                continue
            violations.append(
                Violation(
                    source=source,
                    line=line_number,
                    url=f"{match.group(1)}/{match.group(2)}#{match.group(3)}",
                )
            )
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
    *,
    base: str,
    root: Path,
    current_repository: str,
    owned_owners: set[str] | None = None,
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
                owned_owners=owned_owners,
            )
        )
    return violations


def scan_pull_request_body(
    *,
    event_path: Path,
    current_repository: str,
    owned_owners: set[str] | None = None,
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
    return scan_interaction_text(
        body,
        source="pull_request.body",
        current_repository=current_repository,
        owned_owners=owned_owners,
    )


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description="Preflight or detect unsafe third-party GitHub references."
    )
    result.add_argument("--repository", required=True, help="Current owner/repository")
    result.add_argument("--base", help="Git base commit/ref for changed Markdown scanning")
    result.add_argument("--event", type=Path, help="GitHub pull_request event JSON to scan")
    result.add_argument(
        "--stdin",
        action="store_true",
        help="Preflight the exact proposed interaction text from stdin before a GitHub write",
    )
    result.add_argument("--root", type=Path, default=Path("."))
    return result


def main() -> int:
    args = parser().parse_args()
    if args.base is None and args.event is None and not args.stdin:
        raise SystemExit(
            "external-github-reference-guard: provide --base, --event, and/or --stdin"
        )

    owners = configured_owned_owners()
    try:
        violations: list[Violation] = []
        if args.base is not None:
            violations.extend(
                scan_changed_markdown(
                    base=args.base,
                    root=args.root,
                    current_repository=args.repository,
                    owned_owners=owners,
                )
            )
        if args.event is not None:
            violations.extend(
                scan_pull_request_body(
                    event_path=args.event,
                    current_repository=args.repository,
                    owned_owners=owners,
                )
            )
        if args.stdin:
            violations.extend(
                scan_interaction_text(
                    sys.stdin.read(),
                    source="stdin",
                    current_repository=args.repository,
                    owned_owners=owners,
                )
            )
    except GuardError as error:
        raise SystemExit(f"external-github-reference-guard: {error}") from error

    if violations:
        print("Unsafe third-party GitHub references detected in human-facing text:")
        for violation in violations:
            print(f"  {violation.source}:{violation.line}: {violation.url}")
        print(
            "Before writing to GitHub, use a literal https://redirect.github.com/... URL "
            "or non-linking OWNER/REPOSITORY issue/PR/discussion NUMBER wording. "
            "Do not rely on post-write CI to prevent backlinks."
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Collect one-hop explicit GitHub project memory into Cultist's offline packet."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

API_ROOT = "https://api.github.com"
MAX_HTTP_BYTES = 1024 * 1024
MAX_BODY_BYTES = 32 * 1024
MAX_TITLE_BYTES = 512
MAX_CHANGED_PATHS = 512
MAX_ARTIFACTS_HARD = 32
MAX_EDGE_EVIDENCE_BYTES = 2 * 1024
REPOSITORY_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
REFERENCE_RE = re.compile(r"(?<![A-Za-z0-9_.-])#([1-9][0-9]*)\b")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Collect a selected GitHub pull request and its one-hop explicit "
            "same-repository references into a bounded Cultist project-memory packet."
        )
    )
    result.add_argument("--repository", required=True)
    result.add_argument("--anchor-pr", required=True, type=int)
    result.add_argument("--output", required=True, type=Path)
    result.add_argument(
        "--max-artifacts",
        type=int,
        default=16,
        help=f"maximum admitted artifacts, including anchor (2..{MAX_ARTIFACTS_HARD})",
    )
    return result


class CollectorError(RuntimeError):
    pass


class GitHubClient:
    def __init__(self, token: str | None) -> None:
        self._token = token

    def get_json(self, path: str) -> Any:
        if not path.startswith("/") or "\r" in path or "\n" in path:
            raise CollectorError("invalid GitHub API path")
        headers = {
            "Accept": "application/vnd.github+json",
            "User-Agent": "cargo-cultist-project-memory-research",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        if self._token:
            headers["Authorization"] = f"Bearer {self._token}"
        request = Request(API_ROOT + path, headers=headers)
        try:
            with urlopen(request, timeout=20) as response:
                if response.status != 200:
                    raise CollectorError(f"GitHub API returned HTTP {response.status}")
                payload = response.read(MAX_HTTP_BYTES + 1)
        except HTTPError as error:
            detail = error.read(4096).decode("utf-8", "replace")
            raise CollectorError(
                f"GitHub API request failed with HTTP {error.code}: {detail}"
            ) from error
        except URLError as error:
            raise CollectorError(f"GitHub API request failed: {error.reason}") from error

        if len(payload) > MAX_HTTP_BYTES:
            raise CollectorError("GitHub API response exceeds 1 MiB bound")
        try:
            return json.loads(payload)
        except json.JSONDecodeError as error:
            raise CollectorError(f"GitHub API returned invalid JSON: {error}") from error


def bounded_text(value: object, field: str, maximum: int, *, single_line: bool) -> str:
    if not isinstance(value, str) or not value:
        raise CollectorError(f"{field} is empty or missing")
    if len(value.encode("utf-8")) > maximum or "\x00" in value:
        raise CollectorError(f"{field} exceeds its admitted bound")
    if single_line and ("\n" in value or "\r" in value):
        raise CollectorError(f"{field} must be single-line")
    return value


def optional_body(value: object, title: str) -> tuple[str, bool]:
    if value is None or value == "":
        return title, False
    body = bounded_text(value, "artifact body", MAX_BODY_BYTES, single_line=False)
    return body, True


def exact_sha(value: object, field: str) -> str:
    if not isinstance(value, str) or SHA_RE.fullmatch(value) is None:
        raise CollectorError(f"{field} must be an exact lowercase 40-hex Git object id")
    return value


def timestamp(value: object, field: str) -> str:
    return bounded_text(value, field, 64, single_line=True)


def optional_timestamp(value: object, field: str) -> str | None:
    if value is None:
        return None
    return timestamp(value, field)


def canonical_path(value: object) -> str:
    path = bounded_text(value, "changed path", 4096, single_line=True)
    if (
        path.startswith("/")
        or "\\" in path
        or any(part in {"", ".", ".."} for part in path.split("/"))
    ):
        raise CollectorError(f"non-canonical changed path: {path}")
    return path


def state(value: object) -> str:
    if value not in {"open", "closed"}:
        raise CollectorError(f"unsupported GitHub artifact state: {value!r}")
    return str(value)


def pull_request_artifact(
    client: GitHubClient, repository: str, number: int
) -> dict[str, Any]:
    raw = client.get_json(f"/repos/{repository}/pulls/{number}")
    if not isinstance(raw, dict) or raw.get("number") != number:
        raise CollectorError(f"GitHub PR #{number} payload is malformed")

    title = bounded_text(raw.get("title"), "pull request title", MAX_TITLE_BYTES, single_line=True)
    evidence_text, complete = optional_body(raw.get("body"), title)
    changed_files = raw.get("changed_files")
    if not isinstance(changed_files, int) or isinstance(changed_files, bool):
        raise CollectorError(f"GitHub PR #{number} changed_files is malformed")
    if changed_files < 0 or changed_files > MAX_CHANGED_PATHS:
        raise CollectorError(
            f"GitHub PR #{number} exceeds the {MAX_CHANGED_PATHS}-path collector bound"
        )

    paths: list[str] = []
    page = 1
    while len(paths) < changed_files:
        raw_files = client.get_json(
            f"/repos/{repository}/pulls/{number}/files?per_page=100&page={page}"
        )
        if not isinstance(raw_files, list):
            raise CollectorError(f"GitHub PR #{number} files payload is malformed")
        if not raw_files:
            break
        for file in raw_files:
            if not isinstance(file, dict):
                raise CollectorError(f"GitHub PR #{number} file entry is malformed")
            paths.append(canonical_path(file.get("filename")))
            if len(paths) > MAX_CHANGED_PATHS:
                raise CollectorError(f"GitHub PR #{number} exceeds changed-path bound")
        if len(raw_files) < 100:
            break
        page += 1

    if len(paths) != changed_files:
        raise CollectorError(
            f"GitHub PR #{number} file list is incomplete: expected {changed_files}, got {len(paths)}"
        )

    base = raw.get("base")
    head = raw.get("head")
    if not isinstance(base, dict) or not isinstance(head, dict):
        raise CollectorError(f"GitHub PR #{number} revision payload is malformed")

    artifact_state = state(raw.get("state"))
    closed_at = optional_timestamp(raw.get("closed_at"), "closed_at")
    if artifact_state == "open" and closed_at is not None:
        raise CollectorError(f"open GitHub PR #{number} unexpectedly has closed_at")
    if artifact_state == "closed" and closed_at is None:
        raise CollectorError(f"closed GitHub PR #{number} is missing closed_at")

    return {
        "reference": {"kind": "pull_request", "number": number},
        "title": title,
        "state": artifact_state,
        "created_at": timestamp(raw.get("created_at"), "created_at"),
        "closed_at": closed_at,
        "revision": {
            "head_sha": exact_sha(head.get("sha"), "head_sha"),
            "base_sha": exact_sha(base.get("sha"), "base_sha"),
            "merged": raw.get("merged_at") is not None,
        },
        "changed_paths": paths,
        "evidence_text": evidence_text,
        "evidence_complete": complete,
    }


def issue_artifact(raw: dict[str, Any], number: int) -> dict[str, Any]:
    title = bounded_text(raw.get("title"), "issue title", MAX_TITLE_BYTES, single_line=True)
    evidence_text, complete = optional_body(raw.get("body"), title)
    artifact_state = state(raw.get("state"))
    closed_at = optional_timestamp(raw.get("closed_at"), "closed_at")
    if artifact_state == "open" and closed_at is not None:
        raise CollectorError(f"open GitHub issue #{number} unexpectedly has closed_at")
    if artifact_state == "closed" and closed_at is None:
        raise CollectorError(f"closed GitHub issue #{number} is missing closed_at")
    return {
        "reference": {"kind": "issue", "number": number},
        "title": title,
        "state": artifact_state,
        "created_at": timestamp(raw.get("created_at"), "created_at"),
        "closed_at": closed_at,
        "revision": None,
        "changed_paths": [],
        "evidence_text": evidence_text,
        "evidence_complete": complete,
    }


def referenced_artifact(
    client: GitHubClient, repository: str, number: int
) -> dict[str, Any]:
    raw = client.get_json(f"/repos/{repository}/issues/{number}")
    if not isinstance(raw, dict) or raw.get("number") != number:
        raise CollectorError(f"GitHub issue/PR #{number} payload is malformed")
    if "pull_request" in raw:
        return pull_request_artifact(client, repository, number)
    return issue_artifact(raw, number)


def relation_for_line(line: str) -> str | None:
    normalized = line.strip().lower()
    if re.match(r"^(closes|close|closed|fixes|fix|fixed|resolves|resolve|resolved)\b", normalized):
        return "closes"
    if normalized.startswith("follow-up to") or normalized.startswith("follow up to"):
        return "follow_up_to"
    if normalized.startswith("continuation from") or normalized.startswith("deployment continuation"):
        return "continuation_from"
    if normalized.startswith("parent:"):
        return "parent"
    if normalized.startswith("related:"):
        return "related"
    return None


def explicit_anchor_edges(anchor: dict[str, Any]) -> list[dict[str, Any]]:
    source = anchor["reference"]
    evidence_text = anchor["evidence_text"]
    edges: list[dict[str, Any]] = []
    seen: set[tuple[str, int, str]] = set()

    for raw_line in evidence_text.splitlines():
        line = raw_line.strip()
        relation = relation_for_line(line)
        if relation is None:
            continue
        if len(line.encode("utf-8")) > MAX_EDGE_EVIDENCE_BYTES:
            raise CollectorError("explicit relationship line exceeds edge-evidence bound")
        for match in REFERENCE_RE.finditer(line):
            number = int(match.group(1))
            key = (relation, number, line)
            if key in seen:
                continue
            seen.add(key)
            edges.append(
                {
                    "from": source,
                    "relation": relation,
                    "to": {"kind": "issue", "number": number},
                    "evidence": line,
                }
            )
    return edges


def collect(repository: str, anchor_number: int, max_artifacts: int) -> dict[str, Any]:
    if REPOSITORY_RE.fullmatch(repository) is None:
        raise CollectorError("repository must be canonical owner/name")
    if anchor_number <= 0:
        raise CollectorError("anchor PR number must be positive")
    if max_artifacts < 2 or max_artifacts > MAX_ARTIFACTS_HARD:
        raise CollectorError(f"max artifacts must be between 2 and {MAX_ARTIFACTS_HARD}")

    token = os.environ.get("GITHUB_TOKEN")
    client = GitHubClient(token if token else None)
    anchor = pull_request_artifact(client, repository, anchor_number)
    edges = explicit_anchor_edges(anchor)

    target_numbers: list[int] = []
    seen_numbers = {anchor_number}
    for edge in edges:
        number = edge["to"]["number"]
        if number in seen_numbers:
            continue
        seen_numbers.add(number)
        target_numbers.append(number)

    if 1 + len(target_numbers) > max_artifacts:
        raise CollectorError(
            f"anchor names {len(target_numbers)} distinct artifacts; max-artifacts={max_artifacts} would truncate explicit evidence"
        )

    artifacts = [anchor]
    kinds: dict[int, str] = {}
    for number in target_numbers:
        artifact = referenced_artifact(client, repository, number)
        artifacts.append(artifact)
        kinds[number] = artifact["reference"]["kind"]

    for edge in edges:
        number = edge["to"]["number"]
        kind = kinds.get(number)
        if kind is None:
            raise CollectorError(f"edge target #{number} was not collected")
        edge["to"]["kind"] = kind

    return {
        "schema_version": 1,
        "repository": repository,
        "anchor": {"kind": "pull_request", "number": anchor_number},
        "artifacts": artifacts,
        "edges": edges,
    }


def main() -> int:
    args = parser().parse_args()
    try:
        packet = collect(args.repository, args.anchor_pr, args.max_artifacts)
        encoded = json.dumps(packet, indent=2, sort_keys=True) + "\n"
        if len(encoded.encode("utf-8")) > 256 * 1024:
            raise CollectorError("collected packet exceeds offline packet byte bound")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    except (CollectorError, OSError) as error:
        print(f"project-memory-github: {error}", file=sys.stderr)
        return 1

    print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

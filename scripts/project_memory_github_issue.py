#!/usr/bin/env python3
"""Collect one-hop explicit issue/case memory into Cultist's offline packet."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys
from typing import Any

import project_memory_github as github_memory

MAX_ARTIFACTS_HARD = 32
ISSUE_URL_RE = re.compile(
    r"^https://(?:redirect\.github\.com|github\.com)/"
    r"(?P<repository>[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)/issues/"
    r"(?P<number>[1-9][0-9]*)(?:[/?#].*)?$"
)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Collect a selected GitHub issue and its one-hop explicit same-repository "
            "primary-case references into a bounded Cultist project-memory packet."
        )
    )
    result.add_argument("--repository", required=True)
    result.add_argument("--anchor-issue", required=True, type=int)
    result.add_argument("--output", required=True, type=Path)
    result.add_argument(
        "--max-artifacts",
        type=int,
        default=16,
        help=f"maximum admitted artifacts, including anchor (2..{MAX_ARTIFACTS_HARD})",
    )
    return result


def issue_anchor(
    client: github_memory.GitHubClient, repository: str, number: int
) -> dict[str, Any]:
    raw = client.get_json(f"/repos/{repository}/issues/{number}")
    if not isinstance(raw, dict) or raw.get("number") != number:
        raise github_memory.CollectorError(f"GitHub issue #{number} payload is malformed")
    if "pull_request" in raw:
        raise github_memory.CollectorError(f"GitHub #{number} is a pull request, not an issue")
    return github_memory.issue_artifact(raw, number)


def primary_case_edges(
    repository: str, anchor: dict[str, Any]
) -> list[dict[str, Any]]:
    lines = anchor["evidence_text"].splitlines()
    source = anchor["reference"]
    edges: list[dict[str, Any]] = []
    seen: set[int] = set()

    for index, raw_line in enumerate(lines):
        if raw_line.strip().lower() != "primary case:":
            continue
        target_index = index + 1
        while target_index < len(lines) and not lines[target_index].strip():
            target_index += 1
        if target_index >= len(lines):
            raise github_memory.CollectorError("Primary case block has no target")

        candidate = lines[target_index].strip()
        match = ISSUE_URL_RE.fullmatch(candidate)
        if match is None:
            raise github_memory.CollectorError(
                f"Primary case target is not an admitted GitHub issue URL: {candidate}"
            )
        if match.group("repository").lower() != repository.lower():
            raise github_memory.CollectorError(
                f"Primary case target escapes selected repository: {candidate}"
            )

        number = int(match.group("number"))
        if number == source["number"]:
            raise github_memory.CollectorError("Primary case may not self-reference")
        if number in seen:
            continue
        seen.add(number)

        evidence = "\n".join(lines[index : target_index + 1])
        if len(evidence.encode("utf-8")) > github_memory.MAX_EDGE_EVIDENCE_BYTES:
            raise github_memory.CollectorError("Primary case evidence exceeds edge bound")
        edges.append(
            {
                "from": source,
                "relation": "related",
                "to": {"kind": "issue", "number": number},
                "evidence": evidence,
            }
        )

    return edges


def collect(repository: str, anchor_number: int, max_artifacts: int) -> dict[str, Any]:
    if github_memory.REPOSITORY_RE.fullmatch(repository) is None:
        raise github_memory.CollectorError("repository must be canonical owner/name")
    if anchor_number <= 0:
        raise github_memory.CollectorError("anchor issue number must be positive")
    if max_artifacts < 2 or max_artifacts > MAX_ARTIFACTS_HARD:
        raise github_memory.CollectorError(
            f"max artifacts must be between 2 and {MAX_ARTIFACTS_HARD}"
        )

    token = os.environ.get("GITHUB_TOKEN")
    client = github_memory.GitHubClient(token if token else None)
    anchor = issue_anchor(client, repository, anchor_number)
    edges = primary_case_edges(repository, anchor)
    if not edges:
        raise github_memory.CollectorError("anchor issue has no admitted Primary case blocks")

    target_numbers = [edge["to"]["number"] for edge in edges]
    if 1 + len(target_numbers) > max_artifacts:
        raise github_memory.CollectorError(
            f"anchor names {len(target_numbers)} primary cases; max-artifacts={max_artifacts} "
            "would truncate explicit evidence"
        )

    artifacts = [anchor]
    kinds: dict[int, str] = {}
    for number in target_numbers:
        artifact = github_memory.referenced_artifact(client, repository, number)
        artifacts.append(artifact)
        kinds[number] = artifact["reference"]["kind"]

    for edge in edges:
        number = edge["to"]["number"]
        kind = kinds.get(number)
        if kind is None:
            raise github_memory.CollectorError(f"edge target #{number} was not collected")
        edge["to"]["kind"] = kind

    return {
        "schema_version": 1,
        "repository": repository,
        "anchor": {"kind": "issue", "number": anchor_number},
        "artifacts": artifacts,
        "edges": edges,
    }


def main() -> int:
    args = parser().parse_args()
    try:
        packet = collect(args.repository, args.anchor_issue, args.max_artifacts)
        encoded = json.dumps(packet, indent=2, sort_keys=True) + "\n"
        if len(encoded.encode("utf-8")) > 256 * 1024:
            raise github_memory.CollectorError("collected packet exceeds offline packet byte bound")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    except (github_memory.CollectorError, OSError) as error:
        print(f"project-memory-github-issue: {error}", file=sys.stderr)
        return 1

    print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

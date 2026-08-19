#!/usr/bin/env python3
"""Collect bounded explicit GitHub project-memory lineage into Cultist's offline packet."""

from __future__ import annotations

import argparse
from collections import deque
import json
import os
from pathlib import Path
import sys
from typing import Any

import project_memory_github as github_memory
import project_memory_github_issue as issue_memory

MAX_DEPTH_HARD = 3
MAX_ARTIFACTS_HARD = 32
MAX_EDGES = 256
MAX_PACKET_BYTES = 256 * 1024
MAX_RECEIPT_BYTES = 64 * 1024


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Collect an explicitly linked same-repository GitHub issue/PR lineage into "
            "Cultist's existing bounded project-memory packet."
        )
    )
    result.add_argument("--repository", required=True)
    anchor = result.add_mutually_exclusive_group(required=True)
    anchor.add_argument("--anchor-pr", type=int)
    anchor.add_argument("--anchor-issue", type=int)
    result.add_argument("--output", required=True, type=Path)
    result.add_argument("--receipt-output", required=True, type=Path)
    result.add_argument(
        "--max-depth",
        type=int,
        default=2,
        help=f"maximum explicit-link traversal depth (0..{MAX_DEPTH_HARD})",
    )
    result.add_argument(
        "--max-artifacts",
        type=int,
        default=16,
        help=f"maximum admitted artifacts including anchor (1..{MAX_ARTIFACTS_HARD})",
    )
    return result


def admitted_edges(repository: str, artifact: dict[str, Any]) -> list[dict[str, Any]]:
    """Return only relationship species already admitted by ProjectMemoryPacket v1."""
    edges = list(github_memory.explicit_anchor_edges(artifact))
    edges.extend(issue_memory.primary_case_edges(repository, artifact))
    return edges


def artifact_key(reference: dict[str, Any]) -> tuple[str, int]:
    kind = reference.get("kind")
    number = reference.get("number")
    if (
        kind not in {"issue", "pull_request"}
        or not isinstance(number, int)
        or isinstance(number, bool)
    ):
        raise github_memory.CollectorError("project-memory artifact reference is malformed")
    return kind, number


def edge_key(
    edge: dict[str, Any],
) -> tuple[tuple[str, int], str, tuple[str, int], str]:
    relation = edge.get("relation")
    evidence = edge.get("evidence")
    if not isinstance(relation, str) or not isinstance(evidence, str):
        raise github_memory.CollectorError("project-memory edge is malformed")
    return artifact_key(edge["from"]), relation, artifact_key(edge["to"]), evidence


def _anchor_artifact(
    client: github_memory.GitHubClient,
    repository: str,
    anchor_kind: str,
    anchor_number: int,
) -> dict[str, Any]:
    if anchor_number <= 0:
        raise github_memory.CollectorError("anchor number must be positive")
    if anchor_kind == "pull_request":
        return github_memory.pull_request_artifact(client, repository, anchor_number)
    if anchor_kind == "issue":
        return issue_memory.issue_anchor(client, repository, anchor_number)
    raise github_memory.CollectorError(f"unsupported anchor kind: {anchor_kind}")


def collect(
    repository: str,
    anchor_kind: str,
    anchor_number: int,
    max_depth: int,
    max_artifacts: int,
    *,
    client: github_memory.GitHubClient | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    if github_memory.REPOSITORY_RE.fullmatch(repository) is None:
        raise github_memory.CollectorError("repository must be canonical owner/name")
    if max_depth < 0 or max_depth > MAX_DEPTH_HARD:
        raise github_memory.CollectorError(
            f"max depth must be between 0 and {MAX_DEPTH_HARD}"
        )
    if max_artifacts < 1 or max_artifacts > MAX_ARTIFACTS_HARD:
        raise github_memory.CollectorError(
            f"max artifacts must be between 1 and {MAX_ARTIFACTS_HARD}"
        )

    if client is None:
        token = os.environ.get("GITHUB_TOKEN")
        client = github_memory.GitHubClient(token if token else None)

    anchor = _anchor_artifact(client, repository, anchor_kind, anchor_number)
    anchor_reference = anchor["reference"]
    anchor_identity = artifact_key(anchor_reference)

    artifacts = [anchor]
    artifacts_by_number: dict[int, dict[str, Any]] = {anchor_number: anchor}
    depth_by_number: dict[int, int] = {anchor_number: 0}
    queue: deque[int] = deque([anchor_number])
    edges: list[dict[str, Any]] = []
    seen_edges: set[
        tuple[tuple[str, int], str, tuple[str, int], str]
    ] = set()
    depth_frontier: list[dict[str, Any]] = []

    while queue:
        source_number = queue.popleft()
        source = artifacts_by_number[source_number]
        source_depth = depth_by_number[source_number]
        if source_depth == max_depth:
            # We retain the artifact but deliberately do not inspect it for deeper links.
            # The sidecar receipt makes that omission observable.
            depth_frontier.append(source["reference"].copy())
            continue

        for edge in admitted_edges(repository, source):
            if edge.get("from") != source["reference"]:
                raise github_memory.CollectorError(
                    "collector edge source disagrees with expanded artifact"
                )
            raw_target = edge.get("to")
            if not isinstance(raw_target, dict):
                raise github_memory.CollectorError("collector edge target is malformed")
            target_number = raw_target.get("number")
            if (
                not isinstance(target_number, int)
                or isinstance(target_number, bool)
                or target_number <= 0
            ):
                raise github_memory.CollectorError(
                    "collector edge target number is malformed"
                )
            if target_number == source_number:
                raise github_memory.CollectorError(
                    "explicit project-memory lineage may not self-reference"
                )

            target = artifacts_by_number.get(target_number)
            if target is None:
                if len(artifacts) >= max_artifacts:
                    raise github_memory.CollectorError(
                        f"explicit lineage exceeds max-artifacts={max_artifacts}; "
                        f"next target is #{target_number}"
                    )
                target = github_memory.referenced_artifact(
                    client, repository, target_number
                )
                artifacts_by_number[target_number] = target
                artifacts.append(target)
                depth_by_number[target_number] = source_depth + 1
                queue.append(target_number)

            # `#N` is syntactically ambiguous until GitHub resolves it. Preserve the
            # same relation/evidence while correcting the target artifact species.
            edge["to"]["kind"] = target["reference"]["kind"]
            key = edge_key(edge)
            if key in seen_edges:
                continue
            if len(edges) >= MAX_EDGES:
                raise github_memory.CollectorError(
                    f"explicit lineage exceeds the {MAX_EDGES}-edge packet bound"
                )
            seen_edges.add(key)
            edges.append(edge)

    packet = {
        "schema_version": 1,
        "repository": repository,
        "anchor": anchor_reference,
        "artifacts": artifacts,
        "edges": edges,
    }
    receipt = {
        "schema_version": 1,
        "collector": "github_explicit_lineage",
        "repository": repository,
        "anchor": anchor_reference,
        "requested_max_depth": max_depth,
        "max_artifacts": max_artifacts,
        "artifact_count": len(artifacts),
        "edge_count": len(edges),
        "depth_frontier": depth_frontier,
        "complete_within_requested_depth": True,
    }

    if artifact_key(packet["anchor"]) != anchor_identity:
        raise github_memory.CollectorError(
            "project-memory anchor identity changed during collection"
        )

    packet_bytes = (json.dumps(packet, indent=2, sort_keys=True) + "\n").encode(
        "utf-8"
    )
    if len(packet_bytes) > MAX_PACKET_BYTES:
        raise github_memory.CollectorError(
            f"collected lineage packet exceeds the {MAX_PACKET_BYTES}-byte offline bound"
        )
    receipt_bytes = (
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")
    if len(receipt_bytes) > MAX_RECEIPT_BYTES:
        raise github_memory.CollectorError(
            f"lineage collection receipt exceeds the {MAX_RECEIPT_BYTES}-byte bound"
        )

    return packet, receipt


def main() -> int:
    args = parser().parse_args()
    anchor_kind = "pull_request" if args.anchor_pr is not None else "issue"
    anchor_number = args.anchor_pr if args.anchor_pr is not None else args.anchor_issue
    assert anchor_number is not None

    try:
        packet, receipt = collect(
            args.repository,
            anchor_kind,
            anchor_number,
            args.max_depth,
            args.max_artifacts,
        )
        encoded_packet = json.dumps(packet, indent=2, sort_keys=True) + "\n"
        encoded_receipt = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.receipt_output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded_packet, encoding="utf-8")
        args.receipt_output.write_text(encoded_receipt, encoding="utf-8")
    except (github_memory.CollectorError, OSError) as error:
        print(f"project-memory-github-lineage: {error}", file=sys.stderr)
        return 1

    print(encoded_packet, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

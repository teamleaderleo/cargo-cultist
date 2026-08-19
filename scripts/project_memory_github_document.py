#!/usr/bin/env python3
"""Collect explicitly named repository documents into an offline Cultist packet."""

from __future__ import annotations

import argparse
import base64
import binascii
import json
import os
from pathlib import Path
import sys
from typing import Any
from urllib.parse import quote

import project_memory_github as github_memory

MAX_DOCUMENTS = 16
MAX_DOCUMENT_BYTES = 128 * 1024
MAX_SOURCE_EVIDENCE_BYTES = 8 * 1024


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Collect explicitly named UTF-8 repository files at one exact Git revision, "
            "requiring a selected GitHub issue to name every admitted path."
        )
    )
    result.add_argument("--repository", required=True)
    result.add_argument("--revision", required=True)
    result.add_argument("--source-issue", required=True, type=int)
    result.add_argument("--path", dest="paths", required=True, action="append")
    result.add_argument("--output", required=True, type=Path)
    return result


def canonical_path(value: str) -> str:
    if (
        not value
        or len(value.encode("utf-8")) > 4096
        or value.startswith("/")
        or "\\" in value
        or "\x00" in value
        or any(part in {"", ".", ".."} for part in value.split("/"))
    ):
        raise github_memory.CollectorError(f"non-canonical repository path: {value!r}")
    return value


def source_issue(
    client: github_memory.GitHubClient, repository: str, number: int
) -> dict[str, Any]:
    raw = client.get_json(f"/repos/{repository}/issues/{number}")
    if not isinstance(raw, dict) or raw.get("number") != number:
        raise github_memory.CollectorError(f"GitHub issue #{number} payload is malformed")
    if "pull_request" in raw:
        raise github_memory.CollectorError(f"GitHub #{number} is a pull request, not an issue")
    return github_memory.issue_artifact(raw, number)


def evidence_for_path(evidence_text: str, path: str) -> str:
    lines = evidence_text.splitlines()
    for index, line in enumerate(lines):
        if path not in line:
            continue
        start = index
        previous = index - 1
        while previous >= 0 and not lines[previous].strip():
            previous -= 1
        if previous >= 0:
            start = previous
        evidence = "\n".join(lines[start : index + 1])
        if len(evidence.encode("utf-8")) > MAX_SOURCE_EVIDENCE_BYTES:
            raise github_memory.CollectorError(
                f"source evidence for `{path}` exceeds {MAX_SOURCE_EVIDENCE_BYTES} bytes"
            )
        return evidence
    raise github_memory.CollectorError(
        f"selected source issue does not explicitly name document path `{path}`"
    )


def repository_document(
    client: github_memory.GitHubClient,
    repository: str,
    revision: str,
    path: str,
    source_number: int,
    source_evidence: str,
) -> dict[str, Any]:
    encoded_path = quote(path, safe="/")
    raw = client.get_json(
        f"/repos/{repository}/contents/{encoded_path}?ref={revision}"
    )
    if not isinstance(raw, dict) or raw.get("type") != "file":
        raise github_memory.CollectorError(f"GitHub path is not a file: {path}")
    if raw.get("path") != path:
        raise github_memory.CollectorError(
            f"GitHub content path mismatch: expected {path!r}, got {raw.get('path')!r}"
        )

    size = raw.get("size")
    if not isinstance(size, int) or isinstance(size, bool) or size < 1:
        raise github_memory.CollectorError(f"GitHub content size is malformed for `{path}`")
    if size > MAX_DOCUMENT_BYTES:
        raise github_memory.CollectorError(
            f"document `{path}` exceeds the {MAX_DOCUMENT_BYTES}-byte collector bound"
        )
    if raw.get("encoding") != "base64":
        raise github_memory.CollectorError(f"GitHub content for `{path}` is not base64")

    encoded = raw.get("content")
    if not isinstance(encoded, str):
        raise github_memory.CollectorError(f"GitHub content body is missing for `{path}`")
    try:
        decoded = base64.b64decode("".join(encoded.split()), validate=True)
    except (binascii.Error, ValueError) as error:
        raise github_memory.CollectorError(
            f"GitHub content body is invalid base64 for `{path}`"
        ) from error
    if len(decoded) != size:
        raise github_memory.CollectorError(
            f"GitHub content size mismatch for `{path}`: expected {size}, got {len(decoded)}"
        )
    try:
        text = decoded.decode("utf-8")
    except UnicodeDecodeError as error:
        raise github_memory.CollectorError(
            f"document `{path}` is not UTF-8"
        ) from error

    return {
        "path": path,
        "blob_sha": github_memory.exact_sha(raw.get("sha"), "blob_sha"),
        "text": text,
        "text_complete": True,
        "source": {"kind": "issue", "number": source_number},
        "source_evidence": source_evidence,
    }


def collect(
    repository: str, revision: str, source_number: int, requested_paths: list[str]
) -> dict[str, Any]:
    if github_memory.REPOSITORY_RE.fullmatch(repository) is None:
        raise github_memory.CollectorError("repository must be canonical owner/name")
    revision = github_memory.exact_sha(revision, "revision")
    if source_number <= 0:
        raise github_memory.CollectorError("source issue number must be positive")
    if not requested_paths or len(requested_paths) > MAX_DOCUMENTS:
        raise github_memory.CollectorError(
            f"collector requires 1..={MAX_DOCUMENTS} explicit document paths"
        )

    paths = [canonical_path(path) for path in requested_paths]
    if len(paths) != len(set(paths)):
        raise github_memory.CollectorError("document paths must be unique")

    token = os.environ.get("GITHUB_TOKEN")
    client = github_memory.GitHubClient(token if token else None)
    source = source_issue(client, repository, source_number)
    documents = []
    for path in paths:
        evidence = evidence_for_path(source["evidence_text"], path)
        documents.append(
            repository_document(
                client,
                repository,
                revision,
                path,
                source_number,
                evidence,
            )
        )

    return {
        "schema_version": 1,
        "repository": repository,
        "revision": revision,
        "documents": documents,
    }


def main() -> int:
    args = parser().parse_args()
    try:
        packet = collect(args.repository, args.revision, args.source_issue, args.paths)
        encoded = json.dumps(packet, indent=2, sort_keys=True) + "\n"
        if len(encoded.encode("utf-8")) > 512 * 1024:
            raise github_memory.CollectorError(
                "collected project-document packet exceeds offline byte bound"
            )
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    except (github_memory.CollectorError, OSError) as error:
        print(f"project-memory-github-document: {error}", file=sys.stderr)
        return 1

    print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

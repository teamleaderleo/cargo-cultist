#!/usr/bin/env python3
"""Bind one selected GitHub inline-review thread to Cultist review memory."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import sys
from typing import Any

import project_memory_github as github_memory

MAX_REVIEW_COMMENTS = 512
MAX_COMMENT_BODY_BYTES = 16 * 1024
MAX_ID_BYTES = 1024
MAX_QUERY_BYTES = 256 * 1024
MAX_RECEIPT_BYTES = 256 * 1024
OUTCOMES = {"open", "patch_changed", "rejected_with_evidence", "dismissed"}


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Collect one explicitly selected GitHub inline-review thread and emit "
            "Cultist's revision-aware review-memory query."
        )
    )
    result.add_argument("--repository", required=True)
    result.add_argument("--pull-request", required=True, type=int)
    root = result.add_mutually_exclusive_group(required=True)
    root.add_argument("--comment-id", type=int)
    root.add_argument("--comment-node-id")
    result.add_argument("--concern-key", required=True)
    result.add_argument("--outcome", required=True, choices=sorted(OUTCOMES))
    resolution = result.add_mutually_exclusive_group()
    resolution.add_argument("--resolution-comment-id", type=int)
    resolution.add_argument("--resolution-comment-node-id")
    result.add_argument("--output", required=True, type=Path)
    result.add_argument("--receipt-output", required=True, type=Path)
    return result


def bounded_single_line(value: object, field: str, maximum: int = MAX_ID_BYTES) -> str:
    if (
        not isinstance(value, str)
        or not value
        or value.strip() != value
        or len(value.encode("utf-8")) > maximum
        or "\x00" in value
        or "\n" in value
        or "\r" in value
    ):
        raise github_memory.CollectorError(
            f"{field} must be a bounded non-empty single-line value"
        )
    return value


def bounded_body(value: object, field: str) -> str:
    if not isinstance(value, str) or not value or "\x00" in value:
        raise github_memory.CollectorError(f"{field} is empty or malformed")
    if len(value.encode("utf-8")) > MAX_COMMENT_BODY_BYTES:
        raise github_memory.CollectorError(
            f"{field} exceeds the {MAX_COMMENT_BODY_BYTES}-byte bound"
        )
    return value


def positive_int(value: object, field: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise github_memory.CollectorError(f"{field} must be a positive integer")
    return value


def optional_positive_int(value: object, field: str) -> int | None:
    if value is None:
        return None
    return positive_int(value, field)


def exact_pull_request_url(repository: str, number: int, value: object) -> str:
    expected = f"https://api.github.com/repos/{repository}/pulls/{number}"
    actual = bounded_single_line(value, "pull_request_url", 4096)
    if actual != expected:
        raise github_memory.CollectorError(
            f"review comment belongs to a different pull request: {actual}"
        )
    return actual


def optional_side(value: object, field: str) -> str | None:
    if value is None:
        return None
    if value not in {"LEFT", "RIGHT"}:
        raise github_memory.CollectorError(f"{field} must be LEFT, RIGHT, or null")
    return str(value)


def comment_identity(pull_request: int, comment: dict[str, Any]) -> str:
    return f"github:pull/{pull_request}/review-comment/{positive_int(comment.get('id'), 'comment id')}"


def list_review_comments(
    client: github_memory.GitHubClient, repository: str, pull_request: int
) -> list[dict[str, Any]]:
    comments: list[dict[str, Any]] = []
    page = 1
    while True:
        raw = client.get_json(
            f"/repos/{repository}/pulls/{pull_request}/comments?per_page=100&page={page}"
        )
        if not isinstance(raw, list):
            raise github_memory.CollectorError("GitHub review-comment list is malformed")
        if len(raw) > 100:
            raise github_memory.CollectorError("GitHub review-comment page exceeds per-page bound")
        for item in raw:
            if not isinstance(item, dict):
                raise github_memory.CollectorError("GitHub review-comment entry is malformed")
            comments.append(item)
            if len(comments) > MAX_REVIEW_COMMENTS:
                raise github_memory.CollectorError(
                    f"review-comment inventory exceeds the {MAX_REVIEW_COMMENTS}-comment bound"
                )
        if len(raw) < 100:
            break
        page += 1
    return comments


def index_comments(
    comments: list[dict[str, Any]],
) -> tuple[dict[int, dict[str, Any]], dict[str, dict[str, Any]]]:
    by_id: dict[int, dict[str, Any]] = {}
    by_node: dict[str, dict[str, Any]] = {}
    for comment in comments:
        comment_id = positive_int(comment.get("id"), "comment id")
        node_id = bounded_single_line(comment.get("node_id"), "comment node_id")
        if comment_id in by_id:
            raise github_memory.CollectorError(
                f"duplicate GitHub review-comment id {comment_id}"
            )
        if node_id in by_node:
            raise github_memory.CollectorError(
                f"duplicate GitHub review-comment node_id {node_id}"
            )
        by_id[comment_id] = comment
        by_node[node_id] = comment
    return by_id, by_node


def select_comment(
    by_id: dict[int, dict[str, Any]],
    by_node: dict[str, dict[str, Any]],
    *,
    comment_id: int | None,
    node_id: str | None,
    label: str,
) -> dict[str, Any]:
    if comment_id is not None:
        if comment_id <= 0:
            raise github_memory.CollectorError(f"{label} id must be positive")
        selected = by_id.get(comment_id)
        selector = str(comment_id)
    else:
        if node_id is None:
            raise github_memory.CollectorError(f"{label} selector is missing")
        node_id = bounded_single_line(node_id, f"{label} node_id")
        selected = by_node.get(node_id)
        selector = node_id
    if selected is None:
        raise github_memory.CollectorError(f"selected {label} {selector} was not found")
    return selected


def comment_receipt(
    repository: str, pull_request: int, comment: dict[str, Any]
) -> dict[str, Any]:
    exact_pull_request_url(repository, pull_request, comment.get("pull_request_url"))
    comment_id = positive_int(comment.get("id"), "comment id")
    node_id = bounded_single_line(comment.get("node_id"), "comment node_id")
    review_id = positive_int(comment.get("pull_request_review_id"), "pull_request_review_id")
    path = github_memory.canonical_path(comment.get("path"))
    return {
        "id": comment_id,
        "node_id": node_id,
        "pull_request_review_id": review_id,
        "commit_id": github_memory.exact_sha(comment.get("commit_id"), "comment commit_id"),
        "original_commit_id": github_memory.exact_sha(
            comment.get("original_commit_id"), "comment original_commit_id"
        ),
        "path": path,
        "line": optional_positive_int(comment.get("line"), "comment line"),
        "original_line": optional_positive_int(
            comment.get("original_line"), "comment original_line"
        ),
        "start_line": optional_positive_int(comment.get("start_line"), "comment start_line"),
        "original_start_line": optional_positive_int(
            comment.get("original_start_line"), "comment original_start_line"
        ),
        "side": optional_side(comment.get("side"), "comment side"),
        "start_side": optional_side(comment.get("start_side"), "comment start_side"),
        "in_reply_to_id": optional_positive_int(
            comment.get("in_reply_to_id"), "comment in_reply_to_id"
        ),
        "created_at": github_memory.timestamp(comment.get("created_at"), "comment created_at"),
        "updated_at": github_memory.timestamp(comment.get("updated_at"), "comment updated_at"),
        "body": bounded_body(comment.get("body"), "comment body"),
    }


def collect(
    repository: str,
    pull_request: int,
    concern_key: str,
    outcome: str,
    *,
    comment_id: int | None = None,
    comment_node_id: str | None = None,
    resolution_comment_id: int | None = None,
    resolution_comment_node_id: str | None = None,
    client: github_memory.GitHubClient | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    if github_memory.REPOSITORY_RE.fullmatch(repository) is None:
        raise github_memory.CollectorError("repository must be canonical owner/name")
    if pull_request <= 0:
        raise github_memory.CollectorError("pull request number must be positive")
    concern_key = bounded_single_line(concern_key, "concern_key")
    if outcome not in OUTCOMES:
        raise github_memory.CollectorError(f"unsupported review outcome: {outcome}")

    has_resolution = (
        resolution_comment_id is not None or resolution_comment_node_id is not None
    )
    if outcome == "open" and has_resolution:
        raise github_memory.CollectorError("open outcome forbids a resolution comment")
    if outcome != "open" and not has_resolution:
        raise github_memory.CollectorError(
            f"outcome={outcome} requires a selected resolution comment"
        )
    if comment_id is None and comment_node_id is None:
        raise github_memory.CollectorError("root review-comment selector is missing")
    if comment_id is not None and comment_node_id is not None:
        raise github_memory.CollectorError("root review-comment selector is ambiguous")
    if resolution_comment_id is not None and resolution_comment_node_id is not None:
        raise github_memory.CollectorError("resolution review-comment selector is ambiguous")

    if client is None:
        token = os.environ.get("GITHUB_TOKEN")
        client = github_memory.GitHubClient(token if token else None)

    raw_pr = client.get_json(f"/repos/{repository}/pulls/{pull_request}")
    if not isinstance(raw_pr, dict) or raw_pr.get("number") != pull_request:
        raise github_memory.CollectorError(
            f"GitHub PR #{pull_request} payload is malformed"
        )
    head = raw_pr.get("head")
    if not isinstance(head, dict):
        raise github_memory.CollectorError(
            f"GitHub PR #{pull_request} head payload is malformed"
        )
    current_head = github_memory.exact_sha(head.get("sha"), "current PR head_sha")

    comments = list_review_comments(client, repository, pull_request)
    by_id, by_node = index_comments(comments)
    root = select_comment(
        by_id,
        by_node,
        comment_id=comment_id,
        node_id=comment_node_id,
        label="root review comment",
    )
    root_receipt = comment_receipt(repository, pull_request, root)
    if root_receipt["in_reply_to_id"] is not None:
        raise github_memory.CollectorError(
            "selected root review comment is itself a reply"
        )

    resolution_receipt: dict[str, Any] | None = None
    if has_resolution:
        resolution = select_comment(
            by_id,
            by_node,
            comment_id=resolution_comment_id,
            node_id=resolution_comment_node_id,
            label="resolution review comment",
        )
        resolution_receipt = comment_receipt(repository, pull_request, resolution)
        if resolution_receipt["in_reply_to_id"] != root_receipt["id"]:
            raise github_memory.CollectorError(
                "selected resolution comment is not a direct reply to the root review comment"
            )

    work = f"github:pull/{pull_request}"
    root_identity = comment_identity(pull_request, root)
    resolution_identity = (
        comment_identity(pull_request, resolution)
        if has_resolution and resolution_receipt is not None
        else None
    )

    record: dict[str, Any] = {
        "event_id": root_identity,
        "concern_key": concern_key,
        "source_ref": root_identity,
        "subject": {
            "repository": repository,
            "work": work,
            "revision": root_receipt["commit_id"],
            "scope": {"mode": "exact", "path": root_receipt["path"]},
        },
        "outcome": outcome,
    }
    if resolution_identity is not None:
        record["resolution_ref"] = resolution_identity

    query = {
        "schema_version": 1,
        "current": {
            "concern_key": concern_key,
            "context": {
                "repository": repository,
                "revision": current_head,
                "work": work,
                "path": root_receipt["path"],
            },
        },
        "records": [record],
    }
    receipt = {
        "schema_version": 1,
        "provider": "github_review_comment",
        "repository": repository,
        "pull_request": pull_request,
        "current_head_sha": current_head,
        "concern_key": concern_key,
        "selected_outcome": outcome,
        "review_comment_count": len(comments),
        "pagination_complete": True,
        "root": root_receipt,
        "resolution": resolution_receipt,
    }

    query_bytes = (json.dumps(query, indent=2, sort_keys=True) + "\n").encode("utf-8")
    if len(query_bytes) > MAX_QUERY_BYTES:
        raise github_memory.CollectorError(
            f"review-memory query exceeds the {MAX_QUERY_BYTES}-byte bound"
        )
    receipt_bytes = (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode(
        "utf-8"
    )
    if len(receipt_bytes) > MAX_RECEIPT_BYTES:
        raise github_memory.CollectorError(
            f"GitHub review-memory receipt exceeds the {MAX_RECEIPT_BYTES}-byte bound"
        )
    return query, receipt


def main() -> int:
    args = parser().parse_args()
    try:
        query, receipt = collect(
            args.repository,
            args.pull_request,
            args.concern_key,
            args.outcome,
            comment_id=args.comment_id,
            comment_node_id=args.comment_node_id,
            resolution_comment_id=args.resolution_comment_id,
            resolution_comment_node_id=args.resolution_comment_node_id,
        )
        encoded_query = json.dumps(query, indent=2, sort_keys=True) + "\n"
        encoded_receipt = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.receipt_output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded_query, encoding="utf-8")
        args.receipt_output.write_text(encoded_receipt, encoding="utf-8")
    except (github_memory.CollectorError, OSError) as error:
        print(f"review-memory-github: {error}", file=sys.stderr)
        return 1

    print(encoded_query, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

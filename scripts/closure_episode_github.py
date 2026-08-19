#!/usr/bin/env python3
"""Collect one explicit GitHub re-report + administrative closure episode."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys
from typing import Any

import project_memory_github as github_memory

MAX_COMMENTS = 256
MAX_BODY_BYTES = 64 * 1024
MAX_COMMENT_EVIDENCE_BYTES = 32 * 1024
MAX_EPISODE_BYTES = 256 * 1024
MAX_RECEIPT_BYTES = 128 * 1024
RE_REPORT_RE = re.compile(r"^\*\*Re-reporting\*\* the bug from #([1-9][0-9]*)\b.*$")


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description=(
            "Collect a selected GitHub issue that explicitly re-reports an earlier "
            "same-repository issue, preserving exact administrative closure evidence."
        )
    )
    result.add_argument("--repository", required=True)
    result.add_argument("--later-issue", required=True, type=int)
    result.add_argument("--output", required=True, type=Path)
    result.add_argument("--receipt-output", required=True, type=Path)
    return result


def bounded_body(value: object, field: str, maximum: int = MAX_BODY_BYTES) -> str:
    if not isinstance(value, str) or not value or "\x00" in value:
        raise github_memory.CollectorError(f"{field} is empty or malformed")
    if len(value.encode("utf-8")) > maximum:
        raise github_memory.CollectorError(f"{field} exceeds the {maximum}-byte bound")
    return value


def bounded_comment_evidence(value: object, field: str) -> str:
    return bounded_body(value, field, MAX_COMMENT_EVIDENCE_BYTES)


def login_from_user(value: object, field: str) -> str:
    if not isinstance(value, dict):
        raise github_memory.CollectorError(f"{field} user is malformed")
    return github_memory.bounded_text(
        value.get("login"), f"{field} login", 512, single_line=True
    )


def optional_login_from_user(value: object, field: str) -> str | None:
    if value is None:
        return None
    return login_from_user(value, field)


def issue_snapshot(raw: dict[str, Any], expected_number: int) -> tuple[dict[str, Any], str]:
    if raw.get("number") != expected_number or "pull_request" in raw:
        raise github_memory.CollectorError(
            f"GitHub issue #{expected_number} payload is malformed or is a pull request"
        )
    title = github_memory.bounded_text(
        raw.get("title"), "issue title", 1024, single_line=True
    )
    state = raw.get("state")
    if state not in {"open", "closed"}:
        raise github_memory.CollectorError(f"unsupported issue state: {state!r}")
    created_at = github_memory.timestamp(raw.get("created_at"), "issue created_at")
    closed_at = github_memory.optional_timestamp(raw.get("closed_at"), "issue closed_at")
    if state == "open" and closed_at is not None:
        raise github_memory.CollectorError("open issue unexpectedly has closed_at")
    if state == "closed" and closed_at is None:
        raise github_memory.CollectorError("closed issue is missing closed_at")

    state_reason = raw.get("state_reason")
    if state_reason is not None:
        state_reason = github_memory.bounded_text(
            state_reason, "issue state_reason", 512, single_line=True
        )
    closed_by = optional_login_from_user(raw.get("closed_by"), "closed_by")
    if state == "open" and closed_by is not None:
        raise github_memory.CollectorError("open issue unexpectedly has closed_by")
    reporter = login_from_user(raw.get("user"), "issue reporter")

    snapshot: dict[str, Any] = {
        "number": expected_number,
        "title": title,
        "state": state,
        "created_at": created_at,
    }
    if state_reason is not None:
        snapshot["state_reason"] = state_reason
    if closed_at is not None:
        snapshot["closed_at"] = closed_at
    if closed_by is not None:
        snapshot["closed_by"] = closed_by
    return snapshot, reporter


def re_report_relation(body: str) -> tuple[int, str]:
    matches: list[tuple[int, str]] = []
    for raw_line in body.splitlines():
        line = raw_line.strip()
        match = RE_REPORT_RE.fullmatch(line)
        if match is None:
            continue
        if len(line.encode("utf-8")) > MAX_COMMENT_EVIDENCE_BYTES:
            raise github_memory.CollectorError("re-report evidence exceeds admitted bound")
        matches.append((int(match.group(1)), line))
    if not matches:
        raise github_memory.CollectorError(
            "later issue has no admitted explicit `**Re-reporting** the bug from #N` line"
        )
    if len(matches) != 1:
        raise github_memory.CollectorError(
            "later issue has multiple admitted re-report lines; selection is ambiguous"
        )
    return matches[0]


def list_issue_comments(
    client: github_memory.GitHubClient, repository: str, number: int
) -> list[dict[str, Any]]:
    comments: list[dict[str, Any]] = []
    page = 1
    while True:
        raw = client.get_json(
            f"/repos/{repository}/issues/{number}/comments?per_page=100&page={page}"
        )
        if not isinstance(raw, list):
            raise github_memory.CollectorError("GitHub issue-comment list is malformed")
        if len(raw) > 100:
            raise github_memory.CollectorError("GitHub issue-comment page exceeds bound")
        for item in raw:
            if not isinstance(item, dict):
                raise github_memory.CollectorError("GitHub issue-comment entry is malformed")
            comments.append(item)
            if len(comments) > MAX_COMMENTS:
                raise github_memory.CollectorError(
                    f"issue-comment inventory exceeds the {MAX_COMMENTS}-comment bound"
                )
        if len(raw) < 100:
            break
        page += 1
    return comments


def comment_record(comment: dict[str, Any], field: str) -> dict[str, Any]:
    comment_id = comment.get("id")
    if not isinstance(comment_id, int) or isinstance(comment_id, bool) or comment_id <= 0:
        raise github_memory.CollectorError(f"{field} id must be positive")
    actor = login_from_user(comment.get("user"), field)
    body = bounded_comment_evidence(comment.get("body"), f"{field} body")
    return {
        "comment_id": comment_id,
        "source_ref": f"github:issue-comment/{comment_id}",
        "actor": actor,
        "evidence": body,
    }


def administrative_inactive_evidence(repository: str) -> str:
    return (
        "Closing for now — inactive for too long. Please "
        f"[open a new issue](https://github.com/{repository}/issues/new/choose) "
        "if this is still relevant."
    )


def select_closure(
    comments: list[dict[str, Any]], repository: str
) -> dict[str, Any]:
    expected = administrative_inactive_evidence(repository)
    matches: list[dict[str, Any]] = []
    for comment in comments:
        record = comment_record(comment, "closure candidate")
        if record["actor"] == "github-actions[bot]" and record["evidence"] == expected:
            matches.append(record)
    if not matches:
        raise github_memory.CollectorError(
            "prior issue has no admitted exact administrative-inactivity closure comment"
        )
    if len(matches) != 1:
        raise github_memory.CollectorError(
            "prior issue has multiple exact administrative-inactivity closure comments"
        )
    selected = matches[0]
    return {
        "issue": None,
        "comment_id": selected["comment_id"],
        "source_ref": selected["source_ref"],
        "actor": selected["actor"],
        "kind": "administrative_inactive",
        "evidence": selected["evidence"],
    }


def select_duplicate_challenge(
    comments: list[dict[str, Any]], reporter: str
) -> dict[str, Any] | None:
    suggestions: list[dict[str, Any]] = []
    rejections: list[dict[str, Any]] = []
    for comment in comments:
        record = comment_record(comment, "duplicate-challenge candidate")
        evidence = record["evidence"]
        if (
            record["actor"] == "github-actions[bot]"
            and "possible duplicate issues:" in evidence
            and "This issue will be automatically closed as a duplicate in 3 days." in evidence
        ):
            suggestions.append(record)
        if record["actor"] == reporter and evidence.startswith(
            "Not a duplicate of the suggested issues."
        ):
            rejections.append(record)

    if not suggestions and not rejections:
        return None
    if len(suggestions) != 1 or len(rejections) != 1:
        raise github_memory.CollectorError(
            "duplicate challenge evidence is incomplete or ambiguous"
        )
    suggestion = suggestions[0]
    rejection = rejections[0]
    if suggestion["comment_id"] == rejection["comment_id"]:
        raise github_memory.CollectorError("duplicate challenge comments must differ")
    return {
        "suggestion_comment_id": suggestion["comment_id"],
        "suggestion_source_ref": suggestion["source_ref"],
        "suggestion_actor": suggestion["actor"],
        "suggestion_evidence": suggestion["evidence"],
        "rejection_comment_id": rejection["comment_id"],
        "rejection_source_ref": rejection["source_ref"],
        "rejection_actor": rejection["actor"],
        "rejection_evidence": rejection["evidence"],
    }


def collect(
    repository: str,
    later_number: int,
    *,
    client: github_memory.GitHubClient | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    if github_memory.REPOSITORY_RE.fullmatch(repository) is None:
        raise github_memory.CollectorError("repository must be canonical owner/name")
    if later_number <= 0:
        raise github_memory.CollectorError("later issue number must be positive")

    if client is None:
        token = os.environ.get("GITHUB_TOKEN")
        client = github_memory.GitHubClient(token if token else None)

    later_raw = client.get_json(f"/repos/{repository}/issues/{later_number}")
    if not isinstance(later_raw, dict):
        raise github_memory.CollectorError("later GitHub issue payload is malformed")
    later_snapshot, _later_reporter = issue_snapshot(later_raw, later_number)
    later_body = bounded_body(later_raw.get("body"), "later issue body")
    prior_number, relation_evidence = re_report_relation(later_body)
    if prior_number == later_number:
        raise github_memory.CollectorError("later issue may not re-report itself")

    prior_raw = client.get_json(f"/repos/{repository}/issues/{prior_number}")
    if not isinstance(prior_raw, dict):
        raise github_memory.CollectorError("prior GitHub issue payload is malformed")
    prior_snapshot, prior_reporter = issue_snapshot(prior_raw, prior_number)
    if prior_snapshot["state"] != "closed":
        raise github_memory.CollectorError("selected prior issue is not closed")

    comments = list_issue_comments(client, repository, prior_number)
    closure = select_closure(comments, repository)
    closure["issue"] = prior_number
    duplicate_challenge = select_duplicate_challenge(comments, prior_reporter)

    episode: dict[str, Any] = {
        "schema_version": 1,
        "repository": repository,
        "prior": prior_snapshot,
        "later": later_snapshot,
        "closure": closure,
        "re_report": {
            "from_issue": later_number,
            "to_issue": prior_number,
            "relation": "re_report_of",
            "source_ref": f"github:issue/{later_number}",
            "evidence": relation_evidence,
        },
    }
    if duplicate_challenge is not None:
        episode["duplicate_challenge"] = duplicate_challenge

    receipt = {
        "schema_version": 1,
        "provider": "github_issue_closure_episode",
        "repository": repository,
        "later_issue": later_number,
        "prior_issue": prior_number,
        "prior_comment_count": len(comments),
        "pagination_complete": True,
        "closure_comment_id": closure["comment_id"],
        "closure_actor": closure["actor"],
        "duplicate_challenge_retained": duplicate_challenge is not None,
        "prior_state_reason": prior_snapshot.get("state_reason"),
        "later_state_reason": later_snapshot.get("state_reason"),
    }

    episode_bytes = (json.dumps(episode, indent=2, sort_keys=True) + "\n").encode("utf-8")
    if len(episode_bytes) > MAX_EPISODE_BYTES:
        raise github_memory.CollectorError(
            f"closure episode exceeds the {MAX_EPISODE_BYTES}-byte bound"
        )
    receipt_bytes = (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode("utf-8")
    if len(receipt_bytes) > MAX_RECEIPT_BYTES:
        raise github_memory.CollectorError(
            f"closure provider receipt exceeds the {MAX_RECEIPT_BYTES}-byte bound"
        )
    return episode, receipt


def main() -> int:
    args = parser().parse_args()
    try:
        episode, receipt = collect(args.repository, args.later_issue)
        encoded_episode = json.dumps(episode, indent=2, sort_keys=True) + "\n"
        encoded_receipt = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.receipt_output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded_episode, encoding="utf-8")
        args.receipt_output.write_text(encoded_receipt, encoding="utf-8")
    except (github_memory.CollectorError, OSError) as error:
        print(f"closure-episode-github: {error}", file=sys.stderr)
        return 1

    print(encoded_episode, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Regression controls for the selected GitHub review-memory adapter."""

from __future__ import annotations

import unittest
from typing import Any

import project_memory_github as github_memory
import review_memory_github as adapter

REPOSITORY = "owner/repo"
PR = 7
HEAD_A = "a" * 40
HEAD_B = "b" * 40


class FakeGitHubClient:
    def __init__(self, responses: dict[str, Any]) -> None:
        self.responses = responses
        self.calls: list[str] = []

    def get_json(self, path: str) -> Any:
        self.calls.append(path)
        if path not in self.responses:
            raise AssertionError(f"unexpected GitHub request: {path}")
        return self.responses[path]


def pr_payload(head: str = HEAD_B) -> dict[str, Any]:
    return {"number": PR, "head": {"sha": head}}


def comment_payload(
    comment_id: int,
    node_id: str,
    *,
    commit_id: str = HEAD_A,
    original_commit_id: str = HEAD_A,
    path: str = "src/lib.rs",
    in_reply_to_id: int | None = None,
    pull_request: int = PR,
    body: str = "review concern",
) -> dict[str, Any]:
    return {
        "id": comment_id,
        "node_id": node_id,
        "pull_request_review_id": 42,
        "commit_id": commit_id,
        "original_commit_id": original_commit_id,
        "path": path,
        "line": 12,
        "original_line": 10,
        "start_line": None,
        "original_start_line": None,
        "side": "RIGHT",
        "start_side": None,
        "in_reply_to_id": in_reply_to_id,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:01:00Z",
        "body": body,
        "pull_request_url": f"https://api.github.com/repos/{REPOSITORY}/pulls/{pull_request}",
    }


def responses(comments: list[dict[str, Any]], *, head: str = HEAD_B) -> dict[str, Any]:
    return {
        f"/repos/{REPOSITORY}/pulls/{PR}": pr_payload(head),
        f"/repos/{REPOSITORY}/pulls/{PR}/comments?per_page=100&page=1": comments,
    }


class ReviewMemoryGitHubAdapterTests(unittest.TestCase):
    def test_emits_exact_review_memory_query_from_selected_thread(self) -> None:
        root = comment_payload(10, "ROOT")
        reply = comment_payload(
            11,
            "REPLY",
            commit_id=HEAD_B,
            in_reply_to_id=10,
            body="Fixed in `deadbeef`.",
        )
        client = FakeGitHubClient(responses([root, reply]))

        query, receipt = adapter.collect(
            REPOSITORY,
            PR,
            "review:github-fallback-publish-state",
            "patch_changed",
            comment_node_id="ROOT",
            resolution_comment_node_id="REPLY",
            client=client,
        )

        record = query["records"][0]
        self.assertEqual(query["current"]["context"]["revision"], HEAD_B)
        self.assertEqual(query["current"]["context"]["work"], "github:pull/7")
        self.assertEqual(record["subject"]["revision"], HEAD_A)
        self.assertEqual(record["subject"]["scope"], {"mode": "exact", "path": "src/lib.rs"})
        self.assertEqual(record["outcome"], "patch_changed")
        self.assertEqual(
            record["resolution_ref"], "github:pull/7/review-comment/11"
        )
        self.assertEqual(receipt["root"]["node_id"], "ROOT")
        self.assertEqual(receipt["root"]["original_line"], 10)
        self.assertEqual(receipt["resolution"]["in_reply_to_id"], 10)
        self.assertTrue(receipt["pagination_complete"])

    def test_numeric_comment_selectors_are_supported(self) -> None:
        root = comment_payload(10, "ROOT")
        reply = comment_payload(11, "REPLY", in_reply_to_id=10)
        client = FakeGitHubClient(responses([root, reply]))

        query, _ = adapter.collect(
            REPOSITORY,
            PR,
            "review:fixture",
            "dismissed",
            comment_id=10,
            resolution_comment_id=11,
            client=client,
        )

        self.assertEqual(
            query["records"][0]["event_id"], "github:pull/7/review-comment/10"
        )

    def test_selected_root_may_not_be_a_reply(self) -> None:
        reply = comment_payload(11, "REPLY", in_reply_to_id=10)
        client = FakeGitHubClient(responses([reply]))

        with self.assertRaisesRegex(
            github_memory.CollectorError, "root review comment is itself a reply"
        ):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "open",
                comment_node_id="REPLY",
                client=client,
            )

    def test_resolution_must_reply_directly_to_selected_root(self) -> None:
        root = comment_payload(10, "ROOT")
        wrong_reply = comment_payload(11, "REPLY", in_reply_to_id=9)
        client = FakeGitHubClient(responses([root, wrong_reply]))

        with self.assertRaisesRegex(
            github_memory.CollectorError, "not a direct reply"
        ):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "patch_changed",
                comment_node_id="ROOT",
                resolution_comment_node_id="REPLY",
                client=client,
            )

    def test_selected_comment_must_belong_to_selected_pull_request(self) -> None:
        root = comment_payload(10, "ROOT", pull_request=8)
        client = FakeGitHubClient(responses([root]))

        with self.assertRaisesRegex(
            github_memory.CollectorError, "different pull request"
        ):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "open",
                comment_node_id="ROOT",
                client=client,
            )

    def test_duplicate_node_ids_fail_closed(self) -> None:
        comments = [comment_payload(10, "DUP"), comment_payload(11, "DUP")]
        client = FakeGitHubClient(responses(comments))

        with self.assertRaisesRegex(
            github_memory.CollectorError, "duplicate GitHub review-comment node_id"
        ):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "open",
                comment_node_id="DUP",
                client=client,
            )

    def test_noncanonical_path_rejects(self) -> None:
        root = comment_payload(10, "ROOT", path="../src/lib.rs")
        client = FakeGitHubClient(responses([root]))

        with self.assertRaisesRegex(github_memory.CollectorError, "non-canonical"):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "open",
                comment_node_id="ROOT",
                client=client,
            )

    def test_invalid_comment_and_current_head_shas_reject(self) -> None:
        root = comment_payload(10, "ROOT", commit_id="HEAD")
        client = FakeGitHubClient(responses([root]))
        with self.assertRaisesRegex(github_memory.CollectorError, "comment commit_id"):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "open",
                comment_node_id="ROOT",
                client=client,
            )

        root = comment_payload(10, "ROOT")
        client = FakeGitHubClient(responses([root], head="HEAD"))
        with self.assertRaisesRegex(github_memory.CollectorError, "current PR head_sha"):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "open",
                comment_node_id="ROOT",
                client=client,
            )

    def test_outcome_resolution_contract_rejects_before_provider_work(self) -> None:
        client = FakeGitHubClient({})

        with self.assertRaisesRegex(
            github_memory.CollectorError, "open outcome forbids"
        ):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "open",
                comment_node_id="ROOT",
                resolution_comment_node_id="REPLY",
                client=client,
            )
        with self.assertRaisesRegex(
            github_memory.CollectorError, "requires a selected resolution comment"
        ):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "patch_changed",
                comment_node_id="ROOT",
                client=client,
            )
        self.assertEqual(client.calls, [])

    def test_inventory_overflow_fails_instead_of_truncating(self) -> None:
        all_comments = [
            comment_payload(index + 1, f"NODE-{index + 1}")
            for index in range(adapter.MAX_REVIEW_COMMENTS + 1)
        ]
        paged: dict[str, Any] = {f"/repos/{REPOSITORY}/pulls/{PR}": pr_payload()}
        for page in range(1, 7):
            start = (page - 1) * 100
            paged[
                f"/repos/{REPOSITORY}/pulls/{PR}/comments?per_page=100&page={page}"
            ] = all_comments[start : start + 100]
        client = FakeGitHubClient(paged)

        with self.assertRaisesRegex(
            github_memory.CollectorError, "exceeds the 512-comment bound"
        ):
            adapter.collect(
                REPOSITORY,
                PR,
                "review:fixture",
                "open",
                comment_node_id="NODE-1",
                client=client,
            )


if __name__ == "__main__":
    unittest.main()

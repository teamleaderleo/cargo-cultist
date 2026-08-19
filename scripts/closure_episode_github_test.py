#!/usr/bin/env python3
"""Regression controls for the GitHub issue closure episode collector."""

from __future__ import annotations

import unittest
from typing import Any

import closure_episode_github as adapter
import project_memory_github as github_memory

REPOSITORY = "owner/repo"
PRIOR = 10
LATER = 20


class FakeGitHubClient:
    def __init__(self, responses: dict[str, Any]) -> None:
        self.responses = responses
        self.calls: list[str] = []

    def get_json(self, path: str) -> Any:
        self.calls.append(path)
        if path not in self.responses:
            raise AssertionError(f"unexpected GitHub request: {path}")
        return self.responses[path]


def issue_payload(
    number: int,
    *,
    body: str,
    state: str,
    reporter: str = "reporter",
    closed_by: str | None = "github-actions[bot]",
    state_reason: str | None = "not_planned",
) -> dict[str, Any]:
    return {
        "number": number,
        "title": f"issue {number}",
        "body": body,
        "state": state,
        "state_reason": state_reason,
        "created_at": "2026-01-01T00:00:00Z",
        "closed_at": "2026-01-02T00:00:00Z" if state == "closed" else None,
        "closed_by": {"login": closed_by} if state == "closed" and closed_by else None,
        "user": {"login": reporter},
    }


def comment(comment_id: int, actor: str, body: str) -> dict[str, Any]:
    return {"id": comment_id, "user": {"login": actor}, "body": body}


def admin_closure(repository: str = REPOSITORY) -> str:
    return (
        "Closing for now — inactive for too long. Please "
        f"[open a new issue](https://github.com/{repository}/issues/new/choose) "
        "if this is still relevant."
    )


def base_responses(
    *,
    later_body: str | None = None,
    prior_state: str = "closed",
    comments: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    if later_body is None:
        later_body = "**Re-reporting** the bug from #10 (closed earlier, not fixed)."
    if comments is None:
        comments = [
            comment(
                90,
                "github-actions[bot]",
                "Found 3 possible duplicate issues:\n\nThis issue will be automatically closed as a duplicate in 3 days.",
            ),
            comment(
                91,
                "reporter",
                "Not a duplicate of the suggested issues.\n\nThey describe different failure modes.",
            ),
            comment(100, "github-actions[bot]", admin_closure()),
        ]
    return {
        f"/repos/{REPOSITORY}/issues/{LATER}": issue_payload(
            LATER,
            body=later_body,
            state="closed",
            reporter="later-reporter",
        ),
        f"/repos/{REPOSITORY}/issues/{PRIOR}": issue_payload(
            PRIOR,
            body="original report",
            state=prior_state,
            reporter="reporter",
        ),
        f"/repos/{REPOSITORY}/issues/{PRIOR}/comments?per_page=100&page=1": comments,
    }


class ClosureEpisodeGithubTests(unittest.TestCase):
    def test_collects_exact_closure_rereport_and_duplicate_challenge(self) -> None:
        client = FakeGitHubClient(base_responses())

        episode, receipt = adapter.collect(REPOSITORY, LATER, client=client)

        self.assertEqual(episode["prior"]["number"], PRIOR)
        self.assertEqual(episode["prior"]["state"], "closed")
        self.assertEqual(episode["prior"]["state_reason"], "not_planned")
        self.assertEqual(episode["later"]["number"], LATER)
        self.assertEqual(episode["later"]["state"], "closed")
        self.assertEqual(episode["closure"]["kind"], "administrative_inactive")
        self.assertEqual(episode["closure"]["comment_id"], 100)
        self.assertEqual(episode["re_report"]["to_issue"], PRIOR)
        self.assertEqual(episode["re_report"]["from_issue"], LATER)
        self.assertEqual(episode["re_report"]["relation"], "re_report_of")
        self.assertEqual(
            episode["duplicate_challenge"]["suggestion_comment_id"], 90
        )
        self.assertEqual(
            episode["duplicate_challenge"]["rejection_comment_id"], 91
        )
        self.assertEqual(receipt["prior_comment_count"], 3)
        self.assertTrue(receipt["pagination_complete"])

    def test_closed_state_reason_without_exact_closure_receipt_fails(self) -> None:
        client = FakeGitHubClient(
            base_responses(comments=[comment(100, "maintainer", "Closed")])
        )

        with self.assertRaisesRegex(
            github_memory.CollectorError, "no admitted exact administrative-inactivity"
        ):
            adapter.collect(REPOSITORY, LATER, client=client)

    def test_exact_closure_sentence_from_non_bot_does_not_classify(self) -> None:
        client = FakeGitHubClient(
            base_responses(comments=[comment(100, "human", admin_closure())])
        )

        with self.assertRaisesRegex(
            github_memory.CollectorError, "no admitted exact administrative-inactivity"
        ):
            adapter.collect(REPOSITORY, LATER, client=client)

    def test_arbitrary_or_ambiguous_relation_does_not_create_rereport(self) -> None:
        arbitrary = FakeGitHubClient(base_responses(later_body="Related to #10"))
        with self.assertRaisesRegex(github_memory.CollectorError, "no admitted explicit"):
            adapter.collect(REPOSITORY, LATER, client=arbitrary)
        self.assertNotIn(f"/repos/{REPOSITORY}/issues/{PRIOR}", arbitrary.calls)

        ambiguous = FakeGitHubClient(
            base_responses(
                later_body=(
                    "**Re-reporting** the bug from #10.\n"
                    "**Re-reporting** the bug from #11."
                )
            )
        )
        with self.assertRaisesRegex(github_memory.CollectorError, "multiple admitted"):
            adapter.collect(REPOSITORY, LATER, client=ambiguous)

    def test_prior_issue_must_be_closed(self) -> None:
        client = FakeGitHubClient(base_responses(prior_state="open"))

        with self.assertRaisesRegex(github_memory.CollectorError, "prior issue is not closed"):
            adapter.collect(REPOSITORY, LATER, client=client)

    def test_duplicate_challenge_must_be_complete_when_detected(self) -> None:
        comments = [
            comment(
                90,
                "github-actions[bot]",
                "Found 3 possible duplicate issues:\n\nThis issue will be automatically closed as a duplicate in 3 days.",
            ),
            comment(100, "github-actions[bot]", admin_closure()),
        ]
        client = FakeGitHubClient(base_responses(comments=comments))

        with self.assertRaisesRegex(
            github_memory.CollectorError, "duplicate challenge evidence is incomplete"
        ):
            adapter.collect(REPOSITORY, LATER, client=client)

    def test_comment_inventory_overflow_fails_instead_of_truncating(self) -> None:
        all_comments = [comment(index + 1, "someone", "evidence") for index in range(257)]
        all_comments[-1] = comment(257, "github-actions[bot]", admin_closure())
        responses = base_responses(comments=[])
        for page in range(1, 4):
            start = (page - 1) * 100
            responses[
                f"/repos/{REPOSITORY}/issues/{PRIOR}/comments?per_page=100&page={page}"
            ] = all_comments[start : start + 100]
        client = FakeGitHubClient(responses)

        with self.assertRaisesRegex(
            github_memory.CollectorError, "exceeds the 256-comment bound"
        ):
            adapter.collect(REPOSITORY, LATER, client=client)

    def test_invalid_selection_rejects_before_provider_work(self) -> None:
        client = FakeGitHubClient({})
        with self.assertRaisesRegex(github_memory.CollectorError, "canonical owner/name"):
            adapter.collect("bad/repo/name", LATER, client=client)
        with self.assertRaisesRegex(github_memory.CollectorError, "must be positive"):
            adapter.collect(REPOSITORY, 0, client=client)
        self.assertEqual(client.calls, [])


if __name__ == "__main__":
    unittest.main()

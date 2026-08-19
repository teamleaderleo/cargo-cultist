#!/usr/bin/env python3
"""Regression controls for the bounded GitHub project-memory lineage collector."""

from __future__ import annotations

import unittest
from typing import Any

import project_memory_github as github_memory
import project_memory_github_lineage as lineage

REPOSITORY = "owner/repo"


def issue_payload(number: int, body: str, *, state: str = "closed") -> dict[str, Any]:
    return {
        "number": number,
        "title": f"issue {number}",
        "body": body,
        "state": state,
        "created_at": "2026-01-01T00:00:00Z",
        "closed_at": "2026-01-02T00:00:00Z" if state == "closed" else None,
    }


def pr_issue_payload(number: int, body: str) -> dict[str, Any]:
    payload = issue_payload(number, body)
    payload["pull_request"] = {
        "url": f"https://api.github.com/repos/{REPOSITORY}/pulls/{number}"
    }
    return payload


def pr_payload(number: int, body: str) -> dict[str, Any]:
    return {
        "number": number,
        "title": f"pull request {number}",
        "body": body,
        "state": "closed",
        "created_at": "2026-01-01T00:00:00Z",
        "closed_at": "2026-01-02T00:00:00Z",
        "merged_at": "2026-01-02T00:00:00Z",
        "changed_files": 0,
        "base": {"sha": "1" * 40},
        "head": {"sha": "2" * 40},
    }


class FakeGitHubClient:
    def __init__(self, responses: dict[str, Any]) -> None:
        self.responses = responses
        self.calls: list[str] = []

    def get_json(self, path: str) -> Any:
        self.calls.append(path)
        if path not in self.responses:
            raise AssertionError(f"unexpected GitHub request: {path}")
        return self.responses[path]


def issue_responses(items: dict[int, str]) -> dict[str, Any]:
    return {
        f"/repos/{REPOSITORY}/issues/{number}": issue_payload(number, body)
        for number, body in items.items()
    }


class ProjectMemoryGithubLineageTests(unittest.TestCase):
    def test_collects_two_hop_explicit_lineage(self) -> None:
        client = FakeGitHubClient(
            issue_responses(
                {
                    3: "Follow-up to #2",
                    2: "Continuation from #1",
                    1: "Earlier evidence",
                }
            )
        )

        packet, receipt = lineage.collect(
            REPOSITORY, "issue", 3, 2, 8, client=client
        )

        self.assertEqual(
            [artifact["reference"]["number"] for artifact in packet["artifacts"]],
            [3, 2, 1],
        )
        self.assertEqual(
            [edge["relation"] for edge in packet["edges"]],
            ["follow_up_to", "continuation_from"],
        )
        self.assertEqual(
            receipt["depth_frontier"], [{"kind": "issue", "number": 1}]
        )
        self.assertEqual(receipt["incomplete_evidence_artifacts"], [])
        self.assertTrue(receipt["complete_within_requested_depth"])

    def test_depth_limit_keeps_unexpanded_frontier_visible(self) -> None:
        client = FakeGitHubClient(
            issue_responses(
                {
                    3: "Follow-up to #2",
                    2: "Continuation from #1",
                    1: "Earlier evidence",
                }
            )
        )

        packet, receipt = lineage.collect(
            REPOSITORY, "issue", 3, 1, 8, client=client
        )

        self.assertEqual(
            [artifact["reference"]["number"] for artifact in packet["artifacts"]],
            [3, 2],
        )
        self.assertEqual(
            receipt["depth_frontier"], [{"kind": "issue", "number": 2}]
        )
        self.assertNotIn(f"/repos/{REPOSITORY}/issues/1", client.calls)

    def test_cycle_terminates_without_duplicate_artifacts(self) -> None:
        client = FakeGitHubClient(
            issue_responses(
                {
                    3: "Follow-up to #2",
                    2: "Follow-up to #3",
                }
            )
        )

        packet, _ = lineage.collect(REPOSITORY, "issue", 3, 3, 8, client=client)

        self.assertEqual(len(packet["artifacts"]), 2)
        self.assertEqual(len(packet["edges"]), 2)
        self.assertEqual(
            [artifact["reference"]["number"] for artifact in packet["artifacts"]],
            [3, 2],
        )

    def test_resolves_hash_reference_to_pull_request_kind(self) -> None:
        responses = issue_responses({3: "Related: #4"})
        responses[f"/repos/{REPOSITORY}/issues/4"] = pr_issue_payload(4, "PR body")
        responses[f"/repos/{REPOSITORY}/pulls/4"] = pr_payload(4, "PR body")
        client = FakeGitHubClient(responses)

        packet, _ = lineage.collect(REPOSITORY, "issue", 3, 1, 8, client=client)

        self.assertEqual(
            packet["artifacts"][1]["reference"],
            {"kind": "pull_request", "number": 4},
        )
        self.assertEqual(
            packet["edges"][0]["to"],
            {"kind": "pull_request", "number": 4},
        )

    def test_primary_case_block_composes_with_lineage(self) -> None:
        client = FakeGitHubClient(
            issue_responses(
                {
                    3: f"Primary case:\nhttps://github.com/{REPOSITORY}/issues/2",
                    2: "Follow-up to #1",
                    1: "Earlier evidence",
                }
            )
        )

        packet, _ = lineage.collect(REPOSITORY, "issue", 3, 2, 8, client=client)

        self.assertEqual(
            [edge["relation"] for edge in packet["edges"]],
            ["related", "follow_up_to"],
        )
        self.assertIn("Primary case:", packet["edges"][0]["evidence"])

    def test_duplicate_textual_reference_does_not_duplicate_artifact(self) -> None:
        client = FakeGitHubClient(
            issue_responses(
                {
                    3: "Follow-up to #2/#2",
                    2: "Earlier evidence",
                }
            )
        )

        packet, _ = lineage.collect(REPOSITORY, "issue", 3, 1, 8, client=client)

        self.assertEqual(len(packet["artifacts"]), 2)
        self.assertEqual(len(packet["edges"]), 1)

    def test_artifact_overflow_fails_instead_of_truncating(self) -> None:
        client = FakeGitHubClient(
            issue_responses(
                {
                    3: "Follow-up to #2",
                    2: "Continuation from #1",
                    1: "Earlier evidence",
                }
            )
        )

        with self.assertRaisesRegex(
            github_memory.CollectorError, "exceeds max-artifacts=2"
        ):
            lineage.collect(REPOSITORY, "issue", 3, 2, 2, client=client)

    def test_unrelated_anchor_stays_anchor_only(self) -> None:
        client = FakeGitHubClient(
            issue_responses({3: "Observed failure without an explicit relationship."})
        )

        packet, receipt = lineage.collect(
            REPOSITORY, "issue", 3, 2, 8, client=client
        )

        self.assertEqual(len(packet["artifacts"]), 1)
        self.assertEqual(packet["edges"], [])
        self.assertEqual(receipt["depth_frontier"], [])
        self.assertEqual(receipt["incomplete_evidence_artifacts"], [])
        self.assertTrue(receipt["complete_within_requested_depth"])

    def test_missing_body_marks_lineage_evidence_incomplete(self) -> None:
        client = FakeGitHubClient(issue_responses({3: ""}))

        packet, receipt = lineage.collect(
            REPOSITORY, "issue", 3, 2, 8, client=client
        )

        self.assertEqual(packet["edges"], [])
        self.assertFalse(packet["artifacts"][0]["evidence_complete"])
        self.assertEqual(
            receipt["incomplete_evidence_artifacts"],
            [{"kind": "issue", "number": 3}],
        )
        self.assertFalse(receipt["complete_within_requested_depth"])

    def test_self_reference_fails_closed(self) -> None:
        client = FakeGitHubClient(issue_responses({3: "Follow-up to #3"}))

        with self.assertRaisesRegex(github_memory.CollectorError, "self-reference"):
            lineage.collect(REPOSITORY, "issue", 3, 1, 8, client=client)

    def test_invalid_bounds_reject_before_provider_work(self) -> None:
        client = FakeGitHubClient({})

        with self.assertRaisesRegex(github_memory.CollectorError, "max depth"):
            lineage.collect(REPOSITORY, "issue", 3, 4, 8, client=client)
        with self.assertRaisesRegex(github_memory.CollectorError, "max artifacts"):
            lineage.collect(REPOSITORY, "issue", 3, 1, 0, client=client)
        with self.assertRaisesRegex(github_memory.CollectorError, "owner/name"):
            lineage.collect("bad/repo/name", "issue", 3, 1, 8, client=client)
        self.assertEqual(client.calls, [])


if __name__ == "__main__":
    unittest.main()

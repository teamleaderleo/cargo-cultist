#!/usr/bin/env python3
"""Enrich the landed active-work inventory with bounded divergent remote branches."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
from urllib.parse import quote

from active_work_heads_up_ci import build_inventory, graphql, page_info

BASE_REF = "origin/main"
MAX_BRANCH_CANDIDATES = 20
MAX_RECENT_CLOSED_PRS = 100

RECENT_CLOSED_PR_HEADS_QUERY = r"""
query($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    pullRequests(
      states: [CLOSED, MERGED]
      first: 100
      orderBy: {field: UPDATED_AT, direction: DESC}
    ) {
      nodes {
        number
        headRefName
        headRefOid
        mergedAt
        closedAt
      }
      pageInfo { hasNextPage endCursor }
    }
  }
}
"""


def git_text(args: list[str]) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def git_lines(args: list[str]) -> list[str]:
    output = subprocess.check_output(["git", *args], text=True)
    return [line.strip() for line in output.splitlines() if line.strip()]


def remote_ref_metadata() -> dict[str, tuple[str, str]]:
    records = git_lines(
        [
            "for-each-ref",
            "--format=%(refname:short)%09%(objectname)%09%(committerdate:iso-strict)",
            "refs/remotes/origin",
        ]
    )
    metadata: dict[str, tuple[str, str]] = {}
    for record in records:
        fields = record.split("\t")
        if len(fields) != 3:
            continue
        ref, sha, committed_at = fields
        metadata[ref] = (sha, committed_at)
    return metadata


def unmerged_remote_refs() -> set[str]:
    return set(
        git_lines(
            [
                "branch",
                "-r",
                "--no-merged",
                BASE_REF,
                "--format=%(refname:short)",
            ]
        )
    )


def recent_closed_pr_heads(repo: str) -> tuple[set[tuple[str, str]], bool]:
    owner, name = repo.split("/", 1)
    result = graphql(
        RECENT_CLOSED_PR_HEADS_QUERY,
        {"owner": owner, "name": name},
    )
    data = result.get("data")
    repository = data.get("repository") if isinstance(data, dict) else None
    pull_requests = (
        repository.get("pullRequests") if isinstance(repository, dict) else None
    )
    if not isinstance(pull_requests, dict):
        raise RuntimeError("could not retrieve recent closed pull requests")
    nodes = pull_requests.get("nodes")
    if not isinstance(nodes, list):
        raise RuntimeError("recent closed pull-request connection is missing nodes")

    heads = {
        (str(node["headRefName"]), str(node["headRefOid"]))
        for node in nodes
        if isinstance(node, dict)
        and node.get("headRefName")
        and node.get("headRefOid")
    }
    truncated, _cursor = page_info(pull_requests)
    return heads, truncated


def changed_paths(ref: str) -> list[str]:
    merge_base = git_text(["merge-base", BASE_REF, ref])
    return sorted(
        set(
            git_lines(
                [
                    "-c",
                    "core.quotepath=false",
                    "diff",
                    "--name-only",
                    "--no-renames",
                    merge_base,
                    ref,
                    "--",
                ]
            )
        )
    )


def branch_work_item(repo: str, ref: str, sha: str, committed_at: str) -> dict[str, object]:
    branch = ref.removeprefix("origin/")
    return {
        "id": f"branch:{branch}",
        "kind": "branch",
        "title": branch,
        "url": f"https://github.com/{repo}/tree/{quote(branch, safe='/')}",
        "head_ref": branch,
        "head_sha": sha,
        "updated_at": committed_at,
        "draft": False,
        "changed_paths": changed_paths(ref),
    }


def build_combined_inventory(repo: str, current_number: int) -> dict[str, object]:
    inventory = build_inventory(repo, current_number)
    active_work = inventory["active_work"]
    if not isinstance(active_work, list):
        raise RuntimeError("active_work must be a list")

    open_pr_heads = {
        str(work["head_ref"])
        for work in active_work
        if isinstance(work, dict) and work.get("kind") == "pull_request"
    }
    retired_exact_heads, retired_window_truncated = recent_closed_pr_heads(repo)

    metadata = remote_ref_metadata()
    unmerged = unmerged_remote_refs()
    candidates: list[tuple[str, str, str]] = []
    retired_exact_branch_heads_excluded = 0
    for ref in unmerged:
        if ref in {"origin/HEAD", BASE_REF} or not ref.startswith("origin/"):
            continue
        branch = ref.removeprefix("origin/")
        if branch in open_pr_heads:
            continue
        values = metadata.get(ref)
        if values is None:
            continue
        sha, committed_at = values
        if (branch, sha) in retired_exact_heads:
            retired_exact_branch_heads_excluded += 1
            continue
        candidates.append((ref, sha, committed_at))

    candidates.sort(key=lambda item: (item[2], item[0]), reverse=True)
    omitted = max(0, len(candidates) - MAX_BRANCH_CANDIDATES)
    selected = candidates[:MAX_BRANCH_CANDIDATES]

    branch_work = [
        branch_work_item(repo, ref, sha, committed_at)
        for ref, sha, committed_at in selected
    ]
    branch_work = [work for work in branch_work if work["changed_paths"]]

    inventory["source"] = (
        "github_pull_requests+git_remote_divergent_branches+recent_closed_pr_heads"
    )
    active_work.extend(branch_work)
    inventory["adapter_receipts"] = {
        "branch_base_ref": BASE_REF,
        "open_pr_heads_excluded": len(open_pr_heads),
        "recent_closed_pr_heads_seen": len(retired_exact_heads),
        "recent_closed_pr_head_limit": MAX_RECENT_CLOSED_PRS,
        "recent_closed_pr_head_window_truncated": retired_window_truncated,
        "retired_exact_branch_heads_excluded": retired_exact_branch_heads_excluded,
        "unmerged_non_pr_branch_candidates": len(candidates),
        "branch_candidates_returned": len(branch_work),
        "branch_candidates_omitted": omitted,
        "max_branch_candidates": MAX_BRANCH_CANDIDATES,
        "selection": "most recent commit timestamp first",
        "branch_name_similarity_used": False,
        "retirement_rule": (
            "exclude branch only when current branch name+head SHA exactly matches "
            "a recent closed/merged PR head; an advanced head becomes eligible again"
        ),
    }
    return inventory


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: divergent_branch_inventory.py OUTPUT.json")

    repo = os.environ["GITHUB_REPOSITORY"]
    current_number = int(os.environ["CURRENT_PR"])
    inventory = build_combined_inventory(repo, current_number)
    output = Path(sys.argv[1])
    output.write_text(json.dumps(inventory, indent=2) + "\n")

    receipts = inventory["adapter_receipts"]
    print(
        "branch inventory: "
        f"{receipts['branch_candidates_returned']} returned, "
        f"{receipts['branch_candidates_omitted']} omitted, "
        f"{receipts['open_pr_heads_excluded']} open-PR head(s) excluded, "
        f"{receipts['retired_exact_branch_heads_excluded']} exact closed-PR head(s) retired"
    )


if __name__ == "__main__":
    main()

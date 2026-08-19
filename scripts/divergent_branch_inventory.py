#!/usr/bin/env python3
"""Enrich active-work inventory with bounded provider-validated divergent branches."""

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
MAX_ASSOCIATED_PRS = 20


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


def lifecycle_query(candidates: list[tuple[str, str, str]]) -> str:
    fields = []
    for index, (ref, _sha, _committed_at) in enumerate(candidates):
        branch = ref.removeprefix("origin/")
        qualified = json.dumps(f"refs/heads/{branch}")
        fields.append(
            f"""
    b{index}: ref(qualifiedName: {qualified}) {{
      name
      target {{
        oid
        ... on Commit {{
          committedDate
          associatedPullRequests(first: {MAX_ASSOCIATED_PRS}) {{
            nodes {{
              number
              state
              mergedAt
              closedAt
              headRefName
              headRefOid
              url
            }}
            pageInfo {{ hasNextPage endCursor }}
          }}
        }}
      }}
    }}
"""
        )

    return (
        "query($owner: String!, $name: String!) {\n"
        "  repository(owner: $owner, name: $name) {\n"
        + "".join(fields)
        + "  }\n}"
    )


def provider_lifecycle(
    repo: str,
    candidates: list[tuple[str, str, str]],
) -> tuple[list[tuple[str, str, str]], dict[str, int]]:
    if not candidates:
        return [], {
            "provider_refs_missing": 0,
            "provider_head_mismatches": 0,
            "provider_open_pr_races": 0,
            "retired_exact_branch_heads_excluded": 0,
            "lifecycle_association_truncated_unknown": 0,
        }

    owner, name = repo.split("/", 1)
    result = graphql(
        lifecycle_query(candidates),
        {"owner": owner, "name": name},
    )
    data = result.get("data")
    repository = data.get("repository") if isinstance(data, dict) else None
    if not isinstance(repository, dict):
        raise RuntimeError("could not retrieve provider-current branch lifecycle")

    admitted: list[tuple[str, str, str]] = []
    receipts = {
        "provider_refs_missing": 0,
        "provider_head_mismatches": 0,
        "provider_open_pr_races": 0,
        "retired_exact_branch_heads_excluded": 0,
        "lifecycle_association_truncated_unknown": 0,
    }

    for index, candidate in enumerate(candidates):
        ref, local_sha, committed_at = candidate
        branch = ref.removeprefix("origin/")
        provider_ref = repository.get(f"b{index}")
        if not isinstance(provider_ref, dict):
            receipts["provider_refs_missing"] += 1
            continue

        target = provider_ref.get("target")
        if not isinstance(target, dict):
            receipts["provider_refs_missing"] += 1
            continue
        provider_sha = str(target.get("oid", ""))
        if provider_sha != local_sha:
            receipts["provider_head_mismatches"] += 1
            continue

        associated = target.get("associatedPullRequests")
        if not isinstance(associated, dict):
            receipts["lifecycle_association_truncated_unknown"] += 1
            continue
        nodes = associated.get("nodes")
        if not isinstance(nodes, list):
            receipts["lifecycle_association_truncated_unknown"] += 1
            continue

        exact_open = False
        exact_retired = False
        for node in nodes:
            if not isinstance(node, dict):
                continue
            if str(node.get("headRefName", "")) != branch:
                continue
            if str(node.get("headRefOid", "")) != provider_sha:
                continue
            state = str(node.get("state", ""))
            if state == "OPEN":
                exact_open = True
            elif state in {"CLOSED", "MERGED"}:
                exact_retired = True

        if exact_open:
            receipts["provider_open_pr_races"] += 1
            continue
        if exact_retired:
            receipts["retired_exact_branch_heads_excluded"] += 1
            continue

        truncated, _cursor = page_info(associated)
        if truncated:
            # We cannot prove that an exact retired PR is absent beyond the
            # bounded associated-PR page, so stay quiet about this branch.
            receipts["lifecycle_association_truncated_unknown"] += 1
            continue

        admitted.append((ref, provider_sha, committed_at))

    return admitted, receipts


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

    metadata = remote_ref_metadata()
    unmerged = unmerged_remote_refs()
    candidates: list[tuple[str, str, str]] = []
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
        candidates.append((ref, sha, committed_at))

    candidates.sort(key=lambda item: (item[2], item[0]), reverse=True)
    lifecycle_omitted = max(0, len(candidates) - MAX_BRANCH_CANDIDATES)
    lifecycle_candidates = candidates[:MAX_BRANCH_CANDIDATES]
    admitted, lifecycle_receipts = provider_lifecycle(repo, lifecycle_candidates)

    branch_work = [
        branch_work_item(repo, ref, sha, committed_at)
        for ref, sha, committed_at in admitted
    ]
    branch_work = [work for work in branch_work if work["changed_paths"]]

    inventory["source"] = (
        "github_pull_requests+git_remote_divergent_branches+"
        "provider_current_branch_lifecycle"
    )
    active_work.extend(branch_work)
    inventory["adapter_receipts"] = {
        "branch_base_ref": BASE_REF,
        "open_pr_heads_excluded": len(open_pr_heads),
        "unmerged_non_pr_branch_candidates": len(candidates),
        "lifecycle_candidates_selected": len(lifecycle_candidates),
        "lifecycle_candidates_omitted": lifecycle_omitted,
        "max_branch_candidates": MAX_BRANCH_CANDIDATES,
        "max_associated_prs_per_candidate": MAX_ASSOCIATED_PRS,
        **lifecycle_receipts,
        "branch_candidates_returned": len(branch_work),
        "selection": "most recent commit timestamp first before provider lifecycle",
        "branch_name_similarity_used": False,
        "provider_lifecycle": (
            "one batched provider-current branch-ref query; require provider head SHA "
            "to match local snapshot; inspect exact branch+head associated PRs; "
            "truncated association remains unknown"
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
        f"{receipts['lifecycle_candidates_omitted']} pre-lifecycle omitted, "
        f"{receipts['open_pr_heads_excluded']} open-PR head(s) excluded, "
        f"{receipts['retired_exact_branch_heads_excluded']} exact retired head(s), "
        f"{receipts['provider_head_mismatches']} stale local head(s), "
        f"{receipts['lifecycle_association_truncated_unknown']} lifecycle unknown"
    )


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Enrich the landed active-work inventory with bounded divergent remote branches."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
from urllib.parse import quote

from active_work_heads_up_ci import build_inventory

BASE_REF = "origin/main"
MAX_BRANCH_CANDIDATES = 20


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
    omitted = max(0, len(candidates) - MAX_BRANCH_CANDIDATES)
    selected = candidates[:MAX_BRANCH_CANDIDATES]

    branch_work = [
        branch_work_item(repo, ref, sha, committed_at)
        for ref, sha, committed_at in selected
    ]
    branch_work = [work for work in branch_work if work["changed_paths"]]

    inventory["source"] = "github_pull_requests+git_remote_divergent_branches"
    active_work.extend(branch_work)
    inventory["adapter_receipts"] = {
        "branch_base_ref": BASE_REF,
        "open_pr_heads_excluded": len(open_pr_heads),
        "unmerged_non_pr_branch_candidates": len(candidates),
        "branch_candidates_returned": len(branch_work),
        "branch_candidates_omitted": omitted,
        "max_branch_candidates": MAX_BRANCH_CANDIDATES,
        "selection": "most recent commit timestamp first",
        "branch_name_similarity_used": False,
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
        f"{receipts['open_pr_heads_excluded']} open-PR head(s) excluded"
    )


if __name__ == "__main__":
    main()

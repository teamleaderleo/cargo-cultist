#!/usr/bin/env python3
"""Build live PR inventory, run Cultist heads-up analysis, and render CI advice."""

from __future__ import annotations

import datetime
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

PR_PAGE_QUERY = r"""
query($owner: String!, $name: String!, $after: String) {
  repository(owner: $owner, name: $name) {
    pullRequests(
      states: OPEN
      first: 100
      after: $after
      orderBy: {field: UPDATED_AT, direction: DESC}
    ) {
      nodes {
        number
        title
        url
        headRefName
        headRefOid
        updatedAt
        isDraft
        files(first: 100) {
          nodes { path }
          pageInfo { hasNextPage endCursor }
        }
      }
      pageInfo { hasNextPage endCursor }
    }
  }
}
"""

PR_FILES_QUERY = r"""
query($owner: String!, $name: String!, $number: Int!, $after: String) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      files(first: 100, after: $after) {
        nodes { path }
        pageInfo { hasNextPage endCursor }
      }
    }
  }
}
"""


def gh_json(args: list[str]) -> object:
    output = subprocess.check_output(["gh", *args], text=True)
    return json.loads(output)


def graphql(query: str, variables: dict[str, object]) -> dict[str, object]:
    args = ["api", "graphql", "-f", f"query={query}"]
    for key, value in variables.items():
        if value is None:
            continue
        args.extend(["-F", f"{key}={value}"])
    result = gh_json(args)
    if not isinstance(result, dict):
        raise RuntimeError("unexpected GitHub GraphQL response")
    return result


def page_info(connection: dict[str, object]) -> tuple[bool, str | None]:
    info = connection.get("pageInfo")
    if not isinstance(info, dict):
        raise RuntimeError("GitHub connection is missing pageInfo")
    cursor = info.get("endCursor")
    return bool(info.get("hasNextPage", False)), str(cursor) if cursor else None


def file_paths_from_connection(connection: dict[str, object]) -> list[str]:
    nodes = connection.get("nodes")
    if not isinstance(nodes, list):
        raise RuntimeError("GitHub files connection is missing nodes")
    return [str(node["path"]) for node in nodes if isinstance(node, dict)]


def remaining_file_paths(
    owner: str,
    name: str,
    number: int,
    after: str | None,
) -> list[str]:
    paths: list[str] = []
    cursor = after
    while cursor is not None:
        result = graphql(
            PR_FILES_QUERY,
            {"owner": owner, "name": name, "number": number, "after": cursor},
        )
        data = result.get("data")
        repository = data.get("repository") if isinstance(data, dict) else None
        pull_request = (
            repository.get("pullRequest") if isinstance(repository, dict) else None
        )
        files = pull_request.get("files") if isinstance(pull_request, dict) else None
        if not isinstance(files, dict):
            raise RuntimeError(f"could not paginate files for PR #{number}")
        paths.extend(file_paths_from_connection(files))
        has_next, cursor = page_info(files)
        if not has_next:
            break
    return paths


def work_item(
    owner: str,
    name: str,
    node: dict[str, object],
) -> dict[str, object]:
    number = int(node["number"])
    files = node.get("files")
    if not isinstance(files, dict):
        raise RuntimeError(f"PR #{number} has no readable files connection")

    paths = file_paths_from_connection(files)
    has_next, cursor = page_info(files)
    if has_next:
        paths.extend(remaining_file_paths(owner, name, number, cursor))

    return {
        "id": f"#{number}",
        "kind": "pull_request",
        "title": str(node["title"]),
        "url": str(node["url"]),
        "head_ref": str(node["headRefName"]),
        "head_sha": str(node["headRefOid"]),
        "updated_at": str(node["updatedAt"]),
        "draft": bool(node.get("isDraft", False)),
        "changed_paths": sorted(set(paths)),
    }


def build_inventory(repo: str, current_number: int) -> dict[str, object]:
    owner, name = repo.split("/", 1)
    work: list[dict[str, object]] = []
    cursor: str | None = None

    while True:
        result = graphql(
            PR_PAGE_QUERY,
            {"owner": owner, "name": name, "after": cursor},
        )
        data = result.get("data")
        repository = data.get("repository") if isinstance(data, dict) else None
        pull_requests = (
            repository.get("pullRequests") if isinstance(repository, dict) else None
        )
        if not isinstance(pull_requests, dict):
            raise RuntimeError("could not retrieve open pull requests")
        nodes = pull_requests.get("nodes")
        if not isinstance(nodes, list):
            raise RuntimeError("open pull-request connection is missing nodes")
        work.extend(
            work_item(owner, name, node) for node in nodes if isinstance(node, dict)
        )
        has_next, cursor = page_info(pull_requests)
        if not has_next:
            break
        if cursor is None:
            raise RuntimeError("pull-request pagination promised another page without cursor")

    current = next((item for item in work if item["id"] == f"#{current_number}"), None)
    if current is None:
        raise RuntimeError(f"current PR #{current_number} was absent from open inventory")

    return {
        "schema_version": 1,
        "source": "github_pull_requests_graphql",
        "observed_at": datetime.datetime.now(datetime.timezone.utc)
        .isoformat()
        .replace("+00:00", "Z"),
        "current": current,
        "active_work": work,
    }


def potential_direct_overlap(inventory: dict[str, object]) -> bool:
    current = inventory["current"]
    active_work = inventory["active_work"]
    if not isinstance(current, dict) or not isinstance(active_work, list):
        raise RuntimeError("inventory work fields have unexpected types")

    current_paths = {str(path) for path in current["changed_paths"]}
    current_id = str(current["id"])
    for work in active_work:
        if not isinstance(work, dict):
            continue
        if str(work["id"]) == current_id:
            continue
        if current_paths.intersection(str(path) for path in work["changed_paths"]):
            return True
    return False


def quiet_summary(inventory: dict[str, object]) -> str:
    return "\n".join(
        [
            "## Cargo Cultist active-work heads-up",
            "",
            f"Observed `{inventory['observed_at']}` from `{inventory['source']}`.",
            "",
            "No direct active-work path overlap worth surfacing.",
            "",
            "> Advisory only. No semantic independence is inferred from disjoint paths.",
            "",
        ]
    )


def render_summary(report: dict[str, object]) -> str:
    heads_up = report["heads_up"]
    if not isinstance(heads_up, list):
        raise RuntimeError("heads_up report field must be a list")

    lines = [
        "## Cargo Cultist active-work heads-up",
        "",
        f"Observed `{report['observed_at']}` from `{report['source']}`.",
        "",
    ]

    if not heads_up:
        lines.append("No direct active-work path overlap worth surfacing.")
    else:
        lines.extend([f"**Heads up: {len(heads_up)} active overlap(s).**", ""])
        for item in heads_up:
            work = item["work"]
            if not isinstance(work, dict):
                raise RuntimeError("heads-up work field must be an object")
            lines.extend(
                [
                    f"### {work['id']} — {work['title']}",
                    "",
                    f"- Active head: [`{work['head_ref']}@{str(work['head_sha'])[:8]}`]({work['url']})",
                    f"- Updated: `{work['updated_at']}`",
                    "- Exact overlapping paths:",
                ]
            )
            for path in item["overlap_paths"]:
                lines.append(f"  - `{path}`")
            lines.extend(
                [
                    "- Interpretation: exact path overlap only; duplicate intent, ownership, incompatibility, and required coordination are not inferred.",
                    "",
                ]
            )

    omitted = int(report["omitted_heads_up"])
    if omitted:
        lines.append(f"Additional bounded heads-ups omitted: `{omitted}`.")

    lines.extend(
        [
            "",
            "> Advisory only. Inspect overlap if useful; continuing independently may be correct.",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> None:
    repo = os.environ["GITHUB_REPOSITORY"]
    current_number = int(os.environ["CURRENT_PR"])

    inventory_started = time.monotonic()
    inventory = build_inventory(repo, current_number)
    inventory_seconds = time.monotonic() - inventory_started

    current = inventory["current"]
    if not isinstance(current, dict):
        raise RuntimeError("current work item must be an object")
    active_work = inventory["active_work"]
    if not isinstance(active_work, list):
        raise RuntimeError("active_work must be a list")

    print(
        f"inventory: {len(active_work)} open PR(s), current #{current_number}, "
        f"{len(current['changed_paths'])} current path(s)"
    )

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not potential_direct_overlap(inventory):
        print(
            "No direct active-work path overlap worth surfacing. "
            "Rust analyzer skipped after exact provider-path prefilter."
        )
        print(f"timing: inventory {inventory_seconds:.2f}s; analyzer 0.00s")
        if summary_path:
            with Path(summary_path).open("a") as summary:
                summary.write(quiet_summary(inventory))
        return

    analyzer_started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="cultist-active-work-") as temporary:
        inventory_path = Path(temporary, "inventory.json")
        inventory_path.write_text(json.dumps(inventory, indent=2) + "\n")
        output = subprocess.check_output(
            [
                "cargo",
                "run",
                "--quiet",
                "--example",
                "active_work_heads_up",
                "--",
                str(inventory_path),
            ],
            text=True,
        )
    analyzer_seconds = time.monotonic() - analyzer_started

    report = json.loads(output)
    heads_up = report["heads_up"]
    if not isinstance(heads_up, list):
        raise RuntimeError("heads_up report field must be a list")

    print(
        f"examined: {report['candidates_examined']} active candidate(s); "
        f"self excluded: {report['self_candidates_excluded']}"
    )
    print(f"HEADS UP: {len(heads_up)} active overlap(s)")
    for item in heads_up:
        work = item["work"]
        print(
            f"  {work['id']} {work['title']} "
            f"[{str(work['head_sha'])[:8]} updated {work['updated_at']}]"
        )
        for path in item["overlap_paths"]:
            print(f"    overlaps {path}")
        print("    No duplicate intent or incompatibility inferred.")
    print(
        f"timing: inventory {inventory_seconds:.2f}s; "
        f"analyzer {analyzer_seconds:.2f}s"
    )

    if summary_path:
        with Path(summary_path).open("a") as summary:
            summary.write(render_summary(report))


if __name__ == "__main__":
    main()

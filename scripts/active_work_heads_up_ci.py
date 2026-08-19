#!/usr/bin/env python3
"""Build live PR inventory, run Cultist heads-up analysis, and render CI advice."""

from __future__ import annotations

import datetime
import json
import os
from pathlib import Path
import subprocess
import tempfile


def gh_json(args: list[str]) -> object:
    output = subprocess.check_output(["gh", *args], text=True)
    return json.loads(output)


def paginated_list(repo: str, endpoint: str) -> list[dict[str, object]]:
    pages = gh_json(
        [
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repo}/{endpoint}",
        ]
    )
    if not isinstance(pages, list):
        raise RuntimeError(f"unexpected paginated response for {endpoint}")
    return [item for page in pages for item in page]


def changed_paths(repo: str, number: int) -> list[str]:
    files = paginated_list(repo, f"pulls/{number}/files?per_page=100")
    return sorted({str(item["filename"]) for item in files})


def work_item(repo: str, pr: dict[str, object]) -> dict[str, object]:
    number = int(pr["number"])
    head = pr["head"]
    if not isinstance(head, dict):
        raise RuntimeError(f"PR #{number} has no readable head object")
    return {
        "id": f"#{number}",
        "kind": "pull_request",
        "title": str(pr["title"]),
        "url": str(pr["html_url"]),
        "head_ref": str(head["ref"]),
        "head_sha": str(head["sha"]),
        "updated_at": str(pr["updated_at"]),
        "draft": bool(pr.get("draft", False)),
        "changed_paths": changed_paths(repo, number),
    }


def build_inventory(repo: str, current_number: int) -> dict[str, object]:
    prs = paginated_list(repo, "pulls?state=open&per_page=100")
    current = next((pr for pr in prs if int(pr["number"]) == current_number), None)
    if current is None:
        fetched = gh_json(["api", f"repos/{repo}/pulls/{current_number}"])
        if not isinstance(fetched, dict):
            raise RuntimeError(f"could not fetch current PR #{current_number}")
        current = fetched

    return {
        "schema_version": 1,
        "source": "github_pull_requests",
        "observed_at": datetime.datetime.now(datetime.timezone.utc)
        .isoformat()
        .replace("+00:00", "Z"),
        "current": work_item(repo, current),
        "active_work": [work_item(repo, pr) for pr in prs],
    }


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

    inventory = build_inventory(repo, current_number)
    current = inventory["current"]
    if not isinstance(current, dict):
        raise RuntimeError("current work item must be an object")
    active_work = inventory["active_work"]
    if not isinstance(active_work, list):
        raise RuntimeError("active_work must be a list")

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

    report = json.loads(output)
    heads_up = report["heads_up"]
    if not isinstance(heads_up, list):
        raise RuntimeError("heads_up report field must be a list")

    print(
        f"inventory: {len(active_work)} open PR(s), current #{current_number}, "
        f"{len(current['changed_paths'])} current path(s)"
    )
    print(
        f"examined: {report['candidates_examined']} active candidate(s); "
        f"self excluded: {report['self_candidates_excluded']}"
    )
    if not heads_up:
        print("No direct active-work path overlap worth surfacing.")
    else:
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

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with Path(summary_path).open("a") as summary:
            summary.write(render_summary(report))


if __name__ == "__main__":
    main()

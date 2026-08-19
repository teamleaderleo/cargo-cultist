#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

import external_github_reference_guard as guard

REPOSITORY = "teamleaderleo/cultist"
EXTERNAL = "https://github.com/example/project/issues/123"
REDIRECT = "https://redirect.github.com/example/project/issues/123"
SAME_REPO = "https://github.com/teamleaderleo/cultist/issues/123"


def assert_violation(text: str) -> None:
    violations = guard.scan_text(
        text,
        source="fixture.md",
        current_repository=REPOSITORY,
    )
    assert len(violations) == 1, violations
    assert violations[0].url == EXTERNAL, violations


def test_direct_external_url_fails() -> None:
    assert_violation(f"See [{EXTERNAL}]({EXTERNAL}).")


def test_redirect_and_same_repo_pass() -> None:
    for text in [
        f"See [external]({REDIRECT}).",
        f"See [local]({SAME_REPO}).",
    ]:
        assert guard.scan_text(
            text,
            source="fixture.md",
            current_repository=REPOSITORY,
        ) == []


def test_code_examples_and_inline_code_pass() -> None:
    text = f"""Before.

```text
{EXTERNAL}
```

Literal `{EXTERNAL}` is evidence syntax here.
"""
    assert guard.scan_text(
        text,
        source="fixture.md",
        current_repository=REPOSITORY,
    ) == []


def test_local_evidence_marker_allows_next_or_same_line() -> None:
    next_line = f"""{guard.ALLOW_MARKER}
Exact source: {EXTERNAL}
"""
    same_line = f"Exact source: {EXTERNAL} {guard.ALLOW_MARKER}"
    for text in [next_line, same_line]:
        assert guard.scan_text(
            text,
            source="fixture.md",
            current_repository=REPOSITORY,
        ) == []


def test_marker_does_not_leak_past_one_line() -> None:
    text = f"""{guard.ALLOW_MARKER}
Exact source: {EXTERNAL}
Another source: {EXTERNAL}
"""
    violations = guard.scan_text(
        text,
        source="fixture.md",
        current_repository=REPOSITORY,
    )
    assert [(item.line, item.url) for item in violations] == [(3, EXTERNAL)]


def test_added_markdown_line_parser_ignores_deletions_and_context() -> None:
    diff = """diff --git a/doc.md b/doc.md
index 1111111..2222222 100644
--- a/doc.md
+++ b/doc.md
@@ -2,2 +2,3 @@
 context
-old
+new
+added
"""
    assert guard.added_markdown_lines(diff) == {"doc.md": {3, 4}}


def git(root: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()


def test_changed_markdown_uses_full_file_for_fence_context_but_only_added_lines() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        git(root, "init", "-q")
        git(root, "config", "user.email", "fixture@example.test")
        git(root, "config", "user.name", "Fixture")
        path = root / "doc.md"
        path.write_text(
            "Intro\n\n```text\ncanonical example\n```\n\nExisting prose.\n",
            encoding="utf-8",
        )
        git(root, "add", "doc.md")
        git(root, "commit", "-qm", "base")
        base = git(root, "rev-parse", "HEAD")

        path.write_text(
            f"Intro\n\n```text\ncanonical example\n{EXTERNAL}\n```\n\nExisting prose.\nNew prose: {EXTERNAL}\n",
            encoding="utf-8",
        )
        git(root, "add", "doc.md")
        git(root, "commit", "-qm", "change")

        violations = guard.scan_changed_markdown(
            base=base,
            root=root,
            current_repository=REPOSITORY,
        )
        assert [(item.source, item.line, item.url) for item in violations] == [
            ("doc.md", 9, EXTERNAL)
        ]


def test_pull_request_body_scanner_uses_same_rules() -> None:
    with tempfile.TemporaryDirectory() as directory:
        event_path = Path(directory) / "event.json"
        event_path.write_text(
            json.dumps(
                {
                    "pull_request": {
                        "body": f"Local: {SAME_REPO}\nExternal: {EXTERNAL}"
                    }
                }
            ),
            encoding="utf-8",
        )
        violations = guard.scan_pull_request_body(
            event_path=event_path,
            current_repository=REPOSITORY,
        )
        assert [(item.line, item.url) for item in violations] == [(2, EXTERNAL)]


def test_bad_event_rejects() -> None:
    with tempfile.TemporaryDirectory() as directory:
        event_path = Path(directory) / "event.json"
        event_path.write_text("{}", encoding="utf-8")
        try:
            guard.scan_pull_request_body(
                event_path=event_path,
                current_repository=REPOSITORY,
            )
        except guard.GuardError as error:
            assert "pull_request object" in str(error)
        else:
            raise AssertionError("expected GuardError")


def main() -> int:
    tests = [value for name, value in globals().items() if name.startswith("test_")]
    for test in sorted(tests, key=lambda value: value.__name__):
        test()
    print(f"external GitHub reference guard: {len(tests)} controls passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

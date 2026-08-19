#!/usr/bin/env python3
from __future__ import annotations

import os
import subprocess
import sys

import external_github_reference_guard as guard

REPOSITORY = "teamleaderleo/cultist"
EXTERNAL_URL = "https://github.com/example/project/issues/123"
EXTERNAL_REDIRECT = "https://redirect.github.com/example/project/issues/123"
EXTERNAL_SHORTHAND = "example/project#123"
OWNED_URL = "https://github.com/teamleaderleo/stensibly/issues/123"
OWNED_SHORTHAND = "teamleaderleo/stensibly#123"


def refs(text: str) -> list[str]:
    return [
        violation.url
        for violation in guard.scan_interaction_text(
            text,
            source="fixture",
            current_repository=REPOSITORY,
        )
    ]


def test_direct_third_party_url_rejects() -> None:
    assert refs(f"See {EXTERNAL_URL}.") == [EXTERNAL_URL]


def test_redirect_third_party_url_passes() -> None:
    assert refs(f"See {EXTERNAL_REDIRECT}.") == []


def test_non_linking_third_party_wording_passes() -> None:
    assert refs("See example/project issue 123.") == []


def test_third_party_shorthand_rejects() -> None:
    assert refs(f"See {EXTERNAL_SHORTHAND}.") == [EXTERNAL_SHORTHAND]


def test_owned_references_pass() -> None:
    assert refs(f"See {OWNED_URL} and {OWNED_SHORTHAND}.") == []


def test_interaction_code_block_is_not_an_escape() -> None:
    text = f"```text\n{EXTERNAL_URL}\n```"
    assert refs(text) == [EXTERNAL_URL]


def test_interaction_marker_is_not_an_escape() -> None:
    text = f"{guard.ALLOW_MARKER}\n{EXTERNAL_URL}"
    assert refs(text) == [EXTERNAL_URL]


def test_configured_owned_owner_extends_first_party_set() -> None:
    original = os.environ.get("CULTIST_OWNED_GITHUB_OWNERS")
    os.environ["CULTIST_OWNED_GITHUB_OWNERS"] = "example"
    try:
        assert refs(EXTERNAL_URL) == []
    finally:
        if original is None:
            os.environ.pop("CULTIST_OWNED_GITHUB_OWNERS", None)
        else:
            os.environ["CULTIST_OWNED_GITHUB_OWNERS"] = original


def test_cli_stdin_preflight_rejects_before_write() -> None:
    completed = subprocess.run(
        [
            sys.executable,
            "scripts/external_github_reference_guard.py",
            "--repository",
            REPOSITORY,
            "--stdin",
        ],
        input=f"Proposed body: {EXTERNAL_URL}\n",
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    assert completed.returncode == 1, completed
    assert EXTERNAL_URL in completed.stdout, completed.stdout
    assert "Do not rely on post-write CI" in completed.stdout, completed.stdout


def test_cli_stdin_preflight_accepts_redirect() -> None:
    completed = subprocess.run(
        [
            sys.executable,
            "scripts/external_github_reference_guard.py",
            "--repository",
            REPOSITORY,
            "--stdin",
        ],
        input=f"Proposed body: {EXTERNAL_REDIRECT}\n",
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    assert completed.returncode == 0, completed


def main() -> int:
    tests = [value for name, value in globals().items() if name.startswith("test_")]
    for test in sorted(tests, key=lambda value: value.__name__):
        test()
    print(f"external GitHub interaction preflight: {len(tests)} controls passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

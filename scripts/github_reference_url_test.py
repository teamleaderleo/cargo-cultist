#!/usr/bin/env python3

from github_reference_url import ReferenceUrlError, human_reference_url


def main() -> int:
    assert (
        human_reference_url("https://github.com/The-PR-Agent/pr-agent/issues/2184")
        == "https://redirect.github.com/The-PR-Agent/pr-agent/issues/2184"
    )
    assert (
        human_reference_url(
            "https://github.com/The-PR-Agent/pr-agent/pull/2424#discussion_r1"
        )
        == "https://redirect.github.com/The-PR-Agent/pr-agent/pull/2424#discussion_r1"
    )
    assert (
        human_reference_url(
            "https://github.com/teamleaderleo/cultist/issues/109",
            current_repository="teamleaderleo/cultist",
        )
        == "https://github.com/teamleaderleo/cultist/issues/109"
    )
    assert (
        human_reference_url("https://redirect.github.com/foo/bar/issues/1")
        == "https://redirect.github.com/foo/bar/issues/1"
    )
    assert (
        human_reference_url("https://api.github.com/repos/foo/bar/issues/1")
        == "https://api.github.com/repos/foo/bar/issues/1"
    )

    for bad in (
        "http://github.com/foo/bar/issues/1",
        "https://example.com/foo/bar/issues/1",
        "https://user@github.com/foo/bar/issues/1",
        "https://github.com:444/foo/bar/issues/1",
    ):
        try:
            human_reference_url(bad)
        except ReferenceUrlError:
            pass
        else:
            raise AssertionError(f"expected rejection for {bad}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

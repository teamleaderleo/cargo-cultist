#!/usr/bin/env python3
"""Render human-facing GitHub URLs without incidental external backlinks."""

from __future__ import annotations

import argparse
from urllib.parse import SplitResult, urlsplit, urlunsplit

GITHUB_HOST = "github.com"
REDIRECT_HOST = "redirect.github.com"
API_HOST = "api.github.com"


class ReferenceUrlError(ValueError):
    pass


def repository_from_path(path: str) -> str | None:
    components = [component for component in path.split("/") if component]
    if len(components) < 2:
        return None
    return f"{components[0]}/{components[1]}"


def human_reference_url(url: str, current_repository: str | None = None) -> str:
    """Rewrite an external github.com URL through redirect.github.com.

    Canonical API/provider URLs are left unchanged. A same-repository github.com
    URL is also left unchanged when current_repository is supplied, because that
    cross-reference is usually intentional.
    """

    parsed = urlsplit(url)
    if parsed.scheme != "https":
        raise ReferenceUrlError("GitHub reference URL must use https")
    if parsed.username is not None or parsed.password is not None or parsed.port is not None:
        raise ReferenceUrlError("GitHub reference URL may not contain userinfo or a port")

    host = (parsed.hostname or "").lower()
    if host in {REDIRECT_HOST, API_HOST}:
        return url
    if host != GITHUB_HOST:
        raise ReferenceUrlError("reference helper accepts only github.com URLs")

    repository = repository_from_path(parsed.path)
    if current_repository is not None and repository == current_repository:
        return url

    redirected = SplitResult(
        scheme="https",
        netloc=REDIRECT_HOST,
        path=parsed.path,
        query=parsed.query,
        fragment=parsed.fragment,
    )
    return urlunsplit(redirected)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(
        description="Rewrite a human-facing external GitHub URL through redirect.github.com."
    )
    result.add_argument("url")
    result.add_argument("--current-repository")
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        print(human_reference_url(args.url, args.current_repository))
    except ReferenceUrlError as error:
        raise SystemExit(f"github-reference-url: {error}") from error
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

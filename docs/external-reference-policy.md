# External GitHub reference policy

Cultist often preserves evidence from public GitHub issues, pull requests, reviews, and commits. Keep source identity exact without creating unnecessary cross-repository backlinks from human-facing GitHub conversations.

## Human-facing GitHub conversations

In issue bodies, pull-request bodies, and comments, use GitHub's backlink-avoiding host for external GitHub references:

```text
https://redirect.github.com/OWNER/REPOSITORY/issues/123
https://redirect.github.com/OWNER/REPOSITORY/pull/456
https://redirect.github.com/OWNER/REPOSITORY/commit/SHA
```

Do not use the cross-repository shorthand `OWNER/REPOSITORY#123` when a backlink is unnecessary; GitHub autolinks that form in conversations.

Same-repository references such as `#109`, `#137`, or `#198` may stay short when the cross-reference is intentional.

## Repository files

Research notes may prefer a non-linking source identity when click-through adds little value:

```text
`The-PR-Agent/pr-agent#2424`
`anthropics/claude-code#57507`
```

When a clickable external GitHub reference is useful in Markdown, use `redirect.github.com`.

## Machine and evidence boundaries

Keep canonical provider coordinates unchanged when they are part of the evidence or required by tooling:

```text
https://github.com/...
https://api.github.com/...
```

Examples include:

- GitHub API requests;
- exact provider URLs retained in JSON receipts;
- parser fixtures whose purpose is to recognize canonical GitHub syntax;
- workflow inputs consumed by GitHub tooling;
- source payloads copied verbatim as evidence.

Do not rewrite evidence merely to satisfy presentation hygiene. Apply the redirect rule at the human-facing rendering layer.

## Rule of thumb

```text
provider identity / machine input
  canonical URL

human-facing external GitHub link
  redirect.github.com

human-facing source mention with no click-through need
  literal owner/repo#number
```

The goal is precise provenance with fewer incidental backlinks, while keeping machine behavior and retained source evidence exact.

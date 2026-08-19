# Exact GitHub review-memory carrier

Issue #202 extends #198's local review-memory contract with one opt-in provider adapter. The adapter's job is deliberately narrow:

```text
selected GitHub inline-review comment
+ selected direct reply when the caller declares a resolved outcome
+ exact current PR head
-> existing ReviewMemoryQuery
-> existing Rust review-memory evaluator
```

It does not derive semantic concern identity or review outcome from arbitrary comment prose.

## Why provider coordinates belong in the contract

PR-Agent #2424 provides two complementary lessons.

The first is a useful review lifecycle. A resolved/outdated inline thread on `pr_agent/git_providers/github_provider.py` reports that persistent-comment fingerprints were recorded before GitHub accepted the review, so a 422 fallback retry could be skipped even though no comment had been published. The direct maintainer reply says the concern was fixed in commit `f699e7ea`.

The second is an identity counterexample. Another review thread reports that the GitHub path passed `target_line_no=None` into dedup fingerprints. Two same-file comments with the same normalized text/code on different lines could therefore collapse and one real concern could disappear. The review points at a concrete line/range and recommends preserving the already-derived absolute/inline anchor.

Those cases argue for keeping review semantics and provider coordinates separate:

```text
concern_key
  caller/producer-owned semantic lineage

provider identity
  exact comment / reply / PR / reviewed commit

applicability
  exact repository / work / revision / path

line receipts
  provider evidence retained for a later richer target dimension
```

V0 therefore uses exact path scope in `ReviewMemoryQuery` and preserves line/original-line fields in the provider receipt. It does not pretend path-only scope solves inline-comment identity forever.

## Adapter

The opt-in adapter is:

```text
scripts/review_memory_github.py
```

Example:

```bash
python scripts/review_memory_github.py \
  --repository The-PR-Agent/pr-agent \
  --pull-request 2424 \
  --comment-node-id PRRC_kwDOJ4EDks7IBoVk \
  --resolution-comment-node-id PRRC_kwDOJ4EDks7IB1zX \
  --concern-key pr-agent:persistent-inline-comments:github-fallback-publish-state \
  --outcome patch_changed \
  --output review-memory-query.json \
  --receipt-output review-memory-github-receipt.json

cargo run --quiet --example review_memory \
  < review-memory-query.json
```

The caller supplies `concern_key` and `outcome`. GitHub supplies the exact review coordinates.

## Exact mapping

For one selected root review comment:

```text
ReviewMemoryRecord.event_id
  github:pull/<PR>/review-comment/<numeric comment id>

source_ref
  same exact GitHub review-comment identity

subject.repository
  selected owner/repository

subject.work
  github:pull/<PR>

subject.revision
  root review comment commit_id

subject.scope
  exact root review comment path

current.revision
  current PR head SHA
```

When the caller selects a resolved outcome, the selected resolution comment must be a direct reply to the root. Its exact review-comment identity becomes `resolution_ref`.

This mapping intentionally does not use the comment body to generate `concern_key`, and it does not infer `patch_changed` from the presence of a reply. The body is retained only as bounded source evidence in the provider receipt.

## Provider receipt

The sidecar retains enough exact GitHub evidence to audit or refine the mapping later:

```text
root / resolution
  numeric id
  node_id
  pull_request_review_id
  commit_id
  original_commit_id
  path
  line / original_line
  start_line / original_start_line
  side / start_side
  in_reply_to_id
  created_at / updated_at
  bounded body

PR
  repository
  pull request number
  exact current head SHA
```

The line fields stay receipts rather than silently becoming a new applicability dimension in this slice.

## Bounds and failure policy

V0 admits:

```text
review comments scanned: <= 512
comment body retained:   <= 16 KiB each selected comment
review-memory query:     <= 256 KiB
provider receipt:        <= 256 KiB
```

Review comments are paginated completely up to the bound. Overflow fails instead of returning a partial inventory.

Validation rejects:

- malformed repository or PR identity;
- missing/ambiguous root selector;
- duplicate numeric IDs or node IDs;
- a root comment that is itself a reply;
- a selected comment whose `pull_request_url` points elsewhere;
- malformed current/comment/original Git SHAs;
- noncanonical repository-relative paths;
- a resolved outcome without a selected resolution reply;
- `open` with a resolution reply;
- a resolution comment that is not a direct reply to the selected root;
- oversized selected evidence or inventories.

## Live carrier: PR-Agent #2424

The automatic PR carrier selects:

```text
repository
  The-PR-Agent/pr-agent

pull request
  #2424

root node
  PRRC_kwDOJ4EDks7IBoVk
  “Dedup blocks gh fallback”

resolution reply node
  PRRC_kwDOJ4EDks7IB1zX
  reply names repair commit f699e7ea

caller outcome
  patch_changed
```

The carrier requires the root review `commit_id` to differ from the current PR head. The independent Rust evaluator must then return:

```text
disposition    refresh_existing_thread
match_kind     prior_head
applicability  invalid
outcome        patch_changed (historical evidence)
```

This is the first real provider replay of #198's central distinction: one review concern can retain thread lineage after the patch moves while its old outcome loses exact-head applicability.

## Standard controls

`scripts/review_memory_github_test.py` covers:

- node-ID and numeric-ID selection;
- exact query mapping from root/reply/current PR;
- direct-reply validation;
- root-is-reply rejection;
- cross-PR comment rejection;
- duplicate node-ID rejection;
- malformed SHAs and paths;
- outcome/resolution contracts before provider access;
- fail-closed comment inventory overflow.

The ordinary repository CI runs those controls. `.github/workflows/review-memory-github.yml` additionally runs the live public PR-Agent carrier and validates the emitted query through the independent Rust `review_memory` example.

## What this earns

Cultist can now keep these claims distinct in one real review lifecycle:

```text
OBSERVED provider fact
  review comment C was attached to exact PR/head/path

OBSERVED provider fact
  direct reply R exists on C

caller-supplied review outcome
  patch_changed

APPLICABILITY
  old reviewed head != current PR head -> INVALID

THREAD DELIVERY
  same concern lineage -> refresh existing thread
```

No step upgrades a resolved comment into project policy.

## Next discriminator

The #2424 line-anchor counterexample makes the next research target concrete: exact inline concern identity needs a stable target below path granularity. The next experiment should compare candidate provider coordinates such as original line/range, diff anchor, and semantic item identity across real head movement and retain the one that survives useful edits without collapsing distinct concerns.

Refs #18 #109 #123 #127 #137 #192 #198 #202.

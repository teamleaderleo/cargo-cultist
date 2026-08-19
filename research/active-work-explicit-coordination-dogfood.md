# Active-work explicit coordination dogfood

Date: 2026-08-19

## Question

Can Cultist's always-on PR advisory compose the reviewed explicit coordination metadata extractor with product `preflight --inventory` while preserving the cheap quiet path for unrelated work?

This follows the first producer-side phrase experiment in `research/explicit-coordination-metadata.md`.

## Live adapter split

One GitHub GraphQL snapshot now produces two deliberately separate views:

```text
product active-work inventory
  identity/title/url/head/update/draft/changed_paths

metadata extraction snapshot
  work/source/head/update/body
```

PR body text never enters the strict product `WorkItem`. Only typed `coordination_edges` emitted by the deterministic metadata extractor cross into the product inventory.

The existing `build_inventory(repo, current_number)` function remains available for the divergent-branch research adapter.

## Demand-driven execution

The common case checks cheap provider-normalized evidence first:

```text
no direct changed-path overlap
+ no raw `Do not merge while #` candidate line
-> skip coordination extractor
-> skip product preflight
-> quiet
```

A possible reviewed phrase runs the metadata extractor. Edges that do not involve the current work item do not interrupt it.

A direct path overlap or a typed edge involving current work invokes the landed product command:

```text
cargo cultist preflight --inventory FILE --format json
```

This removes the previous live-path dependence on a parallel research overlap renderer for actual interruptions: product preflight now owns both path-overlap and explicit-coordination semantics.

## Natural quiet control

PR #122 (`dogfood: compose explicit coordination metadata into PR heads-up`) ran concurrently with PR #121 (`research: prototype terse report rendering`).

At observation time:

```text
open PRs:      2
current:       #122
#122 paths:    2
#121 path:     src/render.rs
explicit hold: none
```

Normal CI run:

```text
run: 32238972232
job: 96025148426
```

The always-on step reported:

```text
inventory: 2 open PR(s), current #122, 2 current path(s)
No active-work coordination signal worth surfacing.
timing: inventory 1.32s; coordination 0.00s; product 0.00s
```

So adding PR body metadata to the provider snapshot did not force either the Rust metadata extractor or product analyzer to run for ordinary unrelated work.

## Public disjoint-path positive control

A temporary PR workflow replayed the pinned public Preflight #748/#703 metadata fixture through the **same Python helper functions used by the live adapter**.

Workflow:

```text
run: 32238972367
job: 96025148702
```

The replay deliberately supplied disjoint product paths:

```text
#748 -> src/source-only.txt
#703 -> src/target-only.txt
```

The metadata helper extracted exactly:

```json
{
  "kind": "hold_merge_while",
  "from": "#748",
  "to": "#703",
  "source": "github:pull/748"
}
```

with source receipt:

```text
head:       a2e14c4265e3568d8f943906a53e3b0e16dca141
updated_at: 2026-08-18T19:11:15Z
clause:     Do not merge while #703 is using current-main package evidence; ...
```

The helper then injected only the typed edge into a product-shaped inventory and invoked product preflight.

Required and observed result:

```text
1 x preflight-explicit-coordination
0 x preflight-inventory-path-overlap
OBSERVED: no direct path overlap
UNKNOWN: operational consequence / intent beyond declared relation
```

This is the coordination species the path-only advisory could not see.

## Failure boundary

If a raw reviewed-prefix candidate exists but deterministic metadata extraction fails, the adapter records that coordination metadata was not fully analyzed. Direct path evidence remains usable.

The adapter does not silently convert parser failure into proof that no coordination relation exists.

## What remains unchanged

- Advisory only; no merge/close/rebase/scheduling action.
- Only the reviewed `Do not merge while #N ...` phrase is eligible.
- No fuzzy intent scoring.
- No claim that a PR body as a whole equals current implementation intent.
- Source applicability remains bound to exact work/head/update evidence and UNKNOWN beyond the extracted clause.
- Bare branches remain outside the default live-work feed.

## Result

The first explicit project-metadata relation composes cleanly into the always-on PR advisory:

```text
path overlap
OR reviewed explicit coordination edge
-> product preflight finding

neither
-> cheap quiet result
```

The temporary positive-control workflow was removed after this receipt.

Refs #96, #101, #103, #105, #111, #120, #121, #122.

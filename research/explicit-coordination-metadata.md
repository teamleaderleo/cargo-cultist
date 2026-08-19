# Explicit coordination metadata research

Date: 2026-08-19

## Question

Can a provider-side adapter recover a tiny, high-precision coordination relation from concrete project-authored work metadata without turning ordinary references or arbitrary prose into scheduling policy?

This is the producer-side counterpart to product `preflight --inventory`, which already knows how to consume typed coordination edges.

The first experiment intentionally supports **one sentence form only**:

```text
Do not merge while #N ...
```

when it begins a top-level author line.

The emitted product edge is:

```json
{
  "kind": "hold_merge_while",
  "from": "#SOURCE",
  "to": "#N",
  "source": "github:pull/SOURCE"
}
```

No merge action or scheduling authority follows from extraction.

## Why start this narrowly

Project prose contains many relationships whose English sounds adjacent but whose operational meaning differs:

```text
Refs #N
Related: #N
Parent: #N
see #N
after discussing #N
may conflict with #N
competing experiment for #N
```

Those remain references unless a separate discriminator earns stronger semantics.

The extractor also ignores reviewed-looking phrases when they occur in:

- blockquotes;
- fenced code;
- indented examples;
- list-item examples.

This is intentionally conservative. Missing a coordination edge is preferable to silently converting illustrative or semantically adjacent prose into project scheduling evidence.

## Input contract

The research analyzer consumes a provider-supplied JSON snapshot rather than fetching GitHub itself.

Each admitted PR record binds:

- canonical work ID (`#` + positive decimal number);
- exact `github:pull/N` source identity;
- exact 40-hex head SHA;
- provider `updated_at` receipt;
- bounded body text.

Bounds:

```text
snapshot:       1 MiB
work items:     128
body per work:  128 KiB
line:           32 KiB
edges:          512
```

Unknown fields, unsupported schema/work kinds, duplicate work IDs, noncanonical source identity, and malformed head identity reject.

A referenced endpoint must exist in the admitted work set. Self references and unresolved endpoints do not become edges. Duplicate exact edges collapse.

## Public replay

The committed fixture `research/fixtures/preflight-748-hold.json` snapshots public `teamleaderleo/preflight` PR #748 at:

```text
head:       a2e14c4265e3568d8f943906a53e3b0e16dca141
updated_at: 2026-08-18T19:11:15Z
```

Its body ends with the top-level operative sentence:

```text
Do not merge while #703 is using current-main package evidence; this branch is intended to stay reviewable and green without moving the RC baseline underneath that gate.
```

Target #703 is present in the same admitted work set.

The integration replay requires exactly one edge:

```text
hold_merge_while #748 -> #703
source github:pull/748
```

and requires the source receipt to retain the exact #748 head, update coordinate, and matched clause.

The implementation head after formatting passed repository CI run `32238211815`, job `96022810449`, including:

- rustfmt;
- Clippy with warnings denied;
- the public replay and unit controls;
- the always-on active-work heads-up;
- full tests;
- repository/history/CI-test/diff dogfood.

## Negative controls

Unit controls stay quiet for ordinary reference language and for quoted/fenced/indented/list examples.

They also require:

- self-reference does not promote;
- unresolved target does not promote;
- duplicate exact clauses yield one edge;
- duplicate work IDs reject;
- mismatched `github:pull/N` source identity rejects;
- oversized input rejects before parsing.

No second phrase is admitted merely because it seems intuitively equivalent.

## Applicability boundary exposed by Cultist #111

Cultist PR #111 supplied an important self-dogfood counterexample while this research was being designed.

Its exact title/head/diff described an active-work prefilter identity repair, while its PR body described an unrelated repository warm-scan snapshot/cache feature.

That means:

> explicit project-authored prose can itself be stale, copied, or attached to the wrong current change coordinate.

This extractor therefore makes a deliberately smaller claim than “the body describes current implementation intent.”

A `source_receipt` records:

```text
source work ID
source head SHA
source updated_at
exact matched clause
```

and the report keeps broader/current applicability `UNKNOWN` without independent evidence.

For the first `hold_merge_while` form, the useful observed fact is simply:

```text
OBSERVED
  the admitted current metadata for PR A contains an explicit instruction
  to hold merge while admitted work B is active/relevant
```

Do not silently generalize that into:

```text
PR body == exact current implementation intent
```

A later provider adapter can invalidate/recompute the edge when source metadata/head coordinates move. Richer compact IR may eventually represent that validity directly.

## Composition

The intended path is:

```text
provider adapter
  -> bounded current PR metadata snapshot
  -> reviewed explicit-edge extractor
  -> existing coordination_edges field
  -> product preflight --inventory
  -> advisory question / heads-up
```

The local product core does not need GitHub credentials or a prose parser.

A cheap remote adapter can also prefilter bodies for the exact reviewed prefix and invoke the deterministic extractor only when a possible clause exists.

## What this does not establish

The experiment does not establish that:

- every `Do not merge while` sentence remains applicable forever;
- ordinary dependency/reference prose implies sequencing;
- semantic body/diff agreement can be scored safely;
- a hold edge means either change is wrong;
- the current worker must stop;
- Cultist should merge, close, rebase, or schedule work automatically.

Those remain separate evidence/decision questions.

## Next discriminator

The strongest next dogfood step is to enrich Cultist's existing GitHub active-work adapter with current PR body metadata and use this extractor to emit `coordination_edges` into the already-landed inventory contract.

The quiet path should remain cheap:

```text
no reviewed phrase prefix
  -> no metadata extractor / coordination work

possible exact prefix
  -> deterministic extraction
  -> product preflight composes path overlap + explicit edge evidence
```

A useful positive must have disjoint changed paths but an explicit edge, matching the #748/#703 species. Ordinary references must remain quiet.

Refs #96, #101, #103, #105, #111, #115.

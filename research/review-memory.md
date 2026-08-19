# Revision-aware review concern memory

Issue #192 tests one narrow part of #109's review-memory thesis:

> preserve that a concern has already been surfaced without letting an old review outcome silently become current after the patch moves.

The first carrier is research-only. It reuses the shared applicability evaluator from #123 and changes no ordinary `cargo-cultist` command or `AnalysisReport` schema.

## The distinction

V0 keeps three identities separate:

```text
review event identity
  one observed review delivery/outcome event

concern lineage identity
  producer-supplied key saying two deliveries are the same semantic concern/thread

applicability
  whether a prior outcome applies to the current repository/work/revision/scope
```

A `concern_key` is therefore useful for thread continuity, but grants no truth, policy, or suppression authority.

## Contract

`src/review_memory.rs` defines a bounded record:

```text
ReviewMemoryRecord
  event_id
  concern_key
  source_ref
  subject
    repository
    work
    exact reviewed revision
    optional path scope
  outcome
    open
    patch_changed
    rejected_with_evidence
    dismissed
  resolution_ref?
```

Resolved outcomes require an explicit `resolution_ref`. An open concern forbids one.

The current concern supplies the same producer-owned concern key plus the current applicability context.

Evaluation returns every same-key event with its ordinary #123 applicability receipt and one thread-delivery disposition:

```text
reuse_current_thread
  same concern and exact subject still APPLIES

refresh_existing_thread
  same concern, same repository/work/scope, reviewed head moved
  prior outcome is historical evidence with INVALID exact-head applicability

need_context
  required current coordinates are missing

new_thread
  no same-key event belongs to the current work/scope lineage
```

The evaluator does not choose a "latest" historical resolution from timestamps or vector order. Multiple prior events remain visible and are sorted only by event identity for deterministic rendering.

## External discriminator A: PR-Agent duplicate suggestions

`The-PR-Agent/pr-agent#2184` reports open-source PR-Agent running on every GitHub PR push. The reporter says repeated pushes can produce the same code suggestions again even after developers reply, thumbs-down, or resolve the earlier suggestion.

The issue discussion points to #2037, whose request is explicitly persistent inline comments to prevent duplicate suggestions across repeated runs.

PR #2424 later implements that feature. Its practical design is useful evidence:

```text
existing inline comments
-> hidden fingerprint markers
-> later run scans markers
-> matching fingerprint skips a duplicate post
```

The implementation fingerprints file + anchor line + normalized prose and separately file + anchor line + suggestion code, then matches either fingerprint.

That demonstrates real demand for stable cross-run concern identity. It also supplies an important counterexample.

## External discriminator B: broad identity can hide a real concern

Review of PR-Agent #2424 found a GitHub-path defect in the first implementation: fingerprints were computed with `target_line_no=None` even though an absolute line coordinate already existed. That made same-text comments on different lines in one file indistinguishable to the dedup layer and could incorrectly drop a real second concern.

Cultist's v0 reaction is conservative:

- concern identity is producer-owned, not inferred by fuzzy prose similarity;
- repository/work/scope remain separate applicability dimensions;
- a mismatched scope is `unrelated`, even if the caller reused the same concern key;
- line/symbol-level producers must encode their exact semantic target into the concern key until Cultist earns a richer target-applicability dimension.

This is why v0 does not implement a generic text-hash deduper.

## External discriminator C: old change state can become false

PR-Agent #179 records `/describe` using earlier commit-message history after a later commit reverted the described docstring change. The rendered PR description therefore claimed a change absent from the final diff.

PR-Agent #254 similarly records a rerun whose main-files walkthrough could remain stale after new changes, especially when a previously changed file was later removed.

These are direct controls against treating historical review/change state as current merely because it still exists in the conversation.

For review memory the corresponding invariant is:

```text
head moves
-> exact-head resolution applicability becomes INVALID
-> same concern may reuse/refresh thread identity
-> current concern must be recomputed
```

## Feedback is evidence, not automatic learning

PR-Agent #2075 asks for accepted suggestions and thumbs feedback to continuously teach future reviews. The request notes that existing feedback tracking can be used for statistics/manual evaluation without itself changing the model.

Cultist v0 retains explicit outcomes for inspection and future behavioral evaluation. It adds:

```text
no scalar feedback score
no automatic rule promotion
no fine-tuning loop
no cross-PR suppression from one dismissed comment
```

Repeated reviewed outcomes can later become evidence for #137 interruption economics or an explicit #10/#11 decision/rule promotion path.

## Retained fixture

The provider-shaped fixture is:

```text
research/review-memory/pr-agent-2184.json
```

The exact Git heads are deliberately synthetic because #2184 describes the behavior without publishing the affected PR/head coordinates.

The fixture retains only what the source supports:

```text
source_ref = github:issue/2184
same reported PR/work identity
same concern key
prior dismissed thread at synthetic HEAD A
current synthetic HEAD B
```

Expected evaluation:

```text
match_kind  prior_head
applicability INVALID
outcome     dismissed (historical)
disposition refresh_existing_thread
```

Run it with:

```bash
cargo run --quiet --example review_memory \
  < research/review-memory/pr-agent-2184.json
```

## Standard controls

`tests/review_memory.rs` requires:

- exact same head/work/scope -> reuse current thread;
- moved head -> refresh existing thread while old outcome is INVALID;
- missing current head -> need context / UNKNOWN;
- different work -> new thread;
- different scope -> new thread;
- missing required scope context -> need context even when the head also moved;
- multiple prior events remain visible without latest-event inference;
- unrelated concern keys do not create thread identity;
- empty memory -> new thread;
- duplicate and conflicting event IDs reject;
- reviewed/current revisions are exact lowercase 40-hex Git object IDs;
- resolved/open outcome references obey their state contract;
- retained PR-Agent fixture evaluates to refresh + invalid old resolution.

## Boundary and next discriminator

This does not yet ingest GitHub review comments automatically. A provider adapter needs a high-precision source for:

```text
review event identity
producer-owned concern key
exact reviewed head
exact work identity
scope/target coordinate
explicit outcome / resolution evidence
```

The next useful replay is a real merged PR with a review thread that changes state across at least two exact heads. That can test whether `refresh_existing_thread` reduces interruption while still reopening a concern when the patch invalidates its old resolution.

Refs #10 #11 #18 #109 #123 #127 #137 #147 #192.

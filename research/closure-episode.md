# Issue closure evidence versus failure clearance

Issue #207 tests one narrow archaeology/project-memory distinction:

```text
issue lifecycle state
  open / closed / state reason / closure event

failure clearance
  evidence that establishes the reported condition stopped applying
```

The first experiment intentionally refuses to derive the second from the first.

## Typed episode

`src/closure_episode.rs` defines a research-only `IssueClosureEpisode` containing:

```text
repository
prior issue snapshot
later issue snapshot
exact closure receipt
exact re-report receipt
optional duplicate-challenge receipts
```

The corresponding evaluator exposes:

```text
prior_state
later_state
closure_kind
re_report_observed
clearance
thread/work disposition
```

V0 has one clearance status:

```text
UNKNOWN
```

That is deliberate. This slice earns a way to say why an issue was closed and that it was later explicitly re-reported. It does not yet define a general evidence species that proves a failure has been repaired.

## Administrative inactivity is exact evidence

V0 recognizes only one administrative closure form:

```text
actor
  github-actions[bot]

evidence
  Closing for now — inactive for too long. Please [open a new issue](https://github.com/OWNER/REPO/issues/new/choose) if this is still relevant.
```

Both actor and exact repository-bound sentence are required.

`state_reason=not_planned` alone does not classify the closure as administrative inactivity. A human posting similar prose also does not gain the typed classification.

This keeps GitHub lifecycle metadata and project meaning separate.

## Re-report relation is explicit

The admitted provider relation is also intentionally narrow:

```text
**Re-reporting** the bug from #N ...
```

The exact source line is retained. The relation means only:

```text
the later issue explicitly says it re-reports #N
```

It does not prove that both reports have an identical root cause or that every detail of the earlier issue applies unchanged.

## External discriminator: Claude Code #31294 -> #57507

`anthropics/claude-code#31294` reports Task()-spawned subagents configured with the `memory:` frontmatter field failing to create or update `MEMORY.md`.

Its comment history preserves three useful events:

```text
comment 4008755017
  github-actions[bot] suggests three possible duplicates and announces possible auto-closure

comment 4008815491
  reporter Mationetap explains why each suggested issue is a different failure species

comment 4230270046
  github-actions[bot] closes for inactivity and tells users to open a new issue if still relevant
```

The issue is closed with `state_reason=not_planned`.

`#57507` later starts with:

```text
**Re-reporting** the bug from #31294 (closed-as-inactive 2026-04-11 by github-actions bot, not because it was fixed).
```

It adds a fresh reproduction and a candidate discriminator around explicit `tools:` allowlists. Its thread also contains an independent reproduction. It is later closed by the same inactivity sentence and also has `state_reason=not_planned`.

That makes the pair a useful control against treating `closed` as equivalent to “historical failure exhausted.”

## GitHub collector

The optional adapter is:

```text
scripts/closure_episode_github.py
```

Example:

```bash
python scripts/closure_episode_github.py \
  --repository anthropics/claude-code \
  --later-issue 57507 \
  --output closure-episode.json \
  --receipt-output closure-episode-github-receipt.json

cargo run --quiet --example closure_episode \
  < closure-episode.json
```

The adapter:

1. fetches the explicitly selected later issue;
2. requires exactly one admitted re-report line;
3. fetches only the named prior same-repository issue;
4. requires the prior issue to be closed;
5. reads prior issue comments completely up to the bound;
6. selects exactly one admitted administrative-inactivity bot comment;
7. optionally retains one explicit duplicate-suggestion + reporter-rejection pair;
8. emits the typed episode;
9. leaves clearance semantics to the Rust evaluator.

No title similarity or chronology search is used to discover the prior issue.

## Bounds

V0 admits:

```text
prior issue comments: <= 256
later issue body:     <= 64 KiB
selected comment evidence: <= 32 KiB each
episode:              <= 256 KiB
provider receipt:     <= 128 KiB
```

Pagination is complete-or-fail through the comment bound.

## Standard controls

Rust controls require:

- administrative inactivity + explicit re-report -> clearance UNKNOWN;
- closing the later re-report still leaves clearance UNKNOWN;
- `not_planned` alone does not create administrative-inactivity semantics;
- exact inactivity evidence requires the bot actor and exact repository-bound sentence;
- re-report evidence must name the declared prior issue;
- arbitrary `Related to #N` prose is not admitted as a re-report;
- prior issue must actually be closed;
- duplicate-challenge receipts do not change clearance;
- unknown machine fields fail closed.

Provider controls require:

- one exact re-report line before prior-provider work begins;
- exact inactivity comment from the bot;
- the same sentence from a human stays unclassified;
- open prior issue rejects;
- partial/ambiguous duplicate-challenge evidence rejects;
- comment inventory overflow fails instead of truncating;
- malformed repository/issue selection rejects before provider access.

## Live carrier expectation

`.github/workflows/closure-episode-github.yml` runs the public Claude Code episode on the research PR and validates it through the independent Rust example.

The expected projection is:

```text
prior_state       closed
later_state       closed
closure_kind      administrative_inactive
re_report_observed true
clearance         unknown
disposition       inspect_prior_failure
```

The important property is that closing #57507 again does not retroactively manufacture a clearing receipt for #31294.

### Executed receipt

PR #212 executed the live carrier successfully.

```text
workflow run
  32251752639

artifact
  9364678861

artifact digest
  sha256:922bc95b6fc63236226f15ffc65ef1ee6efc6524e7a14ff62e557d447beb0438

prior issue #31294
  created  2026-03-06T00:41:05Z
  closed   2026-04-11T22:11:48Z
  closed_by github-actions[bot]
  state_reason not_planned

closure comment
  4230270046
  github-actions[bot]
  administrative_inactive

later issue #57507
  created  2026-05-09T00:48:52Z
  closed   2026-06-09T11:10:27Z
  closed_by github-actions[bot]
  state_reason not_planned

re-report evidence
  **Re-reporting** the bug from #31294 (closed-as-inactive 2026-04-11 by github-actions bot, not because it was fixed). ...

duplicate challenge
  suggestion comment 4008755017 by github-actions[bot]
  rejection comment  4008815491 by Mationetap

prior issue comments scanned
  5
```

The independent Rust evaluation returned:

```text
prior_state        closed
later_state        closed
closure_kind       administrative_inactive
re_report_observed true
clearance          unknown
disposition        inspect_prior_failure
```

The sequence therefore demonstrates the exact distinction under test: lifecycle closure is established twice, while no clearing state is manufactured from either closure event.

## Product pressure test

The next behavioral question for #137 is small:

> When a fresh worker lands on closed #31294, does this episode send them to the later reproduction and earlier rejected-duplicate context before they repeat the same investigation or assume closure meant repair?

If it does, closure episodes become a candidate input to pre-edit/review evidence selection. If it does not change useful work, the evidence can remain queryable archaeology.

## Boundary

- no issue similarity model;
- no sentiment classifier;
- no automatic reopening or provider mutation;
- no claim that administrative closure is bad policy;
- no claim that the re-report proves identical root cause;
- no `fixed=false` field in the canonical episode;
- ordinary Cultist commands remain local and read-only.

Refs #16 #18 #62 #109 #137 #183 #188 #202 #206 #207.

# Behavioral receipt for the temporal prior-episode front

This note retains one observed research-plan change after the temporal-history composition landed.

Human-facing GitHub reference:

- [Cultist PR #237](https://redirect.github.com/teamleaderleo/cultist/pull/237)

## Observed sequence

The preceding Stensibly work had produced four independent temporal/evidence replay species:

```text
lesson promotion
proxy revision
observation reconciliation
proof-surface mismatch
```

After the fourth replay landed, another isolated historical species was a plausible next research action. The existing `prior_episode_front` composition seam was then inspected and used instead.

PR #237 composed the four fully observed dispositions into one caller-selected task-facing front:

```text
use_accepted_guard
use_corrected_predicate
await_bounded_convergence
produce_required_proof_artifact
```

That composed result was consulted. The next research action changed:

```text
before
  continue adding isolated temporal episode species

after
  move to behavioral-effect evaluation of selected prior-episode actions
```

The retained receipt is:

```text
research/behavioral-receipts/prior-episode-front-237.json
```

and binds the observation to exact merged revision:

```text
b3e80cfa7e0e238bb6f4aae9fd241d9d3ea4fef9
```

## Receipt semantics

The existing #137 `BehavioralReceipt` vocabulary is reused unchanged:

```text
delivery  surfaced
consulted true
outcome   changed_next_action
action    stop adding isolated temporal episode species and move to behavioral-effect evaluation of selected prior-episode actions
```

This records a concrete plan transition. It does not claim that the prior-episode front improved final task quality, caused every later decision, generalizes to another worker, or deserves product promotion.

The useful fact is smaller:

```text
selected project-memory composition was consulted
-> the immediate next research action changed
```

## Why this stays standalone

Open PR #213 currently owns the shared behavioral batch/summary files while adding another observed research-plan change.

This lane deliberately avoids those paths and adds only:

```text
research/behavioral-receipts/prior-episode-front-237.json
tests/prior_episode_front_behavioral_receipt.rs
research/prior-episode-front-behavioral-receipt.md
```

A later corpus integration can append this receipt after the active batch owner settles. The standalone receipt already uses the canonical schema and ordinary validator.

## Controls

The standard Rust suite requires:

- exact repository/revision/task/evidence coordinates;
- `delivery=surfaced`;
- `consulted=true`;
- `outcome=changed_next_action`;
- the concrete observed action;
- removing the action makes the receipt invalid under the existing #137 validator.

No new behavioral outcome or scoring model is introduced.

## Executed current-main receipt

The formatted semantic head was tested in the merge view of current `main@d18f3c1ebfc9b5c8d1a02ad6936e40008ff2997b`:

```text
head:       7d2024f2d8301d13d5c79f5c885b5d084b0a642f
merge view: 775df66371bc09a1d161a13f08d142db2598b7ac
CI:         32265476336 success
provenance: 32265476673 success
```

That run passed formatter, all-target Clippy with warnings denied, full tests, project-memory/review/closure/redirect controls, and normal Cultist repository/history/CI/diff dogfood. The provider-specific carriers skipped on their path filters; this lane performed no provider fetch.

## Next research question

The next stronger behavioral test should use a current task whose exact discriminator selects one prior temporal episode, then retain treatment/control receipts for whether the surfaced next action changes the work.

That requires an earned relevance-selection seam. Current open refinement work such as #231 demonstrates the useful pattern—source-owned exact discriminator/subject mapping—but this receipt lane does not stack on or copy those unpublished types.

Refs #41 #137 #213 #217 #219 #222 #229 #233 #236 #237.

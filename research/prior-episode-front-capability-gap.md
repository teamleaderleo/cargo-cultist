# Prior-episode front: selected action vs operational detail

The merged temporal `prior_episode_front` can select an already-observed historical disposition and put an exact next action at the front of task context.

The landed capability-demand retirement replay supplies a held-out Stensibly task that lets us ask a sharper question:

> Does the compact front carry enough operational detail to execute the selected historical action, or does the worker still need one deeper evidence acquisition step?

Human-facing Stensibly references use `redirect.github.com`:

- [#1571](https://redirect.github.com/teamleaderleo/stensibly/pull/1571)
- [#1573](https://redirect.github.com/teamleaderleo/stensibly/pull/1573)
- [#1575](https://redirect.github.com/teamleaderleo/stensibly/pull/1575)

## Held-out task

Current `main` retains:

```text
research/capability-demand-retirement/stensibly-convex-index-review-v1.json
```

The worker sees a proposed `convex/schema.ts` patch containing a new 68-character Convex index identifier and is asked to review merge eligibility against existing repository conventions and production constraints.

The evaluator-only oracle records:

```text
blocking reason         convex_index_identifier_limit
max identifier length   64
proposed length          68
corrective action        shorten_identifier_preserve_field_order
```

The worker prompt deliberately omits those oracle facts.

## Source lesson evidence

The retained lesson-promotion claim still carries the exact source-owned repair marker:

```text
64-character identifier limit
```

and evaluates to the observed merged common guard from #1575.

So the source history knows the operational threshold.

## Task-facing front

The one-item temporal front correctly preserves:

```text
next                    use_accepted_guard
candidate value         index_identifier_limit
enforcement path        test/convex-index-identifier-limit.test.ts
scope                    convex/**/*.ts
guard                    PR #1575
same-class repairs       PR #1571, PR #1573
automatic authority      false
```

But the serialized front does not carry:

```text
64-character identifier limit
max_identifier_length
corrective_action
```

`tests/prior_episode_front_capability_gap.rs` proves both sides from current repository fixtures:

```text
source lesson claim contains exact 64-character marker
-> selected front says use_accepted_guard / index_identifier_limit
-> compact front omits the operational threshold and corrective action
-> held-out oracle still requires 64 / shorten_identifier_preserve_field_order
```

## Meaning

This is a useful negative result about task-facing memory compression.

Selection and action projection succeeded. Operational execution detail remains a separate evidence need.

That suggests a later behavioral treatment should avoid either extreme:

```text
A. eagerly attach full historical source text to every front item
B. treat use_accepted_guard as self-executing
```

A stronger composition is:

```text
selected prior episode
-> exact next action
-> identify missing guard detail
-> bounded acquisition of accepted guard evidence
-> worker action
-> behavioral receipt
```

The current front can stay compact. Missing detail can be acquired only when the selected action requires it.

## Executed current-main receipt

The formatted semantic head was tested against `main@d18f3c1ebfc9b5c8d1a02ad6936e40008ff2997b`:

```text
head:       636eebe6f0b04a605f2336954589cc9a21d5d384
CI:         32265811976 success
provenance: 32265811717 success
```

The ordinary CI run passed formatter, strict all-target Clippy, the operational-detail gap test, full tests, project-memory/review/closure/redirect controls, and normal Cultist repository/history/CI/diff dogfood. Provider-specific carriers skipped on their path filters.

`main` subsequently advanced through the standalone #242 behavioral receipt. This note update is the only change after the semantic head; its final merge-view pass therefore tests the same two-file negative control on the newest base.

## Active trial ownership

Two new research owners appeared after this probe started:

- [#245](https://redirect.github.com/teamleaderleo/cultist/pull/245) owns blindable paired behavioral-trial plans/materialization/reconciliation for prior-episode-front treatments.
- [#246](https://redirect.github.com/teamleaderleo/cultist/pull/246) owns externally executable capability-demand worker run receipts and pair interpretation.

This lane does not add a competing trial plan, run receipt, worker harness, or model execution surface. The gap result can inform those owners: a `use_accepted_guard` treatment may still need exact accepted-guard detail before execution.

Open [#231](https://redirect.github.com/teamleaderleo/cultist/pull/231) demonstrates a related exact-mapping principle in a different refinement lane. This experiment does not stack on or copy its unpublished types.

The merged `main` also has no `src/evidence_acquisition.rs` module, so this lane stops at the negative result instead of importing an unpublished planner stack.

## Boundary

- test + research note only;
- no change to `prior_episode_front` output schema;
- no claim that a worker necessarily fails with the compact front;
- no model run;
- no provider/network call;
- no automatic policy authority;
- no eager source-prose attachment;
- no competing paired-trial or run-receipt implementation.

The result is narrow: the currently serialized front does not itself contain the held-out task's explicit operational threshold/corrective-action fields, even though the retained source lesson does.

Refs #41 #137 #145 #164 #217 #219 #222 #231 #237 #242 #245 #246.

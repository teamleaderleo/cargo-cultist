# Prior-episode front: selected action vs operational detail

The merged temporal `prior_episode_front` can now select an already-observed historical disposition and put an exact next action at the front of task context.

The newly landed capability-demand retirement replay supplies a held-out Stensibly task that lets us ask a sharper question:

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

That suggests the next behavioral treatment should avoid either extreme:

```text
A. dump full historical source text into every front item
B. assume use_accepted_guard is self-executing
```

A stronger composition is:

```text
selected prior episode
-> exact next action
-> identify missing guard detail
-> bounded acquisition of the accepted guard evidence
-> worker action
-> behavioral receipt
```

The current front should remain compact. The missing detail can be acquired only when the selected action requires it.

## Relationship to current research

Open #231 demonstrates a related design principle in a different lane: source-owned exact mappings bind selected refinement needs to exact observation subjects. This experiment does not stack on or copy those unpublished types.

The existing #145/#164 evidence-acquisition work is the more natural future composition point for fetching the selected guard detail after the front identifies `use_accepted_guard`.

## Boundary

- test + research note only;
- no change to `prior_episode_front` output schema;
- no claim that a worker necessarily fails with the compact front;
- no model run;
- no provider/network call;
- no automatic policy authority;
- no requirement that every prior episode attach source prose eagerly.

The result is narrower: the currently serialized front does not itself contain the held-out task's explicit operational threshold/corrective-action fields, even though the retained source lesson does.

Refs #41 #137 #145 #164 #217 #219 #222 #231 #237.

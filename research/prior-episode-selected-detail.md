# Selected prior-episode detail

Merged #237 can put one exact historical next action at the front of task context. Merged #244 then proved a useful compression boundary: for the Stensibly Convex index-limit episode, the compact front says `use_accepted_guard` and preserves the failure class, guard path, scope, and source coordinates, while the explicit operational threshold remains in the admitted source lesson.

This experiment adds a post-selection detail projector for that exact gap.

Human-facing source refs use `redirect.github.com`:

- [Cultist #237](https://redirect.github.com/teamleaderleo/cultist/pull/237)
- [Cultist #244](https://redirect.github.com/teamleaderleo/cultist/pull/244)
- [Stensibly #1571](https://redirect.github.com/teamleaderleo/stensibly/pull/1571)
- [Stensibly #1573](https://redirect.github.com/teamleaderleo/stensibly/pull/1573)
- [Stensibly #1575](https://redirect.github.com/teamleaderleo/stensibly/pull/1575)

## Contract

`src/prior_episode_detail.rs` consumes one already-admitted `PriorEpisodeInput`.

It first runs that exact input through the existing `evaluate_prior_episode_front()` path. Detail is available only after the source episode still earns its actionable front disposition.

V1 supports one case:

```text
lesson_promotion
+ front next = use_accepted_guard
-> accepted_guard detail
```

Other prior-episode kinds reject explicitly until a concrete execution-detail need is demonstrated for them.

The accepted-guard detail contains only selected source-owned fields:

```text
packet-local id
next = use_accepted_guard
candidate discriminator/value
exact operational marker
guard PR coordinate
guard marker
guard source evidence
enforcement kind/path/scope
same-class repair refs
automatic_policy_authority = false
```

It does not copy the full `ProjectMemoryPacket` into worker-facing output and does not carry adjacent different-class predecessor evidence.

Serialized detail is capped at 32 KiB after construction. The source claim already bounds individual evidence excerpts before this projector runs.

## Real Stensibly carrier

The retained lesson-promotion input contains:

```text
operational marker
  64-character identifier limit

guard source evidence
  ... fails when any exceed 64 characters ...

guard
  PR #1575

enforcement path
  test/convex-index-identifier-limit.test.ts

scope
  convex/**/*.ts

same-class repairs
  PR #1571
  PR #1573
```

The adjacent #1569 `node:crypto` production failure remains in the full project-memory/lesson evidence but is excluded from the selected detail packet.

This is the intended compression behavior:

```text
full historical packet
-> selected actionable episode
-> exact operational detail for that action
```

instead of:

```text
full historical packet
-> copy every neighboring event into treatment context
```

## Held-out capability-demand relation

Current `main` retains the non-leaky Stensibly review trial where a proposed index identifier is 68 characters and the evaluator-only oracle records the production maximum as 64.

The compact front alone omits the explicit 64-character marker. This selected detail restores the exact source-owned threshold text without reading the evaluator oracle:

```text
64-character identifier limit
fails when any exceed 64 characters
```

The detail projector does not invent a normalized `max_identifier_length` field or the oracle's `corrective_action`. The accepted source evidence stays source text; a worker may inspect/count the proposed identifier against that rule.

## Fail-closed controls

`tests/prior_episode_detail.rs` requires:

- the retained Stensibly lesson still evaluates to `use_accepted_guard` before detail projection;
- detail contains the exact 64-character marker and accepted guard evidence;
- detail preserves #1571/#1573 and #1575;
- detail excludes #1569, `node_runtime_bundle`, and `node:crypto`;
- removing #1573 from guard coverage makes the source front incomplete and detail projection reject;
- a fully observed proxy-revision input still rejects because v1 has no demonstrated proxy-detail contract yet.

## Replay

`examples/prior_episode_detail.rs` accepts one ordinary prior-episode-front query on stdin, requires exactly one selected input, and prints the bounded selected detail:

```text
cargo run --example prior_episode_detail < SELECTED_PRIOR_EPISODE.json
```

No provider/network access occurs.

## Relationship to behavioral trials

Merged [#245](https://redirect.github.com/teamleaderleo/cultist/pull/245) owns blindable paired behavioral-trial mechanics. Its worker packet fingerprints an arbitrary exact treatment context; it intentionally does not decide how that context was assembled.

This projector sits one layer earlier:

```text
selected historical input
-> source-owned operational detail
-> treatment context assembly
-> #245 trial packet
```

Open [#246](https://redirect.github.com/teamleaderleo/cultist/pull/246) owns capability-demand worker run receipts and pair interpretation. This lane adds no worker execution or result semantics.

Open [#164](https://redirect.github.com/teamleaderleo/cultist/pull/164) remains the broader evidence-planner research lane, stacked on unpublished durable-obligation work. This experiment does not import that planner or its types.

## Boundary

- research only;
- one demonstrated detail species in v1;
- no relevance discovery;
- no similarity search;
- no provider/network call;
- no model/worker invocation;
- no front schema expansion;
- no normalized provider-rule ontology;
- no automatic policy authority or external effect;
- no competing trial/run-receipt evaluator.

The projector answers one narrow question after selection: which already-admitted source evidence is needed to execute the chosen historical action?

Refs #41 #137 #164 #217 #219 #222 #237 #244 #245 #246.

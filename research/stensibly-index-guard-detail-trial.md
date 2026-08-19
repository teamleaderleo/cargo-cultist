# Blind first-action trial: compact front vs selected guard detail

Merged #245 gives Cultist a blindable paired behavioral-trial protocol. Merged #249 gives the temporal prior-episode path one source-owned post-selection detail packet for the Stensibly Convex index-limit guard.

Merged #254 freezes the first paired plan that composes those two primitives.

Human-facing references use `redirect.github.com`:

- [Cultist #245](https://redirect.github.com/teamleaderleo/cultist/pull/245)
- [Cultist #249](https://redirect.github.com/teamleaderleo/cultist/pull/249)
- [Cultist #254](https://redirect.github.com/teamleaderleo/cultist/pull/254)
- [Stensibly #1571](https://redirect.github.com/teamleaderleo/stensibly/pull/1571)
- [Stensibly #1573](https://redirect.github.com/teamleaderleo/stensibly/pull/1573)
- [Stensibly #1575](https://redirect.github.com/teamleaderleo/stensibly/pull/1575)

## Question

For the exact held-out Stensibly patch already retained by the capability-demand retirement research, does adding selected accepted-guard detail change the worker's first justified action beyond the compact prior-episode front alone?

The task stays:

```text
repository  teamleaderleo/stensibly
revision    85cecf2608ad9e734a67518577fa85b9a08a550c
target      convex/schema.ts
blob        7fdd51e2f9fba80d1c0a814cea708d601a7b9925
```

and proposes the same index:

```text
by_project_issue_revision_instruction_set_sha256_provider_updated_at
```

The worker is asked to choose exactly one first justified action before writing review feedback.

## Fixed action vocabulary

Both arms receive the same four actions:

```text
block_and_shorten_identifier
inspect_accepted_guard_detail
approve_patch
inspect_more_repository_context
```

The vocabulary supplies candidate actions to both arms equally; it carries no arm label or success annotation.

## Control: compact front only

The control context contains the exact patch plus the selected compact prior-episode facts:

```text
next                    use_accepted_guard
candidate discriminator convex_production_failure_class
candidate value         index_identifier_limit
guard                    pull_request#1575
enforcement path         test/convex-index-identifier-limit.test.ts
scope                    convex/**/*.ts
same-class repairs       pull_request#1571, pull_request#1573
automatic authority      false
```

The explicit threshold text is absent from control:

```text
64-character identifier limit
fails when any exceed 64 characters
```

## Treatment: same bytes plus selected detail

The treatment context starts with the complete control context byte-for-byte and appends only:

```text
Cultist selected accepted-guard detail
operational marker
  64-character identifier limit

guard marker
  two historical mail index names

guard source evidence
  exact #1575 source evidence, including
  "fails when any exceed 64 characters"
```

Evaluator-only capability-demand oracle fields remain absent from both arms:

```text
max_identifier_length
corrective_action
```

The decisive rule comes from the admitted historical source detail, independent of the retirement oracle.

Worker-visible context sizes are frozen and asserted in the ordinary test suite:

```text
control    1082 bytes
treatment  1775 bytes
```

## Post-selection contrast vs the retirement leak control

This plan reuses the **task and patch** from the capability-demand retirement corpus, while asking a later question after project-memory selection has already happened.

The original retirement experiment prohibited worker-visible fragments such as `#1571`, `#1573`, `64`, and `identifier limit` so it could test whether broader scoped evidence retired a capability demand.

This trial intentionally starts later:

```text
both arms
  compact selected prior episode
  #1571 / #1573 / #1575 coordinates
  use_accepted_guard
  index_identifier_limit

treatment only
  exact accepted-guard operational detail
```

The isolation rule for this question is byte-level: every worker-visible byte before the selected-detail suffix is identical across arms.

## Exact fingerprints

Registered plan:

```text
cultist-behavioral-trial-plan-sha256-v1:6f3eddecf177271c0ad60f32fb17008841bdb81f34aa717f52be90c3bdd1f69b
```

Control worker packet:

```text
cultist-behavioral-worker-packet-sha256-v1:9949665d13b162692ebd3f7d12b6f162881f18d2d381c7257100bbb89c317f01
```

Treatment worker packet:

```text
cultist-behavioral-worker-packet-sha256-v1:6d3d93c574e81800ef1829216356d258553cabc7d2bdce2d09c32f46659c738c
```

The materialized worker packet omits organizer-only `context_ref` / arm identity under the existing #245 contract.

## Controls

`tests/behavioral_trial_index_guard_detail.rs` requires:

- known plan and worker-packet fingerprints;
- identical task/action vocabulary across arms;
- exact 1082/1775 worker-visible context sizes;
- treatment starts with the exact complete control context;
- the only worker-visible suffix is selected accepted-guard detail;
- both arms carry the same compact front and proposed patch;
- only treatment carries the exact 64-character marker and guard source evidence;
- neither arm carries normalized retirement-oracle field names;
- treatment detail matches the current merged `project_prior_episode_detail()` result;
- worker packets omit organizer `context_ref` identifiers;
- one-byte treatment context drift invalidates the registered digest;
- reversed synthetic observation order still maps actions to the correct arms.

The synthetic pair uses:

```text
control first action
  inspect_accepted_guard_detail

treatment first action
  block_and_shorten_identifier
```

only to verify pair mapping and `same_first_action=false`. It is never represented as a real worker result or evidence that the treatment improves behavior.

## Executed semantic receipt

The final semantic head merged in #254 was:

```text
head:       c9fccae8877ba6a1f684b803420943e3d1946b79
CI:         32268660788 success
provenance: 32268660723 success
```

That run passed formatter, strict all-target Clippy, exact fingerprint/context-isolation tests, full tests, project-memory/review/closure controls, external GitHub reference controls, the pull-request redirect guard, and normal Cultist repository/history/CI/diff dogfood. Provider-specific carriers skipped on their path filters.

The frozen plan merged as:

```text
0eabd231a23dfc243c167d336b7ad9836aff2bcc
```

The durable receipt text missed that merge because the PR merged while the note-only commit was being written. This follow-up changes only this note and validates the already-merged plan on current main, which also contains the later source-owned discriminator-observation work and the capability-demand frozen-manifest binding repair.

## Execution boundary

Merged [#246](https://redirect.github.com/teamleaderleo/cultist/pull/246) owns capability-demand run receipts for its registered retirement protocol. This #245 plan has its own generic observation/reconciliation format and still requires genuinely independent worker executions before any descriptive treatment/control pair exists.

A later runner may materialize both blind packets from this registered plan and retain actual `BehavioralTrialObservation` records. Cultist should continue to report only the observed first-action IDs and `same_first_action`; interpretation can remain separate.

## Boundary

- research only;
- no provider/network call;
- no model/worker invocation;
- no synthetic observation represented as real evidence;
- no treatment-effect or causal claim;
- no analyzer ranking or promotion;
- no ordinary product CLI/report change;
- no change to #245 trial protocol or #249 selected-detail schema.

This isolates one narrower question than capability success: does the selected source detail change what the worker does first?

Refs #41 #137 #179 #187 #217 #219 #237 #244 #245 #246 #249 #254.

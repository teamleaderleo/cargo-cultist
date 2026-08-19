# Blind first-action trial: compact front vs selected guard detail

Merged #245 gives Cultist a blindable paired behavioral-trial protocol. Merged #249 gives the temporal prior-episode path one source-owned post-selection detail packet for the Stensibly Convex index-limit guard.

Merged #254 froze the first paired plan that composed those primitives. Issue #258 found that its shared action vocabulary leaked the desired correction before any real worker execution. This note keeps that historical receipt while registering the repaired neutral-vocabulary plan as the executable version.

Human-facing references use `redirect.github.com`:

- [Cultist #245](https://redirect.github.com/teamleaderleo/cultist/pull/245)
- [Cultist #249](https://redirect.github.com/teamleaderleo/cultist/pull/249)
- [Cultist #254](https://redirect.github.com/teamleaderleo/cultist/pull/254)
- [Cultist #258](https://redirect.github.com/teamleaderleo/cultist/issues/258)
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

## Neutral fixed action vocabulary

Both arms receive the same four actions:

```text
block_patch
inspect_accepted_guard_detail
approve_patch
inspect_more_repository_context
```

`block_patch` is labeled only:

```text
Block the patch as currently proposed
```

The shared task/action vocabulary contains none of the explicit downstream correction markers:

```text
shorten
64
identifier limit
preserving field order
```

The superseded #254 plan used `block_and_shorten_identifier` and a label that explicitly told the worker to shorten the identifier while preserving field order. That wording weakened the intended intervention by giving the control arm corrective content outside its context. No real worker observation was executed under that plan, so the vocabulary was repaired before evidence collection.

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

The explicit operational threshold and guard-source sentence remain absent from the control context.

## Treatment: same bytes plus selected detail

The treatment starts with the complete control context byte-for-byte and appends only the selected accepted-guard detail:

```text
operational marker
  64-character identifier limit

guard marker
  two historical mail index names

guard source evidence
  exact #1575 source evidence including
  "fails when any exceed 64 characters"
```

No normalized capability-demand oracle fields are copied into either context:

```text
max_identifier_length
corrective_action
```

Worker-visible context sizes remain unchanged:

```text
control    1082 bytes
treatment  1775 bytes
```

## Post-selection contrast vs the retirement leak control

This plan reuses the task and patch from the capability-demand retirement corpus while asking a later question after project-memory selection has already happened.

Both arms intentionally know the selected historical coordinates and compact class:

```text
#1571 / #1573 / #1575
use_accepted_guard
index_identifier_limit
```

Treatment alone receives the exact accepted-guard operational detail. The isolation rule is byte-level: every worker-visible context byte before the selected-detail suffix is identical across arms, and the shared action vocabulary is now correction-neutral.

## Exact repaired fingerprints

The existing #245 typed materializer regenerated these identities from the neutral plan:

Registered plan:

```text
cultist-behavioral-trial-plan-sha256-v1:1aca6332c77ed72b49cb20593f215c7eb2952121ad9bf3d5ae60bea0df5df024
```

Control worker packet:

```text
cultist-behavioral-worker-packet-sha256-v1:5fa460fe007276013ed019830daaf9fc8d086cc5d3d5dfdfc5dd33a58052887d
```

Treatment worker packet:

```text
cultist-behavioral-worker-packet-sha256-v1:a80303b59b9a46c5f4e6adb446abb01fbaeb5d898911bf6ad1248b3b0cf38549
```

The materialized worker packets still omit organizer-only `context_ref` / arm identity under the #245 contract. The blind-input carrier separately checks that the shared task/action vocabulary contains none of the leaked correction markers before emitting artifacts.

## Controls

`tests/behavioral_trial_index_guard_detail.rs` pins the repaired identities and requires:

- exact neutral action-ID list;
- identical task/action vocabulary across arms;
- absence of correction markers from the shared worker-visible vocabulary;
- exact 1082/1775 context sizes;
- treatment starts with the exact complete control context;
- only treatment carries the exact 64-character marker and guard source evidence;
- both arms carry the same compact front and proposed patch;
- neither arm carries normalized retirement-oracle field names;
- treatment detail matches the current merged `project_prior_episode_detail()` result;
- worker packets omit organizer `context_ref` identifiers;
- one-byte treatment drift invalidates the registered digest;
- reversed synthetic observation order still maps actions to the correct arms.

The synthetic descriptive pair now uses:

```text
control first action
  inspect_accepted_guard_detail

treatment first action
  block_patch
```

only to verify arm mapping and `same_first_action=false`. It remains fixture data, not worker evidence.

## Superseded pre-execution receipt

The original #254 plan merged as:

```text
0eabd231a23dfc243c167d336b7ad9836aff2bcc
```

with semantic head and checks:

```text
head:       c9fccae8877ba6a1f684b803420943e3d1946b79
CI:         32268660788 success
provenance: 32268660723 success
```

Its original plan/control/treatment fingerprints were:

```text
plan
6f3eddecf177271c0ad60f32fb17008841bdb81f34aa717f52be90c3bdd1f69b

control
9949665d13b162692ebd3f7d12b6f162881f18d2d381c7257100bbb89c317f01

treatment
6d3d93c574e81800ef1829216356d258553cabc7d2bdce2d09c32f46659c738c
```

Those identities are retained only as historical audit evidence. They are superseded for execution because the action vocabulary leaked the desired correction. No real worker run exists under those packet identities.

## Execution boundary

The blind packet materializer must regenerate artifacts from the repaired fingerprints above before any external worker session is admitted. A genuine runner still needs independent fresh sessions and must keep organizer arm mapping hidden from the worker.

Cultist's #245 reconciliation remains descriptive: observed first-action IDs plus whether the two actions match. The separate run-admission lane may add session/harness comparability checks without changing this plan or the minimal observation object.

## Boundary

- research only;
- no provider/network call;
- no model/worker invocation;
- no synthetic observation represented as real evidence;
- no treatment-effect or causal claim;
- no analyzer ranking or promotion;
- no ordinary product CLI/report change;
- no change to #245 trial protocol or #249 selected-detail schema.

This keeps one narrow question intact: does the selected source detail change what the worker does first, when the action vocabulary itself does not reveal the correction?

Refs #41 #137 #179 #187 #217 #219 #237 #244 #245 #246 #249 #254 #258.

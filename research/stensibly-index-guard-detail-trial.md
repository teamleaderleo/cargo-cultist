# Blind first-action trial: compact front vs selected guard detail

Merged #245 gives Cultist a blindable paired behavioral-trial protocol. Merged #249 gives the temporal prior-episode path one source-owned post-selection detail packet for the Stensibly Convex index-limit guard.

Merged #254 froze the first paired plan. Issue #258 found that its shared first-action vocabulary itself carried the intended corrective action. No real worker observation was executed under that plan. The current registration repairs that leak before evidence collection while preserving #254/#259 as historical pre-execution lineage.

Human-facing references use `redirect.github.com`:

- [Cultist #245](https://redirect.github.com/teamleaderleo/cultist/pull/245)
- [Cultist #249](https://redirect.github.com/teamleaderleo/cultist/pull/249)
- [Cultist #254](https://redirect.github.com/teamleaderleo/cultist/pull/254)
- [Cultist #258](https://redirect.github.com/teamleaderleo/cultist/issues/258)
- [Cultist #259](https://redirect.github.com/teamleaderleo/cultist/pull/259)
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

The worker chooses exactly one first justified action before writing review feedback.

## Current neutral action vocabulary

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

The shared task/action vocabulary therefore supplies a decision boundary while withholding the downstream correction. It contains none of the explicit operational answer markers `shorten`, `64`, `identifier limit`, or `preserving field order`.

The superseded #254 registration used `block_and_shorten_identifier`, which supplied corrective content to both arms. #258 identified that leakage before any worker execution.

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

The explicit operational threshold and guard-source sentence stay outside the control context.

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

Normalized capability-demand oracle fields stay outside both contexts. Worker-visible context sizes remain:

```text
control    1082 bytes
treatment  1775 bytes
```

## Exact current fingerprints

The existing #245 typed materializer regenerated these identities from the neutral plan in workflow run `32269342216`:

```text
plan
cultist-behavioral-trial-plan-sha256-v1:1aca6332c77ed72b49cb20593f215c7eb2952121ad9bf3d5ae60bea0df5df024

control
cultist-behavioral-worker-packet-sha256-v1:5fa460fe007276013ed019830daaf9fc8d086cc5d3d5dfdfc5dd33a58052887d

treatment
cultist-behavioral-worker-packet-sha256-v1:a80303b59b9a46c5f4e6adb446abb01fbaeb5d898911bf6ad1248b3b0cf38549
```

The same run checked the shared worker-visible task/action vocabulary for the leaked corrective markers before printing the fingerprints.

## Current controls

`tests/behavioral_trial_index_guard_detail.rs` pins the current identities and requires:

- the exact neutral action-ID list;
- identical task/action vocabulary across arms;
- absence of corrective-answer markers from that shared vocabulary;
- exact 1082/1775 worker-visible context sizes;
- treatment-prefix equality;
- source-owned selected detail only in treatment;
- hidden organizer arm refs;
- one-byte treatment-drift rejection.

The synthetic descriptive pair uses:

```text
control first action
  inspect_accepted_guard_detail

treatment first action
  block_patch
```

only to verify arm mapping and `same_first_action=false`. It remains synthetic fixture data.

`.github/workflows/stensibly-index-guard-detail-inputs.yml` independently materializes the two blind packet files and an organizer-only manifest. It pins the same current fingerprints and rejects shared task/action vocabulary containing `shorten`, `64`, `identifier limit`, or `preserving field order`.

## Superseded pre-execution receipt: #254 / #259

The first registered plan used:

```text
block_and_shorten_identifier
Block this patch and shorten the proposed index identifier while preserving field order
```

Its exact fingerprints were:

```text
plan
cultist-behavioral-trial-plan-sha256-v1:6f3eddecf177271c0ad60f32fb17008841bdb81f34aa717f52be90c3bdd1f69b

control
cultist-behavioral-worker-packet-sha256-v1:9949665d13b162692ebd3f7d12b6f162881f18d2d381c7257100bbb89c317f01

treatment
cultist-behavioral-worker-packet-sha256-v1:6d3d93c574e81800ef1829216356d258553cabc7d2bdce2d09c32f46659c738c
```

The semantic head merged in #254 was:

```text
head:       c9fccae8877ba6a1f684b803420943e3d1946b79
CI:         32268660788 success
provenance: 32268660723 success
```

The frozen plan merged as:

```text
0eabd231a23dfc243c167d336b7ad9836aff2bcc
```

#259 retained that executed validation receipt on current main. It remains useful historical evidence that the #254 bytes and tests were internally consistent. #258 changes the experiment-validity judgment: the shared action vocabulary revealed the intended correction, so those fingerprints are superseded for worker execution.

No real `BehavioralTrialObservation` was collected under the superseded registration. The repair therefore invalidates zero observed worker evidence.

## Execution boundary

The current neutral plan is frozen and materializable. A later runner may supply each blind packet to a genuinely independent worker session and retain actual `BehavioralTrialObservation` records.

Cultist's #245 reconciliation remains descriptive: observed first-action IDs plus whether the two actions match. Interpretation stays separate from the receipt.

## Boundary

- research only;
- no provider/model invocation in this lane;
- no synthetic observation represented as real evidence;
- no treatment-effect or causal claim;
- no analyzer ranking or promotion;
- no ordinary product CLI/report change;
- no change to the #245 protocol or #249 selected-detail schema.

Refs #41 #137 #179 #187 #217 #219 #237 #244 #245 #246 #249 #254 #258 #259.

# Behavioral-trial external run receipts

Merged #245 defines blindable first-action plans, worker packets, observations, and descriptive pair reconciliation. Merged #262 repairs the Stensibly trial to a neutral shared action vocabulary before any real worker execution. Issue #267 makes the remaining external boundary concrete: genuinely fresh sessions, fixed worker/harness/tool/sampling configuration, preserved raw outputs, and exact packet identity.

This experiment adds an organizer-side run receipt for those facts without changing the worker-visible observation schema.

## Existing worker contract stays unchanged

The worker still receives one `BehavioralWorkerPacket` and returns one exact `BehavioralTrialObservation`:

```text
trial_id
plan_fingerprint
worker_packet_fingerprint
worker_ref
first_action_id
```

The worker still does not classify correctness, usefulness, treatment effect, success, or causality.

## Run metadata

The organizer supplies one `BehavioralTrialRunMetadata` alongside the raw packet and raw worker response:

```text
execution_origin = external_harness
sequence_index = 1 | 2
worker_identity
harness_identity
affordance_identity
sampling_config_sha256
session_id
freshness_receipt
fresh_session = true
prior_condition_exposure = false
```

`fresh_session=true` is an external execution assertion. Cultist cannot prove provider-session freshness from local process state. The receipt therefore retains `freshness_receipt` as the evidence coordinate the harness/provider uses to justify the assertion.

## Receipt construction

`build_behavioral_trial_run_receipt()` consumes:

```text
frozen BehavioralTrialPlan
run metadata
exact raw worker-packet JSON bytes
exact raw worker-output JSON bytes
```

It then:

1. validates the metadata and freshness assertions;
2. parses the raw packet through the existing `BehavioralWorkerPacket` schema;
3. requires the parsed packet to equal one exact arm materialized from the frozen plan;
4. parses the raw worker output through the existing `BehavioralTrialObservation` schema;
5. binds the observation to the exact plan and exact packet fingerprint;
6. requires the first action to be in the frozen vocabulary;
7. computes `sha256:<hex>` from the exact raw packet bytes;
8. computes `sha256:<hex>` from the exact raw output bytes;
9. retains both exact raw JSON strings plus the typed observation.

The organizer does not hand-type either content hash.

## Pair admission

`evaluate_behavioral_trial_run_pair()` revalidates both receipts from their retained raw bytes before calling the existing #245 pair evaluator.

A pair is admitted only when:

```text
one exact control packet
one exact treatment packet
fresh_session = true for both
prior_condition_exposure = false for both
sequence indexes are exactly {1, 2}
session ids differ
freshness evidence refs differ
worker run refs differ
worker_identity matches
harness_identity matches
affordance_identity matches
sampling_config_sha256 matches
```

The output keeps the ordinary #245 descriptive trial result and adds the organizer execution coordinates needed to inspect the pair:

```text
control/treatment first-action ids
same_first_action
fixed worker/harness/affordance/sampling identities
control/treatment sequence index
control/treatment session id
control/treatment freshness receipt
```

It introduces no success, improvement, treatment-effect, or causal field.

## Real current-plan controls

`tests/behavioral_trial_run.rs` uses the merged neutral Stensibly guard-detail plan from #262.

The standard controls require:

- the loaded action vocabulary contains `block_patch` and excludes superseded `block_and_shorten_identifier`;
- a fresh BA pair reconciles correctly even when receipt vector order is treatment/control;
- exact raw packet and output SHA-256 receipts are computed by the builder;
- retained raw output reparses to the stored typed observation;
- `fresh_session=false` rejects;
- `prior_condition_exposure=true` rejects;
- an observation naming the opposite worker packet rejects;
- an unregistered first action rejects;
- the same session id across arms rejects;
- worker/harness/tool/sampling drift across arms rejects;
- mutating retained raw output after receipt construction rejects on SHA mismatch.

The unit pair uses synthetic metadata only as a deterministic validator control. It is not retained as a real external run.

The organizer examples compile shared `behavioral_trial` modules through complementary entry points, so their local module declarations explicitly allow dead code while canonical source remains strict under ordinary Clippy.

## Organizer examples

Build one run receipt from external files:

```text
cargo run --example behavioral_trial_run_receipt -- \
  PLAN.json \
  METADATA.json \
  WORKER_PACKET.json \
  RAW_OUTPUT.json
```

Reconcile exactly two admitted run receipts:

```text
cargo run --example behavioral_trial_run_reconcile < RUN_PAIR.json
```

Both examples are provider-neutral and perform no model/provider call.

## Relationship to the blind Stensibly pair

Merged #262 rematerializes the repaired neutral blind packets through the permanent input workflow introduced by the earlier trial-input lane. Issue #267 asks an external harness to run those two current packet files in genuinely fresh sessions and preserve raw outputs plus freshness/config evidence.

This run receipt is the typed intake boundary for that future execution:

```text
#262 neutral blind packet
+ external fresh-session metadata
+ exact raw worker response
-> this run receipt
-> pair admission
-> existing #245 descriptive first-action reconciliation
```

The superseded #254/#260 packet identities remain historical pre-execution evidence only and are rejected by the current plan binding.

The capability-demand retirement research has a separate run-receipt contract because it evaluates a different completion outcome. This first-action receipt reuses #245 observations and does not import capability-demand outcome semantics.

## Pair-classification composition

Issue #264 owns a separate projection after this byte-authentic intake:

```text
validated individual receipts
-> admitted | confounded | invalid_pair
-> #245 descriptive reconciliation only for admitted
```

That projection should consume this receipt type instead of defining a second run-receipt schema. Malformed/tampered individual receipts continue to reject here before experimental comparability is classified.

## Boundary

- research only;
- no provider/model SDK;
- no API key or secret handling;
- no model/worker invocation;
- no claim that local process isolation proves a fresh provider session;
- no synthetic-as-real observation;
- no automatic behavioral interpretation;
- no treatment-effect or causal claim;
- no ordinary product CLI/report change.

The goal is narrower: make an eventual external first-action pair auditable from exact packet bytes, exact raw response bytes, and explicit session/config evidence instead of hand-assembled observation JSON.

Refs #41 #137 #245 #254 #260 #262 #264 #267.

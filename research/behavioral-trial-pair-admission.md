# Behavioral-trial pair comparability projection

Tracking: #264. This lane composes strictly after merged #269/#273 byte-authentic individual run receipts.

## Ownership boundary

```text
#269 / #273
  exact canonical worker-packet bytes
  exact raw worker-output bytes
  typed observation binding
  authentic run metadata, including freshness/exposure facts

this lane
  admitted | confounded | invalid_pair
  + typed pair-level reasons

#245
  descriptive control/treatment first actions for admitted pairs only
```

This module defines no second run-receipt schema and performs no packet serialization or raw-output capture of its own.

## Authenticity before comparability

Every supplied `BehavioralTrialRunReceipt` is rebuilt through merged `build_behavioral_trial_run_receipt()` using its retained raw packet/output bytes and metadata. Any tampered or internally drifted receipt rejects before a comparability verdict.

Authentic receipts may truthfully record:

```text
fresh_session = false
prior_condition_exposure = true
```

because #273 separates receipt authenticity from strict admitted-pair freshness.

## Verdicts and reasons

The projection preserves every observed pair-level reason in a sorted `reasons[]` vector:

```text
same_arm
worker_identity_drift
harness_identity_drift
affordance_identity_drift
sampling_config_drift
non_fresh_session
prior_condition_exposure
reused_session_id
reused_freshness_receipt
reused_worker_ref
invalid_sequence_coverage
```

Several reasons may coexist on one authentic pair. The projection retains them together instead of collapsing the evidence to one first failure.

### `invalid_pair`

Both receipts are individually authentic, but they do not cover one control plus one treatment for the frozen plan. `same_arm` has verdict precedence, while any additional pair-level reasons remain visible.

### `confounded`

The two authentic receipts cover distinct arms, but one or more execution/session reasons remain. No behavioral interpretation is emitted for a confounded pair.

### `admitted`

The two authentic receipts cover one control and one treatment and `reasons[]` is empty. Only this verdict calls merged `evaluate_behavioral_trial_run_pair()` and retains its descriptive #245 result.

AB and BA vector order are both valid. Recorded sequence indexes determine execution order; packet fingerprints determine semantic arm identity.

## Output boundary

The projection retains the evidence needed to audit every pair:

```text
trial_id
plan_fingerprint
execution-order packet fingerprints
sequence indexes
session ids
freshness receipts
worker refs
verdict
reasons[]
frozen_identity_match
fresh_uncontaminated_sessions
distinct_arm_coverage
```

and keeps:

```text
automatic_effect_claim = false
automatic_generalization = false
```

No success, improvement, treatment-effect, capability-retirement, promotion, or causal label is introduced.

## Controls

`tests/behavioral_trial_pair_admission.rs` covers:

- admitted AB order with empty reasons;
- admitted BA vector order while preserving recorded execution sequence;
- admitted same and different first actions;
- each worker/harness/affordance/sampling drift reason;
- authentic non-fresh, prior-exposed, session-reused, freshness-reused, worker-ref-reused, and duplicate-sequence reasons;
- simultaneous confounds preserved together;
- same authentic arm twice -> `invalid_pair` + `same_arm`;
- tampered individual receipt -> hard rejection before comparability classification.

Synthetic receipts in the test are built through the canonical #269 receipt builder from actual current neutral Stensibly packet bytes. They are validator fixtures only, never represented as observed worker behavior.

## Research CLI

```text
cargo run --example behavioral_trial_pair_admission < RUN_PAIR.json
```

The CLI parses the canonical #269 run-pair schema and prints this projection. It performs no worker/model/provider invocation.

This reason-preserving surface supersedes the useful pair-reason idea from stale #274 while keeping the current #273 authenticity boundary.

Refs #137 #245 #262 #264 #265 #267 #269 #271 #273 #274.

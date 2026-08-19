# Paired behavioral trial run receipts

Status: research execution-admission layer for the blind first-action protocol from #245, using the neutral Stensibly plan repaired by #262.

The behavioral-trial core intentionally keeps one semantic observation small:

```text
worker packet
-> one first_action_id
```

That object records the observed action. It does not prove that a two-run experiment used fresh isolated sessions or fixed worker/harness/tool settings.

This layer adds that missing execution receipt **outside** the minimal observation schema.

## Flow

```text
BehavioralTrialPlan
-> exact blind BehavioralWorkerPacket bytes
-> external fresh worker session
-> preserved raw output
-> BehavioralTrialRunReceipt
-> execution-pair admission
-> existing #245 BehavioralTrialObservation
-> existing #245 first-action reconciliation
```

Cultist still does not launch the worker.

## Run receipt v1

Each external run supplies one bounded JSON object:

```text
schema_version = 1

trial_id
pair_id
run_id
sequence_index = 1 | 2

plan_fingerprint
worker_packet_fingerprint
worker_packet_file_sha256

worker_ref
worker_identity
harness_identity
affordance_identity
sampling_config_sha256

session_id
fresh_session
prior_condition_exposure

raw_worker_output_sha256
first_action_id
```

`worker_packet_file_sha256` is the SHA-256 of the exact pretty-JSON worker packet file emitted by `behavioral_trial_materialize`, including its terminal newline. This binds the receipt to the concrete bytes given to the worker as well as the typed packet fingerprint.

`raw_worker_output_sha256` binds the normalized first-action receipt back to preserved raw output. Cultist does not infer private reasoning from that output.

## Per-run admission

For each receipt, Cultist recomputes the current plan fingerprint and both materialized worker packets.

The run rejects before pair interpretation if:

- `trial_id` differs from the plan;
- `plan_fingerprint` differs;
- the packet fingerprint belongs to neither plan arm;
- the packet-file SHA differs from the canonical materialized bytes for that packet;
- `first_action_id` is outside the plan vocabulary;
- receipt hashes/coordinates are malformed.

These are invalid receipts, not experimental confounds.

## Pair admission

Frozen axes across the pair:

```text
pair_id
worker_identity
harness_identity
affordance_identity
sampling_config_sha256
```

Fresh-session requirements:

```text
both fresh_session = true
both prior_condition_exposure = false
session ids differ
run ids differ
sequence indices exactly {1, 2}
```

The two receipts must also cover both distinct worker-packet fingerprints from the plan.

Pair verdicts:

```text
admitted
confounded
invalid_pair
```

`confounded` means the frozen axes or fresh-session requirements failed.

`invalid_pair` means individually valid receipts were supplied for the same arm twice, so the pair does not represent the registered intervention.

## Behavioral interpretation

Only an `admitted` execution pair is converted into the existing #245 observations and passed through `evaluate_behavioral_trial_pair()`.

That nested result remains deliberately descriptive:

```text
control.first_action_id
treatment.first_action_id
same_first_action
```

Execution order does not define arm identity. AB and BA are both admissible because the existing packet fingerprints identify the arms.

The outer result always says:

```text
automatic_effect_claim = false
automatic_generalization = false
```

A changed first action is evidence that the observed action changed in that admitted pair. This layer does not call it improvement, causation, capability retirement, or a promotion criterion.

## Repaired Stensibly exact-byte control

Issue #258 found that the original #254 shared action vocabulary leaked the desired correction. Merged #262 neutralized that vocabulary before any real worker execution and regenerated the executable blind packets.

Current plan fingerprint:

```text
cultist-behavioral-trial-plan-sha256-v1:1aca6332c77ed72b49cb20593f215c7eb2952121ad9bf3d5ae60bea0df5df024
```

Current typed worker-packet fingerprints:

```text
control
cultist-behavioral-worker-packet-sha256-v1:5fa460fe007276013ed019830daaf9fc8d086cc5d3d5dfdfc5dd33a58052887d

treatment
cultist-behavioral-worker-packet-sha256-v1:a80303b59b9a46c5f4e6adb446abb01fbaeb5d898911bf6ad1248b3b0cf38549
```

The retained Stensibly plan must reproduce the exact serialized files from the repaired #262 materializer run:

```text
control packet file
sha256 6a568aed1eb660141cd7e7759e47edeb10a5c759fe1402689699dd7b6837149e
bytes  2252

treatment packet file
sha256 1063efc8ecdf0313b947923dad8216fb9fa43b2e8b8cafa7ab6b63d53eb65c7d
bytes  2953
```

The context bytes remain 1082 / 1775; the serialization changed because the shared action vocabulary is now neutral:

```text
block_patch
inspect_accepted_guard_detail
approve_patch
inspect_more_repository_context
```

Artifact receipt from the repaired materializer:

```text
run       32271062286
artifact  9372168346
archive   sha256:d4de6fe63a7088a921cbd6ea1a3c386f22a5620c5c25a71f34edc5bb11fd23cd
```

The superseded #254/#260 packet identities are intentionally excluded from execution admission.

## Replay command

After an external harness has produced two run receipts:

```text
cargo run --example behavioral_trial_run -- \
  research/behavioral-trials/stensibly-index-guard-detail.json \
  run-a.json \
  run-b.json
```

The external harness should preserve the raw worker outputs whose SHA-256 values appear in the receipts.

## What remains outside Cultist

A real execution harness still owns:

- choosing and pinning the worker implementation/version;
- starting genuinely fresh sessions;
- fixing sampling configuration;
- fixing available tools/affordances;
- keeping organizer arm assignments hidden from the worker;
- preserving raw outputs;
- producing the run receipts.

No real worker run exists merely because the synthetic admission tests pass.

## Boundary

- research only;
- no provider/model SDK;
- no API key/secret handling;
- no model-brand taxonomy;
- no chain-of-thought;
- no synthetic receipt represented as real behavior;
- no treatment-effect or causal label;
- no authority grant;
- no change to `BehavioralTrialObservation`;
- no primary product CLI/report change.

North star:

> Admit first-action A/B evidence only when the repository can prove which exact blind input each fresh session received and which experimental axes stayed fixed.

# Blindable paired behavioral trials

Issue #241 fills the A/B seam left open by #137's behavioral receipts.

Behavioral receipts answer:

```text
what happened after one evidence candidate surfaced or stayed quiet?
```

Paired trials answer a narrower comparison question:

```text
with the same task and raw historical facts,
did the registered control and treatment packets produce the same first-action ID?
```

The protocol deliberately stops there. `different first action` is not serialized as improvement, treatment effect, success, prevention, or product promotion.

## Internal-only boundary

The first trial protocol performs no provider or model calls.

It uses retained same-repository corpus files and produces local JSON packets. External source identities in the registered contexts are literal provenance, not new external interactions.

The experiment adds no ordinary `cargo-cultist` command or `AnalysisReport` field.

## Plan v1

`src/behavioral_trial.rs` defines a bounded plan:

```text
BehavioralTrialPlan
  trial_id
  task_instruction
  allowed_first_actions[]
  control
    context_ref
    exact context text
    context_digest
  treatment
    context_ref
    exact context text
    context_digest
```

Each context digest uses:

```text
cultist-behavioral-context-sha256-v1
```

The supplied digest is validated against the exact context bytes. Control and treatment must have different exact digests.

The fully validated typed plan receives:

```text
cultist-behavioral-trial-plan-sha256-v1:<sha256>
```

Fingerprint framing is field-order independent after JSON deserialization. One-byte semantic/context changes produce a different fingerprint.

## Blindable worker packet

The organizer selects an arm when materializing:

```bash
cargo run --quiet --example behavioral_trial_materialize < materialize-request.json
```

The returned `BehavioralWorkerPacket` contains:

```text
trial_id
plan_fingerprint
worker_packet_fingerprint
task_instruction
context
context_digest
allowed_first_actions
```

It omits:

```text
control / treatment label
context_ref
expected action
behavioral outcome
```

The worker-packet fingerprint uses:

```text
cultist-behavioral-worker-packet-sha256-v1
```

The organizer retains the arm-to-packet mapping. A worker only needs the materialized packet.

## Worker observation

The worker chooses exactly one registered first-action ID and returns:

```text
BehavioralTrialObservation
  trial_id
  plan_fingerprint
  worker_packet_fingerprint
  worker_ref
  first_action_id
```

The worker does not classify the result as useful, correct, changed, successful, or causal.

## Pair reconciliation

After one control and one treatment observation exist:

```bash
cargo run --quiet --example behavioral_trial_reconcile < pair.json
```

The evaluator maps packet fingerprints back to arms and emits:

```text
control
  worker_ref
  first_action_id

treatment
  worker_ref
  first_action_id

same_first_action
```

It rejects observations for unknown packets, two observations for one arm, changed plan fingerprints, and action IDs outside the pre-registered vocabulary.

A later #137 `BehavioralReceipt` may interpret an observed episode only after the concrete worker consequence is available.

## Registered trial A: stale review front

Plan:

```text
research/behavioral-trials/prior-review-stale.json
```

Source carrier already retained by #206/#219:

```text
The-PR-Agent/pr-agent#2424
prior review comment 3355870564
prior reviewed head  8fb9e4e86b4794d39afba2d62413571cbc04a744
current head         f6070fb1a45516565bbb8deeb02a1f66cec13d91
prior outcome        patch_changed
```

Both arms receive those raw coordinates.

Treatment appends only:

```text
Cultist prior-episode front:
source disposition: refresh_existing_thread
old outcome applicability: INVALID
next: recompute_and_refresh_review_thread
```

Registered fingerprints:

```text
plan
  cultist-behavioral-trial-plan-sha256-v1:b394eefd406bac16e2ba7690bd45f6373d3b029208fdd55d70c3b9a943f15d65

control packet
  cultist-behavioral-worker-packet-sha256-v1:4a59e3aac7200f97bf059d3df4fa3ba81c322d2ecaaac18ade25724ca6799272

treatment packet
  cultist-behavioral-worker-packet-sha256-v1:3f9a24e968a1b2cfd2284e6668054537e46eaeca0ba5fdb69c116d4a7b3f8e9d
```

Action vocabulary includes new-thread, reuse, recompute/refresh, acquire-coordinate, and inspect-more-history choices.

**Status: registered, not executed.**

## Registered trial B: closed issue / re-report front

Plan:

```text
research/behavioral-trials/closed-rereport.json
```

Source carrier already retained by #212/#219:

```text
anthropics/claude-code#31294
anthropics/claude-code#57507
prior state       closed
prior reason      not_planned
closure actor     github-actions[bot]
later relation    explicit re-report
later state       closed
```

Both arms receive the same raw lifecycle/re-report facts.

Treatment appends only:

```text
Cultist prior-episode front:
closure kind: administrative_inactive
re-report observed: true
clearance: UNKNOWN
next: inspect_prior_failure_and_rereport
```

Registered fingerprints:

```text
plan
  cultist-behavioral-trial-plan-sha256-v1:8d42a49630eace01d6c14055a79d41a91dcb22f84f32db2a92e483ec60840ba4

control packet
  cultist-behavioral-worker-packet-sha256-v1:73ce3562f8fc382db9ab3505236fdf965c5d6b4f6a5c6cd745ddc2b121c8f7e6

treatment packet
  cultist-behavioral-worker-packet-sha256-v1:6c63c84bcf52831cb0d4bc3490bf3a279bf77cacd67ede8d300fb6c09ca7d4e8
```

The action vocabulary includes exhausted/prior-only/later-only/combined-inspection/acquire-more-evidence choices.

**Status: registered, not executed.**

## Controls

The standard Rust controls require:

- exact known context, plan, and packet fingerprints;
- typed plan fingerprint stability across JSON field order/formatting;
- context mutation changes the arm and plan fingerprints;
- worker packets omit arm labels and context refs;
- same first action remains descriptive;
- different first action remains descriptive;
- observation order does not determine arm identity;
- unknown actions reject;
- duplicate same-arm observations reject;
- unknown packet fingerprints reject;
- plan mutation after packet materialization rejects;
- identical control/treatment contexts reject;
- supplied context digest must match exact bytes;
- duplicate action IDs reject;
- unknown machine fields fail closed.

The two registered plans are parsed and fingerprinted by `tests/behavioral_trial_real.rs` so future edits cannot silently mutate an already-prepared experiment.

## What this earns

Cultist can now do the honest part of a behavioral A/B experiment before any worker is recruited:

```text
freeze task
freeze action vocabulary
freeze raw control context
freeze treatment projection
freeze packet identities
hide arm labels from the worker packet
accept one exact first-action ID
compare the pair descriptively
```

What remains unearned is the result.

Once a fresh-worker pair is actually run, #137 can decide whether to retain the observation as `changed_next_action`, `prevented_or_reversed_wrong_turn`, `useful_same_action`, `irrelevant`, or another existing behavioral outcome.

Refs #137 #213 #217 #219 #237 #241.

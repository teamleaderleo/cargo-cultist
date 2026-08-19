# Capability-demand retirement execution packaging

Status: research execution tooling for #253 / #238.

The frozen Stensibly trial (#239) and strict worker-run receipt evaluator (#246) already separate repository evidence from model/harness behavior. This slice closes the organizer-side gap before any real model call:

```text
exact frozen trial inputs
  -> blind slot 1 / slot 2 bundles
  -> external fresh worker sessions
  -> preserved raw outputs + external run metadata
  -> computed WorkerRunReceipt v1
  -> existing #246 pair evaluator
```

## Why a separate organizer plan exists

The worker-facing bundle must not reveal which evidence condition is the baseline or treatment.

Each slot receives only:

```text
worker-input.json
  trial / pair / slot / run identity
  sequence index
  exact repository / revision / target identity
  task / patch / evidence fingerprints

task.txt
proposed.patch
evidence.json
```

`worker-input.json` does not contain the condition ID, baseline/treatment label, decisive-evidence flag, or evaluator oracle.

The organizer-only `organizer-plan.json` keeps the slot-to-condition mapping needed to build a valid #246 receipt later.

## Exact-input admission

The packager accepts only task, patch, and evidence bytes whose SHA-256 + byte counts match the supplied materialized input manifest.

For the worker task and proposed patch it also independently reconstructs the line-terminated bytes from the frozen trial spec and requires exact equality. A forged or mismatched manifest therefore cannot silently replace the worker task while preserving the same trial coordinate.

V1 requires exactly two conditions with one `decisive_evidence_present=false` condition and one `true` condition.

## Order control

The organizer may request:

```text
AB
BA
seed:<organizer-seed>
```

`AB` executes the no-decisive-evidence arm first. `BA` executes the decisive-evidence arm first. Seeded ordering hashes the organizer seed together with the pair ID; only the seed hash is retained in the organizer plan.

Condition semantics do not depend on execution order. #246 derives baseline/treatment roles from the admitted input manifest.

## Preparing a pair

```text
cargo run --example capability_demand_pair_prepare -- \
  TRIAL.json \
  INPUT_MANIFEST.json \
  worker-task.txt \
  proposed.patch \
  packet-one.json \
  packet-two.json \
  prepared-pair \
  pair-001 \
  seed:batch-001
```

The output directory must not already exist.

The two packet arguments may be supplied in either order; exact manifest fingerprints identify their conditions.

## External run metadata

After a genuinely external worker session completes one slot, the harness records a bounded metadata object:

```json
{
  "schema_version": 1,
  "slot_id": "slot-1",
  "execution_origin": "external_harness",
  "worker_identity": "worker-family@version",
  "harness_identity": "harness@version",
  "affordance_identity": "tool-set@version",
  "session_id": "provider-or-harness-session-id",
  "fresh_session": true,
  "prior_condition_exposure": false,
  "evaluated_outcome": "failed",
  "evidence_inspection": "unobservable",
  "context_expanded": false
}
```

Starting a new operating-system process is not sufficient evidence for `fresh_session=true`. The external harness owns that assertion and must preserve whatever provider/session receipt justifies it.

`evaluated_outcome` remains an external evaluator classification under the fixed #239 completion contract. This lane adds no prose-similarity or model judge.

## Building an admitted run receipt

```text
cargo run --example capability_demand_run_receipt -- \
  prepared-pair/organizer-plan.json \
  slot-1-run-metadata.json \
  sampling-config.json \
  checkout-reset-receipt.txt \
  raw-worker-output.txt \
  > run-1.json
```

The receipt builder computes these fingerprints from the supplied bytes:

```text
sampling_config_sha256
checkout_reset_receipt_sha256
worker_output_sha256
```

The organizer cannot hand-type those hashes independently. The generated object is reparsed through the existing #246 `WorkerRunReceipt v1` validator before it is emitted.

## Synthetic test boundary

The test suite has a separate `synthetic_test` execution origin that exists only under Rust `cfg(test)`.

The production receipt builder rejects it. Synthetic fake-worker controls can therefore prove AB/BA reconciliation, session-contamination rejection, fingerprint admission, and the `paired_retirement_signal` path without creating JSON that looks like executed external worker evidence.

## What still requires an external harness

This repository tooling deliberately does not perform the two worker calls. A valid real pair still needs:

1. one fixed worker/harness/tool/sampling configuration;
2. two genuinely fresh sessions;
3. no condition exposure across sessions;
4. exact frozen worker bundles;
5. raw output preservation;
6. checkout/worktree reset evidence;
7. an evaluator-owned completion classification;
8. #246 pair admission and interpretation.

A single successful A/B pair remains a local signal. Replicated retirement evidence requires repeated fresh pairs under controlled order and sampling, as recorded on #238.

## Boundary

- research tooling only;
- no primary `cargo-cultist` CLI change;
- no provider/model SDK;
- no API key or secret handling;
- no model invocation;
- no private chain-of-thought;
- no oracle in worker bundles;
- no automatic causal/generalization claim;
- no authority grant from task success.

# Behavioral-trial pair comparability classification

Status: research projection for #264 over the byte-authentic external run receipts merged in #269.

#269 owns individual run authenticity:

```text
frozen plan
+ exact canonical worker-packet file bytes
+ exact raw typed worker output bytes
+ external run metadata
-> BehavioralTrialRunReceipt
```

It hard-rejects malformed, tampered, nonfresh, or prior-exposed individual runs. It also retains enough organizer metadata to inspect whether two individually valid runs are actually comparable.

This layer classifies that **pair-level** question without introducing another run-receipt schema.

## Flow

```text
BehavioralTrialRunPair
-> rebuild each receipt from its retained exact bytes
-> hard reject invalid individual receipt
-> inspect pair geometry
-> admitted | confounded | invalid_pair
-> existing #245 descriptive reconciliation only when admitted
```

## Verdicts

```text
admitted
confounded
invalid_pair
```

`admitted` means both exact arms are present once and the frozen execution axes/session coordinates are compatible.

`confounded` means both individual receipts are valid, but the pair changes or reuses an execution axis that prevents clean comparison.

`invalid_pair` means both individual receipts are valid but do not cover the registered two-arm intervention. V1 uses this for same-arm pairs.

Malformed/tampered individual evidence remains an error from the #269 intake boundary. It is never downgraded into one of these experimental verdicts.

## Pair-level reason vocabulary

Typed reasons are:

```text
same_arm
worker_identity_drift
harness_identity_drift
affordance_identity_drift
sampling_config_drift
reused_session_id
reused_freshness_receipt
reused_worker_ref
invalid_sequence_coverage
```

The classifier preserves all observed pair-level reasons instead of stopping at the first confound.

Verdict precedence is deterministic:

```text
same_arm present
-> invalid_pair

otherwise any reason present
-> confounded

otherwise
-> admitted
```

## Individual receipt revalidation

Before pair classification, every retained #269 receipt is rebuilt through `build_behavioral_trial_run_receipt()` using:

```text
same frozen plan
receipt.metadata
receipt.raw_worker_packet exact bytes
receipt.raw_output exact bytes
```

The rebuilt receipt must equal the retained receipt exactly.

That rechecks:

- external execution origin;
- fresh-session / no-prior-exposure assertions;
- exact neutral #262 materialized packet bytes;
- exact raw output bytes;
- derived packet/output hashes;
- typed observation binding;
- registered first-action vocabulary.

A receipt whose retained raw output, hash, observation, or packet bytes drift after construction rejects before pair geometry is considered.

## Admitted pair

Only `admitted` calls the existing #269 `evaluate_behavioral_trial_run_pair()`, which itself delegates behavioral interpretation to #245.

The nested result remains descriptive:

```text
control first_action_id
treatment first_action_id
same_first_action
```

AB and BA execution order are both valid because arm identity comes from exact packet fingerprints, not vector position.

This projection adds no success/correctness/effect semantics and always emits:

```text
automatic_effect_claim = false
automatic_generalization = false
```

## Deterministic controls

`tests/behavioral_trial_pair_classification.rs` covers:

- admitted AB;
- admitted BA;
- same-arm `invalid_pair`;
- worker identity drift;
- harness identity drift;
- affordance drift;
- sampling-config drift;
- reused session ID;
- reused freshness receipt;
- reused worker reference;
- invalid sequence coverage;
- multiple simultaneous confounds preserved together;
- tampered individual receipt hard-rejects instead of becoming `confounded`.

Synthetic tests exercise classification only. They are not represented as real external worker runs.

## Research CLI

Classify one already-materialized #269 run pair from stdin:

```text
cargo run --example behavioral_trial_pair_classify < RUN_PAIR.json
```

This performs no provider/model call and mutates nothing outside process memory.

## Relationship to #267

#267 remains the real execution task. A future external harness should:

1. use the neutral #262 blind packet files;
2. create genuinely fresh sessions;
3. preserve raw outputs and freshness/config evidence;
4. build #269 run receipts;
5. submit the two receipts here for comparability classification.

A green synthetic classifier suite is supporting evidence only. It does not mean the #267 worker pair has been executed.

## Boundary

- research only;
- no new run-receipt schema;
- no provider/model SDK;
- no API key or secret handling;
- no worker invocation;
- no chain-of-thought;
- no synthetic-as-real behavior;
- no treatment-effect or causal claim;
- no authority grant;
- no primary product CLI/report change.

North star:

> Keep evidence authenticity, experimental comparability, and behavioral interpretation as three separate gates so each can fail for the right reason.

Refs #137 #223 #245 #262 #264 #267 #269.
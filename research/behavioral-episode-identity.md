# Behavioral episode identity

Tracking: #161, building on merged #157 / #137.

Behavioral receipt v1 records one evidence-delivery outcome. It deliberately does not carry a durable observation identity. That is sufficient for reading a single receipt and unsafe for longitudinal aggregation, where copied receipts could otherwise be counted more than once.

This experiment keeps v1 receipts unchanged and adds a small outer identity layer.

## Episode wrapper

```text
BehavioralEpisode
  episode_id
  receipt: BehavioralReceipt v1

BehavioralEpisodeBatch
  schema_version
  episodes[]
```

`episode_id` identifies the observed delivery episode. Repository/revision/task/evidence coordinates stay inside the receipt and continue to carry their original meaning.

Example identities:

```text
pull-140:main-head-movement:4f8f9fcd->85e0b08b
github-actions:run/32242752523#active-work-heads-up
heldout:smolrunner-clone-01:treatment
```

The format is deliberately opaque beyond bounded canonical text. V1 does not parse semantics out of the ID.

## Duplicate semantics

A batch rejects duplicate episode IDs before any aggregate can count them.

```text
same episode_id + identical receipt
  -> duplicate -> reject

same episode_id + different receipt semantics
  -> conflicting duplicate -> reject

same receipt coordinates + distinct episode_id
  -> independent delivery -> accept
```

This separates **observation identity** from **receipt content identity**. Content hashes can still be useful later, but editing an annotation or action sentence should not silently mint a second observation.

## Retained controls

The ordinary test harness wraps the two live #140 receipts already preserved by #157:

1. main-head movement that changed the next action;
2. active-work heads-up that correctly stayed quiet on disjoint work.

The pair is accepted under distinct episode IDs.

Adversarial controls require:

- an exact copied episode to reject;
- the same episode ID with changed outcome/action to reject as a conflict;
- a repeated delivery of the same receipt under a new episode ID to remain distinct;
- malformed IDs to reject;
- oversized batches to reject before JSON parsing.

## Research validator

```text
cargo run --example behavioral_episode_batch < batch.json
```

Input is bounded to 256 KiB and 512 episodes. Every nested receipt is revalidated through merged #157 before the batch is admitted.

## Boundary

- no aggregate actionability score;
- no automatic analyzer promotion or demotion;
- no worker/model identity field;
- episode identity is observation identity, not Cultist finding semantic lineage;
- no chronological or causal semantics are inferred from the ID;
- receipt v1 remains unchanged and independently usable;
- `observed_at` stays deferred until a real longitudinal query requires one timestamp contract.

Once this survives dogfood, #137 can safely experiment with small aggregate views over unique admitted episodes while keeping raw receipts available for inspection.

# Durable UNKNOWN obligation research receipt

Tracking: #144. Stacked on the #141 justification-graph carrier.

## Dogfood discriminator from #141

The first justification graph required every obligation node to have a clearing edge. That is convenient for evaluating a graph containing known candidate evidence, but it fails the central durable-handoff case:

```text
worker finishes bounded investigation
-> material discriminator remains missing
-> clearing evidence does not exist yet
-> preserve the open obligation for the next worker
```

Encoding a prospective clearing edge as if evidence already existed would confuse an expected future observation with an observed evidence object.

This carrier therefore treats **the open durable record** and **arrived clearing receipts** as separate things.

## v0 record

```text
DurableObligation
  id
  question
  exact subject applicability requirements
  established_evidence[]
  missing_discriminator { kind, target }
  clearing_conditions[]
```

A clearing condition names an exact typed discriminator plus exact applicability requirements. A supplied receipt must match both before it can participate in clearing.

`established_evidence` is intentionally a bounded list of evidence IDs in this experiment. It preserves completed investigation for handoff without granting those strings authority or semantic lineage.

## Evaluation

The record and every candidate receipt reuse the shared applicability evaluator.

```text
subject APPLIES
  + matching clearing receipt APPLIES
    -> CLEARED

subject APPLIES
  + no matching/current clearing receipt
    -> OPEN

subject APPLIES
  + matching receipt applicability UNKNOWN
    -> UNKNOWN

subject applicability UNKNOWN
    -> UNKNOWN

subject INVALID because the exact coordinate moved
    -> REOPEN_REQUIRED
```

A semantically adjacent receipt with another discriminator kind remains unmatched even when it names the same target/revision.

## Why `REOPEN_REQUIRED` exists

An obligation captured for exact head A cannot silently become the obligation for head B. Head movement invalidates the old subject coordinate. The next worker/orchestrator can compile a fresh obligation for B while preserving the old record as a historical handoff receipt.

This keeps reopening distinct from pretending the stale obligation is still current.

## Standard-suite controls

The stacked carrier tests:

- open obligation with zero clearing evidence;
- exact matching receipt clears;
- exact subject head movement produces `reopen_required`;
- missing current coordinate remains `unknown`;
- semantically adjacent receipt does not clear;
- record round-trips with completed evidence references for fresh-worker handoff;
- a clearing condition must actually answer the declared missing discriminator.

## Boundary

- discriminator `{kind,target}` is a research key, not a final universal evidence ontology;
- receipt requirements use exact v1 matching against the clearing condition before applicability evaluation;
- richer subject/predicate envelopes belong to #147;
- probe discovery/cost belongs to #145;
- this record grants no mutation, review, merge, or deployment authority;
- solved records can remain as history, while current use always rechecks applicability.

## Next discriminator

The next useful experiment is #145: given this typed missing discriminator and a bounded set of available probe capabilities, select the cheapest admitted probe capable of producing a matching receipt. That experiment should consume this object rather than restating the missing question in prose.

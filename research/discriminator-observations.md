# Source-owned discriminator observation references

Tracking: #185. Stacked on the green #179 / #148 refinement-episode carrier.

## Question

Can three very different analyzer families expose one tiny typed reference seam for already-earned discriminator observations without moving their domain evaluators into the analyzer-refinement layer?

V0 deliberately models **references to observations**, not the logic that produces them.

## Observation contract

```text
DiscriminatorObservation
  observation_id
  discriminator_id
  subject_ref
  source_receipt
  value_state
    known { value_ref }
    unknown { reason_ref }
    invalid { reason_ref }
  applicability_ref?
```

The fields mean:

- `observation_id` — identity of this supplied observation event/object;
- `discriminator_id` — which distinction the source analyzer says this observation answers;
- `subject_ref` — exact source-owned subject identity;
- `source_receipt` — where the value/status came from;
- `value_ref` — opaque supplied partition value when current;
- `reason_ref` — source-owned explanation reference for UNKNOWN/INVALID;
- `applicability_ref` — optional exact reference to the source applicability/coordinate receipt.

The generic layer parses none of those reference strings for semantic meaning.

## Enumeration contract

`enumerate_discriminator_partitions` groups only KNOWN observations by:

```text
discriminator_id
  -> value_ref
     -> current observation receipts[]
```

UNKNOWN and INVALID observations remain separate inspectable lists under the same discriminator and never enter current partitions.

Each current partition keeps every observation identity, subject, source receipt, and applicability reference. Equal values from two source receipts therefore remain two observations rather than being silently deduplicated into one evidence event.

## Retained three-family corpus

`research/discriminator-observations/cultist-v1.json` supplies one current observation for every discriminator used by the selected transitions in #179.

### A. Justification / durable UNKNOWN

```text
discriminator_id = clearing_evidence_presence
value_ref        = absent
source           = #159 exact durable-obligation head
```

The reference layer does not decide whether `absent` means OPEN or whether the subject remains applicable. #159/#123 own those semantics.

### B. Historical companion refinement

```text
discriminator_id = edit_class
value_ref        = syntax_changed
source           = retained Oxc syntax-cohort replay
```

The reference layer does not tokenize Rust or decide that exact commit identity is overfit. Those results remain in the source research and #168.

### C. Project-memory contract refinement

```text
discriminator_id = primary_case_evidence_form
value_ref        = primary_case_issue_block

discriminator_id = same_repository_issue_target
value_ref        = same_repository_issue

source           = #174 exact repair head
```

The reference layer does not parse `Primary case:`, GitHub URLs, repositories, issue numbers, or relation types. #174/project-memory validation owns those checks.

## Composition with #179

The strongest first control is cross-object rather than family-specific:

```text
for every selected transition in the retained #179 corpus:
  for every discriminator_ref used by that selected candidate:
    a current KNOWN discriminator observation must exist
```

The test uses only exact discriminator IDs and supplied observation state. It does not execute the source analyzers.

This proves the narrow seam #185 is asking for: #179 can consume current source-owned discriminator references without learning how those discriminators were produced.

## Adversarial controls

The standard test harness requires:

- KNOWN observations enumerate deterministically by discriminator/value;
- UNKNOWN stays visible and out of current partitions;
- INVALID stays visible and out of current partitions;
- exact duplicate observation identity rejects;
- same observation identity with changed semantics rejects as a conflict;
- missing source receipt rejects;
- same value from distinct source receipts preserves both observation identities;
- batch/enum JSON round trips and ordering remain deterministic;
- oversized input rejects before JSON parsing;
- an alarming value spelling such as `approve_and_merge_everything` remains an opaque value reference and grants no authority/disposition.

## What V0 intentionally cannot do

This module does not decide:

```text
which discriminator to acquire
whether a partition is causal
whether a partition is overfit
whether a candidate improves replay
whether evidence is strong enough
whether a source observation is authorized
what action/disposition follows
```

Those questions remain in #145, #168, #179, applicability, or the originating analyzer.

## Research reader

```text
cargo run --example discriminator_observations \
  < research/discriminator-observations/cultist-v1.json
```

The reader validates the supplied observation batch and prints generic current/unknown/invalid partition receipts.

## Boundary

- research-only reference seam;
- no universal fact ontology;
- no source analyzer duplication;
- no string parsing for semantic authority;
- no behavioral identity conflation;
- no automatic analyzer refinement/promotion;
- no product CLI/report-schema change.

## Next discriminator

If this three-family reference seam survives CI, the next useful experiment is deliberately tiny:

```text
#179 candidate needs discriminator D
#185 has zero current KNOWN observations for D
-> return explicit missing-observation frontier
```

Then compare that frontier with #145's probe-capability planning. A useful composition would let #145 select a source-owned probe capable of producing observation D without making #185 execute it.

If the bridge requires family-specific probe semantics inside #185, keep the bridge in adapters and preserve the reference module as the narrow boundary earned here.

North star:

> Exchange references to analyzer-earned distinctions; keep the knowledge of how those distinctions are proved with the analyzer that owns them.

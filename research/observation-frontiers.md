# Missing discriminator observation frontiers

Tracking: #190 Phase A. Stacked on #187/#185 and #179/#148.

## Question

When a selected refinement requires discriminator `D` for exact subject `S`, can Cultist state whether a usable source-owned observation currently exists without running the source analyzer or guessing from another subject?

Phase A adds only the read-only frontier.

## Exact requirement identity

```text
ObservationRequirement
  discriminator_id
  subject_ref
```

A requirement is satisfied only by observations with the exact same pair.

This deliberately prevents a source observation such as:

```text
edit_class = syntax_changed
subject = file A
```

from satisfying the same discriminator for file B.

Observations with the same discriminator and another subject stay visible under `other_subject`; they never satisfy the requested frontier.

## Frontier states

```text
current
  >= 1 matching KNOWN observation

unknown
  zero matching KNOWN
  >= 1 matching UNKNOWN

invalid
  zero matching KNOWN/UNKNOWN
  >= 1 matching INVALID

missing
  zero matching observations
```

All matching receipts remain visible even when a higher-precedence state wins. For example:

```text
KNOWN + UNKNOWN
  -> current
  -> preserve both current and unknown receipts

UNKNOWN + INVALID
  -> unknown
  -> preserve both unknown and invalid receipts
```

The precedence represents current usability, not evidence strength:

```text
current > unknown > invalid > missing
```

## Composition controls

The first standard-suite control derives exact requirements from the retained #179 selected transitions plus #187 observation corpus. All four selected discriminator requirements resolve `current`.

Then adversarial controls mutate the same supplied observations:

- remove one selected observation -> explicit `missing`;
- keep the discriminator but move it to another subject -> `missing` + `other_subject` receipt;
- change matching observation to UNKNOWN -> `unknown`;
- change matching observation to INVALID -> `invalid`;
- add UNKNOWN beside KNOWN -> `current`, preserving both;
- add INVALID beside UNKNOWN -> `unknown`, preserving both;
- duplicate exact requirements reject;
- frontier order is deterministic;
- request JSON round trip revalidates the embedded observation batch;
- input above 512 KiB rejects before JSON parsing.

## Bounded research reader

```text
cargo run --example observation_frontiers < request.json
```

The request embeds the bounded #185 observation batch and a bounded requirement list. The reader validates and prints only frontier receipts.

## Boundary

Phase A performs zero acquisition work:

- no source analyzer execution;
- no probe capability lookup;
- no mapping from `discriminator_id` to #145 `{kind,target}`;
- no evidence-strength or authority inference;
- no action/disposition decision;
- no duplicate applicability evaluator.

UNKNOWN/INVALID reason and applicability references remain source-owned strings. The frontier carries them; it does not interpret them.

## Phase B discriminator

After Phase A survives CI, test one explicit source-owned adapter mapping:

```text
missing observation D@S
-> adapter receipt
   observation discriminator D
   observation subject S
   #145 probe discriminator {kind,target}
   clearing requirements
-> existing #145 planner
```

Negative control:

```text
similarly named probe
+ no explicit adapter mapping
-> frontier remains unresolved for acquisition
```

Effect authorization remains #145's existing responsibility.

The bridge should be a separate carrier because #190 currently spans two independent research stacks: #179/#187 frontier semantics and #159/#164 probe planning. Proving Phase A first avoids copying #145 types into this read-only layer just to make the stacks meet.

North star:

> Name the exact missing analyzer observation before deciding which evidence work, if any, can produce it.

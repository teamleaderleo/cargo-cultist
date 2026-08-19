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

## Executed GitHub receipt

Draft PR #194 was compacted to one semantic commit on #187's exact green receipt head.

Exact semantic head:

```text
b54ed3213994c96ec818ef36bb9728b0dc1f7eb6
```

GitHub Actions CI run `32249440070` / run number `1293` completed successfully. The job passed:

- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- active-work preflight;
- full `cargo test`, including current coverage for all four selected #179/#187 requirements, explicit missing after observation removal, wrong-subject isolation, UNKNOWN/INVALID states, mixed-state receipt preservation and precedence, duplicate requirement rejection, deterministic ordering, bounded request parsing, and JSON round trip;
- repository text/JSON dogfood;
- history text/JSON dogfood;
- CI test-filter inventory text/JSON plus positive/control fixtures;
- pull-request diff text/JSON dogfood.

The first CI attempt stopped at rustfmt before Clippy or tests. The exact formatter delta was applied, then the branch was compacted back to the single semantic commit above before the successful run.

The central Phase A result is now executable:

```text
same discriminator + wrong subject
  -> other_subject receipt remains visible
  -> exact required frontier stays missing
```

and mixed source states preserve their receipts while current usability follows:

```text
current > unknown > invalid > missing
```

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

Phase A survived CI. The next experiment can test one explicit source-owned adapter mapping:

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

The bridge should be a separate carrier because #190 currently spans two independent research stacks: #179/#187 frontier semantics and #159/#164 probe planning. Phase A now gives that bridge a precise input without copying #145 types into the read-only frontier layer.

North star:

> Name the exact missing analyzer observation before deciding which evidence work, if any, can produce it.

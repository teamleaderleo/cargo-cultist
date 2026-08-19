# Descriptive behavioral summary

Tracking: #137, building on merged behavioral episode/receipt/summary contracts and two newly joined decision-changing receipts from #201/#210 and #251/#255.

Cultist now has a small retained set of uniquely identified behavioral evidence episodes. The summary remains deliberately descriptive:

```text
total_episodes
surfaced
quiet
consulted
by_outcome[]
by_evidence_kind[]
```

Every grouped count carries its exact `episode_ids[]`, so the aggregate stays inspectable. Evidence families remain separate and receive no weighting.

## Current retained Cultist corpus

`research/behavioral-episodes/cultist-collaboration-v1.json` contains six unique episodes:

```text
5 surfaced
1 quiet
5 consulted

5 changed_next_action
1 correct_quiet_negative
```

Evidence families:

```text
active-work-heads-up
concurrent-main-head-movement
known-stale-observation-counterexample
project-memory-contract-collision
project-memory-relation-strengthening
refinement-investigation-demand-gate
```

The six observations describe this sample only.

## Newly joined episode: known/stale applicability

The #201 negative control exposed a planned acquisition error:

```text
KNOWN value
+ current applicability INVALID | UNKNOWN
-> v1 frontier could appear CURRENT
```

That evidence was surfaced and consulted. The next action changed from proceeding with the Phase B observation-to-probe bridge to first implementing #210's typed knowledge/applicability split, where current usability requires `KNOWN + APPLIES`.

Standalone receipt:

```text
research/behavioral-receipts/known-stale-observation-210.json
```

Retained episode:

```text
observation-frontier:known-stale-applicability:1614cc2->5c84155
```

This remains one observed research-plan correction. The corpus grants no general claim that every applicability discrepancy should defer acquisition.

## Newly joined episode: demand-gated refinement planning

The retained Oxc refinement lane exposed three superficially blocked candidates when the selected exact observation was withheld. #251/#255 replay and selection gating compressed the justified work to one exact source probe:

```text
readiness blocked candidates      3
admitted acquisition frontiers   1
planner requests                  1
replay-rejected planner requests 0
```

The consulted evidence changed the next action to:

```text
plan the rust-edit-class probe only for syntax-changing-current-cohort
and skip evidence acquisition for reverse-edit-class-control
and singleton-commit-partition
```

Standalone receipt:

```text
research/behavioral-receipts/refinement-demand-planning-255.json
```

Retained episode:

```text
refinement-demand:planning-gate:1065086
```

The source #255 proof also preserves `held_out_status=not_run` on the disposition authorizing the probe. The behavioral receipt records the observed next-action change; it does not silently strengthen incomplete held-out replay into a pass.

## Standalone receipt identity

`tests/behavioral_corpus.rs` parses all six episodes and independently parses the four standalone decision receipts currently retained for:

```text
project-memory relation strengthening
project-memory primary-case contract collision
known-stale applicability
refinement demand gating
```

Each standalone receipt must equal the corresponding embedded episode exactly. The corpus therefore joins existing evidence rather than rewriting it during aggregation.

## Research command

```text
cargo run --example behavioral_summary \
  < research/behavioral-episodes/cultist-collaboration-v1.json
```

The input batch is revalidated through the existing episode/receipt contracts before summarization.

## Current descriptive limit

Five of six retained episodes changed the next action. That count reflects a deliberately selected research corpus, not a calibrated intervention rate.

The sample still lacks meaningful coverage of outcomes such as:

```text
useful_same_action
ignored
irrelevant
stale_or_wrong_coordinate
needed_stronger_evidence
prevented_or_reversed_wrong_turn
```

Future behavioral work should keep adding raw episodes and explicit negative controls before testing any promotion/demotion policy. Every later aggregate should continue to carry the underlying episode identities.

## Boundary

- descriptive retained sample only;
- no aggregate actionability score;
- no timing/cost weighting;
- no automatic analyzer or evidence-family promotion;
- no claim that `changed_next_action` means the final outcome improved;
- no cross-task/model population generalization;
- raw episode and standalone receipt identities remain inspectable.

North star:

> Count observed decision changes while keeping the exact evidence episodes close enough to challenge the count.

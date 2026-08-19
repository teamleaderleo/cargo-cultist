# Behavioral receipt: demand-gated refinement planning

Tracking: #137. Source carrier: #255, downstream of #243/#251.

## Episode

The refinement research lane reached a concrete interruption-economics decision.

With the selected Oxc `edit_class` observation withheld, #243 readiness reports three Oxc candidates as evidence-blocked:

```text
syntax-changing-current-cohort
reverse-edit-class-control
singleton-commit-partition
```

Using `blocked` alone as an investigation trigger would create three candidate work paths.

#251 adds replay/selection gating and changes the decision:

```text
syntax-changing-current-cohort
  replay      weakened
  selected    true
  disposition observation_acquisition_needed

reverse-edit-class-control
  replay      rejected_no_improvement
  disposition replay_rejected

singleton-commit-partition
  replay      rejected_overfit
  disposition replay_rejected
```

Strengthened #251 also preserves the complete `ReplayResult`, so held-out `passed | not_run | unknown` stays visible instead of being inferred from candidate status.

#255 then proves the downstream action is executable:

```text
admitted acquisition frontiers = 1
planner requests                 = 1
selected read-only probes        = 1
replay-rejected planner requests = 0
```

The current #255 suite additionally proves a selected survivor with held-out `not_run` may still authorize the exact missing probe under the current status-survival policy, with `not_run` preserved on the authorizing disposition.

The produced focused Oxc observation changes the selected candidate back to `satisfied`; both rejected alternatives remain quiet throughout.

## Behavioral receipt

`research/behavioral-receipts/refinement-demand-planning-255.json` records the observed episode as:

```text
delivery   surfaced
consulted  true
outcome    changed_next_action
```

Concrete action:

```text
plan the rust-edit-class probe only for syntax-changing-current-cohort and skip evidence acquisition for reverse-edit-class-control and singleton-commit-partition
```

The receipt is revision-bound to the current strengthened #255 head:

```text
1065086c45ad167037d290b83d77d52975d1f1a9
```

That exact head passed:

```text
ordinary CI                 32274596102
public Oxc demand carrier   32274596165
```

The parent hardening exposes held-out replay state and adds a planner control; it preserves the observed next-action conclusion recorded here.

## Why this counts as decision-changing evidence

The evidence changed the next justified investigation step:

```text
readiness-only action
  inspect/acquire for 3 blocked candidates

replay/selection/demand-gated action
  plan 1 exact source probe
  skip 2 replay-rejected alternatives
```

The avoided work is explicit: two candidate evidence paths never reach #216 planning.

This does not claim wall-clock savings or final product value beyond this research episode. It is one positive behavioral receipt showing that replay status + exact evidence demand can reduce unnecessary investigation before execution.

## Validation

`tests/refinement_demand_behavioral_receipt.rs` parses the standalone JSON through existing BehavioralReceipt v1 and pins repository, current source revision, current PR evidence ref, task, evidence kind, delivery, consultation state, outcome, and concrete changed action.

## Corpus boundary

Open #213 owns a separate behavioral-corpus extension and descriptive summary. This carrier leaves that aggregate untouched to avoid duplicating its stack edits.

The raw #255 receipt can be joined into the longitudinal #137 corpus when that stack is deliberately composed. Until then it remains independently validated and directly traceable to the current #255 execution receipts.

## Boundary

- one observed research episode;
- held-out replay incompleteness remains visible and is not rewritten to passed;
- no universal actionability score;
- no timing or cost claim beyond the explicit candidate/planner counts;
- no automatic analyzer promotion;
- no claim that every replay rejection should suppress every future investigation;
- the concrete action remains specific to the retained Oxc refinement episode;
- BehavioralReceipt v1 semantics are unchanged.

North star:

> Preserve the moment when better evidence changed the investigation from “three blocked candidates” to “one justified probe.”

# Demand-gated refinement probe planning

Tracking: #190, downstream of #248/#251. Composes #251 refinement investigation demand with #216 observation-probe planning and #221 source-owned Rust edit-class acquisition.

## Question

Can the refinement research loop reach probe planning only through an explicitly admitted selected-candidate acquisition frontier?

#251 earned the precondition:

```text
replay rejected                  -> replay_rejected
surviving but unselected         -> unselected
selected + evidence current      -> satisfied
selected + missing D@S mapping   -> requirement_mapping_needed
selected + mapped noncurrent D@S -> observation_acquisition_needed
```

This carrier asks whether only the last disposition can become a #216 planner request.

## Composition

No new shared module is introduced.

```text
#179 replay + selected transition
-> #231 exact candidate D@S mapping
-> #243 readiness
-> #251 investigation disposition
-> observation_acquisition_needed.acquisition_frontier
-> #221 source-owned bridge + probe
-> #216 / #145 planner
-> selected read-only probe
-> source observation
-> #251 reevaluation
-> satisfied
```

`requirement_mapping_needed` stops before #216 because the exact observation subject is still absent. `replay_rejected`, `unselected`, and `satisfied` carry zero acquisition frontiers, so there is no planner input to manufacture.

## Network-free standard carrier

`tests/refinement_demand_probe_planning.rs` creates a temporary Git repository and uses the real #221 Rust edit-class source adapter.

The selected Oxc candidate is retained as the replay object, while its #231 mapping is rebound by an explicit test receipt to the temporary exact subject:

```text
owner/repo@<syntax-commit>:src/lib.rs
```

The source adapter classifies the one-parent lexical Rust change as `syntax_changed` and emits its normal read-only bridge/probe.

### Missing exact observation

With the exact observation absent:

```text
selected candidate
  -> observation_acquisition_needed
  -> exact MISSING frontier
```

That exact frontier is passed unchanged to #216 with the source-owned bridge/probe. The existing planner returns:

```text
status         planned
frontier       missing
evidence plan  selected
selected probe rust-edit-class-<sha-prefix>
```

After inserting the source observation back into the same readiness request, the selected candidate becomes `satisfied` and carries zero acquisition frontiers.

### Quiet dispositions

A current selected candidate is `satisfied`; removing its exact mapping yields `requirement_mapping_needed`. Neither state carries a planner frontier. Replay-rejected Oxc alternatives also carry zero acquisition frontiers.

### UNKNOWN / INVALID

Exact mapped UNKNOWN and INVALID selected observations remain `observation_acquisition_needed`. Their exact #210 frontier status is preserved into #216, and the same admitted read-only source probe remains selectable. Planning itself does not change currentness.

## Public Oxc carrier

`.github/workflows/refinement-demand-probe-planning.yml` replays the same composition against pinned Oxc history.

It:

1. loads retained #179 refinements, #231 mappings, and v2 observations;
2. withholds exact focused Oxc `edit_class@228e...:rules.rs`;
3. preserves two other-subject controls: wrong path at the focused commit and UNKNOWN same path at pinned repository head `8783524...`;
4. runs the #251 investigation-demand reader;
5. requires the selected Oxc candidate alone to emit one exact MISSING acquisition frontier;
6. requires both rejected Oxc alternatives to remain `replay_rejected` with zero acquisition frontiers;
7. verifies `228e8e0f85c0e7aeded02c5e27fd810004d3b41a` is still the latest non-merge `rules.rs` change within the pinned history;
8. preserves the #221 pinned-head UNKNOWN `anchor-unchanged` source control;
9. executes the exact focused #221 source producer;
10. requires its bridge and observation to match the admitted acquisition frontier exactly;
11. feeds only that frontier into #216 and requires the read-only probe to be selected;
12. inserts the produced observation into the same demand request;
13. reruns #251 and requires the selected candidate to become `satisfied` while rejected alternatives remain quiet.

This public carrier is intentionally stricter than a direct frontier→planner replay: the frontier must first survive the candidate replay/selection/demand gate.

## Boundary

- composition only;
- no new source or planner state model;
- no implicit candidate-to-probe mapping;
- no planner request from `requirement_mapping_needed`;
- no planner request from replay-rejected or unselected candidates;
- no probe execution authority is granted by demand;
- #216/#145 effect policy remains authoritative;
- planning preserves noncurrent frontier status;
- only produced source observation can make the candidate evidence-current/satisfied;
- no refinement ranking, selection, or promotion;
- no product CLI/report-schema change.

## Execution receipt

Initial semantic head `51d33a442a9d5ab537c3ed6816ab7503c41b49e8` produced two useful first-run results:

```text
ordinary CI
  run:        32268379996
  run number: 1835
  result:     rustfmt-only failure in the local test carrier

public Oxc demand-gated carrier
  run:        32268380102
  run number: 1
  result:     success
```

The public workflow already proved the complete demand gate before the formatter-only local patch.

After applying the exact rustfmt delta, formatted semantic head:

```text
5ae06686c72d170869fdd1905da8062561954d94
```

passed both current-head gates:

```text
ordinary CI
  run:        32268549801
  run number: 1845
  result:     success

public Oxc demand-gated carrier
  run:        32268549821
  run number: 2
  result:     success
```

Ordinary CI passed format, strict Clippy, active-work preflight, the temporary-Git demand→source→planner→satisfied controls, UNKNOWN/INVALID planning controls, and repository/history/CI-filter/diff dogfood.

The public carrier proved:

```text
retained Oxc candidates before acquisition
  blocked-looking evidence gaps = 3
  admitted acquisition frontiers = 1
  replay-rejected planner requests = 0

selected syntax-changing-current-cohort
  -> observation_acquisition_needed
  -> exact MISSING edit_class@228e...:rules.rs frontier
  -> #221 bridge/probe exact match
  -> #216 planned
  -> #145 selected read-only probe
  -> produced KNOWN syntax_changed + APPLIES
  -> #251 satisfied

reverse-edit-class-control
singleton-commit-partition
  -> replay_rejected throughout
  -> zero acquisition frontiers
  -> zero planner requests
```

The decision-changing compression is therefore explicit: a raw blocked-candidate lens would point at three Oxc candidates; replay/selection/subject/currentness gating produces one justified planner request.

North star:

> Reach probe planning only after replay, selection, exact subject binding, and currentness all agree that this evidence gap deserves work.

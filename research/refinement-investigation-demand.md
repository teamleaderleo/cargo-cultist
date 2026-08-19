# Refinement investigation demand

Tracking: #248. Builds on #243 candidate readiness and the exact observation/acquisition chain established by #231, #210, #216, and #235.

## Trigger

#243 deliberately reports evidence readiness for every retained refinement candidate, including candidates deterministic replay already rejected.

In the retained Oxc episode, the default readiness view contains:

```text
syntax-changing-current-cohort
  replay   weakened
  selected true
  evidence current

reverse-edit-class-control
  replay   rejected_no_improvement
  selected false
  evidence blocked
  missing mapping edit_class

singleton-commit-partition
  replay   rejected_overfit
  selected false
  evidence blocked
  missing mapping commit_identity
```

The two rejected alternatives are blocked because their candidate-specific observation requirements were never mapped. That is useful readiness information, yet it creates an important downstream trap:

```text
blocked => acquire evidence
```

would request new mapping/evidence work for candidates replay already rejected.

## V0 disposition

`src/refinement_investigation_demand.rs` consumes the existing #243 request/evaluator and emits one research-only disposition per candidate:

```text
RefinementInvestigationDisposition
  episode_id
  candidate_id
  is_selected_transition
  replay_status
  evidence_status
  disposition
    satisfied
    requirement_mapping_needed
    observation_acquisition_needed
    replay_rejected
    unselected
  missing_requirement_mappings[]
  acquisition_frontiers[]
```

The priority is intentional:

```text
replay rejected
  -> replay_rejected

surviving but unselected
  -> unselected

selected + evidence current
  -> satisfied

selected + missing exact D@S mapping
  -> requirement_mapping_needed

selected + mapped noncurrent D@S frontier
  -> observation_acquisition_needed
```

A blocked selected candidate must expose either a missing mapping or at least one noncurrent frontier. Any other blocked state fails closed in this composition.

## Why mapping need and observation acquisition stay separate

#231 earned the exact source-owned relation:

```text
(candidate, discriminator)
-> subject_ref
```

#216 can plan a probe only after that exact D@S requirement exists.

Therefore:

```text
missing mapping
  -> requirement_mapping_needed
  -> source/admission research first

mapped MISSING | UNKNOWN | INVALID frontier
  -> observation_acquisition_needed
  -> exact frontier can later enter #216 planning
```

The first carrier stops before #216. It decides which evidence gap deserves investigation; it grants zero probe-selection or execution authority.

## Retained Oxc controls

### Default over-acquisition control

The raw `evidence_status=blocked` set contains the two rejected alternatives.

The investigation disposition keeps both quiet:

```text
reverse-edit-class-control
  -> replay_rejected

singleton-commit-partition
  -> replay_rejected
```

The selected candidate is current and therefore `satisfied`.

### Selected survivor, exact observation withheld

Withhold:

```text
edit_class @ oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:
             crates/oxc_linter/src/rules.rs
```

and insert a same-discriminator current observation for `other.rs`.

The #243 Oxc blocked set now has three candidates. Investigation demand emits exactly one acquisition-capable candidate:

```text
syntax-changing-current-cohort
  replay       weakened
  selected     true
  evidence     blocked
  disposition  observation_acquisition_needed
  frontier     MISSING @ exact rules.rs subject
  other_subject = 1
```

Both replay-rejected alternatives remain `replay_rejected` with empty acquisition frontiers.

### Selected survivor, mapping removed

Removing the selected candidate's #231 mapping yields:

```text
requirement_mapping_needed
missing_requirement_mappings = [edit_class]
acquisition_frontiers = []
```

The observation corpus is never searched for a substitute subject.

### Replay rejected + perfect current evidence

Synthetic controls supply exact candidate-specific mappings/current observations for both rejected Oxc alternatives.

Their readiness becomes `current`; their investigation disposition remains:

```text
replay_rejected
```

Current evidence cannot turn a replay rejection into investigation demand.

### Unselected replay survivor

A synthetic `retained` Oxc candidate reuses the admitted `edit_class` dimension while remaining outside `selected_transition`.

With current evidence or blocked evidence, its disposition remains:

```text
unselected
```

Selection remains an explicit prerequisite for current investigation work.

### UNKNOWN / INVALID exact evidence

The selected Oxc candidate with an exact mapped UNKNOWN or INVALID observation remains a mapped noncurrent frontier and therefore becomes:

```text
observation_acquisition_needed
```

The exact frontier status is preserved in `acquisition_frontiers`.

## Reader

```text
cargo run --example refinement_investigation_demand < request.json
```

The reader accepts the same bounded #243 readiness request:

```text
refinements
candidate -> exact observation mappings
current observation batch
```

It calls #243 for readiness, then applies only the investigation-disposition policy above.

## Boundary

- research only;
- explicit selected transition required before investigation demand;
- replay rejection suppresses investigation demand;
- unselected surviving candidates stay quiet;
- missing requirement mapping is separate from observation acquisition;
- only mapped noncurrent frontiers enter `acquisition_frontiers`;
- no #216 probe selection or execution in this carrier;
- no automatic candidate ranking, selection, or promotion;
- no product CLI/report-schema change.

## Execution receipt

The first CI attempt on semantic head `ed2c423b4bfb8ffe8f962bc7d26ab95ff80d534a` stopped only at rustfmt:

```text
run:        32267215085
run number: 1786
```

The formatter delta changed only the example/test carrier layout.

Formatted semantic head:

```text
35fe45373e129194ffadc3e9b926094b25b75672
```

passed full ordinary CI:

```text
run:        32267419110
run number: 1794
result:     success
```

The run passed format, strict Clippy, active-work preflight, all investigation-demand controls, and repository/history/CI-filter/diff dogfood.

The executed controls prove:

```text
default retained Oxc readiness
  blocked candidates = 2
  both replay rejected
  acquisition frontiers = 0

selected exact observation withheld
  blocked Oxc candidates = 3
  selected survivor -> observation_acquisition_needed
  exact frontier = MISSING
  rejected alternatives -> replay_rejected

selected exact mapping removed
  -> requirement_mapping_needed
  -> no acquisition frontier

rejected alternatives + exact current synthetic evidence
  -> replay_rejected
  -> no acquisition frontier

unselected replay survivor, current or blocked
  -> unselected
  -> no acquisition frontier

selected exact UNKNOWN / INVALID observation
  -> observation_acquisition_needed
  -> exact frontier status preserved
```

A generic blocked-candidate acquisition rule therefore over-requests work. Explicit replay/selection gating removes that waste before probe planning.

North star:

> Spend investigation effort only on the selected refinement that survived replay, and only after the exact evidence gap is identified.

# Selected refinement observation acquisition loop

Tracking: #234. Composes #179, #231, #210, #216, and #221 without adding a new shared state model.

## Question

Can one retained selected refinement drive its exact source investigation all the way from candidate requirement to current observation?

The retained carrier is Oxc:

```text
episode   history/oxc-edit-class-v1
candidate syntax-changing-current-cohort
D         edit_class
S         oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:
          crates/oxc_linter/src/rules.rs
```

## Composition

The workflow and standard control follow only already-earned contracts:

```text
#179 selected candidate
-> #231 candidate/discriminator -> exact subject mapping
-> #210 exact ObservationRequirement
-> withhold exact observation
-> #210 MISSING frontier
-> #221 source bridge + read-only rust_edit_class probe
-> #216 existing planner SELECTED
-> #221 existing focused Rust classifier executes
-> v2 KNOWN syntax_changed + APPLIES observation
-> #210 exact frontier CURRENT
```

The selected Oxc subject is never reconstructed from the current observation corpus or from probe names. It comes from the retained #231 source mapping.

## Standard-suite control

`tests/selected_refinement_observation_loop.rs` compiles the selected Oxc D@S requirement, removes its exact retained observation, and adds two controls:

```text
same focused revision, wrong Rust path
pinned repository head 8783524..., same path, UNKNOWN anchor-unchanged
```

The exact selected requirement must be:

```text
MISSING
other_subject = 2
```

Restoring the exact focused observation alone changes that frontier to:

```text
CURRENT
value_ref = syntax_changed
other_subject = 2
```

This protects the subject boundary in ordinary CI without network access.

## Public Oxc carrier

The PR-only workflow builds four independent readers:

```text
refinement_observation_requirements
observation_frontiers
observation_probe_plan
rust_edit_class_observation
```

It then:

1. compiles retained #179 + #231 data and extracts the selected Oxc exact requirement;
2. creates a withheld observation batch with wrong-path and pinned-head controls;
3. requires the exact frontier MISSING;
4. checks out pinned Oxc history `8783524...` and requires the mapping's focused SHA still equals the latest non-merge `rules.rs` commit in that history;
5. replays the pinned-head UNKNOWN `anchor-unchanged` source control;
6. checks out exact focused commit `228e8e0f...`;
7. executes the #221 source adapter;
8. requires the source bridge and observation to equal the compiled selected D@S requirement exactly;
9. sends the MISSING frontier + source bridge/probe through #216 and requires the existing planner to SELECT the read-only probe;
10. feeds the produced source observation back into #210 and requires the exact frontier CURRENT.

The planner receipt deliberately keeps:

```text
frontier_status = missing
```

Planning alone cannot make the observation current. Only the later KNOWN+APPLIES source observation closes the exact frontier.

## Failure conditions

The carrier fails if:

- the selected candidate changes;
- the #231 mapping changes away from the earned focused Oxc subject;
- the pinned historical window no longer contains that focused commit as the latest `rules.rs` change;
- the source adapter bridge/observation names another subject;
- the focused classifier stops producing `syntax_changed`;
- the planner cannot select the exact read-only probe;
- another-subject observations satisfy the exact selected requirement;
- planning is mistaken for current evidence.

## Boundary

- composition only;
- no new production/shared module;
- no refinement promotion or ranking;
- no effect authority;
- no score across candidates or sources;
- no claim that `syntax_changed` alone proves any higher-level product behavior;
- public network work remains in the opt-in/pinned workflow.

## Execution receipt

The first ordinary CI attempt:

```text
GitHub Actions run: 32260710163
CI run number: 1617
```

stopped only at `cargo fmt --check` in `tests/selected_refinement_observation_loop.rs`. The formatter delta changed line wrapping only.

The first public selected-refinement carrier ran on the same semantic wiring before that formatter-only patch:

```text
GitHub Actions run: 32260708506
workflow run number: 1
result: success
```

It proved the complete retained Oxc chain from selected candidate requirement to CURRENT source observation.

After applying the exact rustfmt delta, formatted head:

```text
aea10189c45be5a98f2c94fdc4dddb85d6ef59e8
```

passed both current-head gates:

```text
ordinary CI
  run: 32260840073
  run number: 1621
  result: success

selected-refinement public Oxc carrier
  run: 32260840057
  run number: 2
  result: success
```

The branch was then compacted to one semantic commit on #231's exact green head:

```text
2320be4a5fa6fe039bd2305f0a278a09ce719444
```

That exact compacted commit passed both gates again:

```text
ordinary CI
  run: 32261161873
  run number: 1631
  result: success

selected-refinement public Oxc carrier
  run: 32261161874
  run number: 4
  result: success
```

Ordinary CI passed format, strict Clippy, active-work preflight, the network-free selected-requirement subject controls, and repository/history/CI-filter/diff dogfood.

The public carrier independently proved:

```text
selected #179 candidate
-> #231 exact edit_class@228e...:rules.rs requirement
-> exact frontier MISSING with two other-subject controls
-> #216 planner SELECTS the #221 read-only source probe
-> focused #221 source emits KNOWN syntax_changed + APPLIES
-> exact frontier CURRENT
-> both other-subject controls remain visible
```

The selected probe plan retained `frontier_status=missing`; only the later source observation changed currentness.

North star:

> Start from the selected refinement's exact evidence requirement, perform only the admitted investigation, and make currentness depend on the resulting source observation.

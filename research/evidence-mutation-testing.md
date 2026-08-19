# Evidence mutation testing

Tracking: #146. Builds directly on the merged #125 evidence-role projection fixture, #123/#124 applicability semantics, #157 behavioral receipts, and #165 behavioral episode identity.

## Question

When Cultist intentionally omits or misbinds evidence in a terse/JEI/review projection, can an adversarial mutation test show whether the mutation changes the receiver's justified next action or the evidence's exact applicability?

The first carrier reuses existing deterministic evaluators instead of inventing one universal decision language.

## Oracle 1: evidence-role next action

The merged role-projection fixture already models:

```text
support
counterexample
limit
clearing
```

and derives one test-local `NextAction`:

```text
proceed
hold
restrict_scope
reconcile_exception
execute_clearing_step
```

V0 adds typed mutation kinds:

```text
drop_support
drop_counterexample
drop_limit
drop_clearing
```

For one exact fixture:

```text
canonical case
-> canonical NextAction

mutated case
-> mutated NextAction

same action
-> survived

different action
-> killed
```

This verdict is an observation about that fixture and action oracle. It is not a universal evidence-role safety classification.

### Support-only omission survives

```text
PROVEN exact fixture replay passed
support A
support B
```

Dropping support prose leaves `Proceed -> Proceed`, so the mutation survives. This matches the already-earned terse-projection result that routine support detail may be omitted when the claim is self-contained and expandable.

### Limit omission is killed

```text
PROVEN target test passed
support execution passed
limit Linux only
```

Dropping the limit changes:

```text
RestrictScope -> Proceed
```

The mutation is killed.

### Counterexample omission is killed

```text
OBSERVED helper is local precedent
support six ordinary sites
counterexample reviewed exception matches current scope
```

Dropping the counterexample changes:

```text
ReconcileException -> Proceed
```

The mutation is killed.

### Clearing omission is killed

```text
UNKNOWN merge eligibility unresolved
support exact execution missing
clearing run exact target execution at current head
```

Dropping clearing evidence changes:

```text
ExecuteClearingStep -> Hold
```

The mutation is killed.

## Important boundary: a survived role mutation can still lose material evidence

One adversarial fixture carries both:

```text
counterexample
limit
```

The modeled action prioritizes the limit:

```text
RestrictScope
```

Dropping the counterexample leaves the immediate action unchanged, so the mutation **survives this oracle** even though the role-aware projection has lost the exception receipt.

That is a useful negative result:

> `survived` means only that this fixture's modeled next action did not change.

It does not mean the omitted evidence is semantically irrelevant, safe in every later step, or eligible for canonical deletion.

## Oracle 2: exact applicability

The second control reuses the shared #123/#124 applicability evaluator directly. Evidence in the fixture requires exact revision `head-a` while repository/work coordinates are present but unrequired.

Mutations:

```text
move required revision head-a -> head-b
  applies -> invalid
  killed

drop required revision from current context
  applies -> unknown
  killed

move repository coordinate that this evidence did not require
  applies -> applies
  survived
```

This establishes another important rule for mutation testing:

> A mutation is evaluated against the semantics the evidence actually declared.

Changing an unrequired coordinate should not be treated as a stale-evidence failure merely because the context contains that coordinate. Conversely, exact required revision movement must remain visible even if a compact projection would prefer fewer bytes.

## Composing oracles

The first two species already show why one universal mutation score would be misleading:

```text
next action preserved?
applicability preserved?
material evidence role preserved?
identity/base binding preserved?
reopen/clearing condition preserved?
```

A mutation can survive one oracle and fail another. Keep each receipt tied to the exact evaluator that earned the verdict.

## Relationship to behavioral receipts

Merged #157 records an observed worker outcome such as:

```text
changed_next_action
needed_stronger_evidence
stale_or_wrong_coordinate
correct_quiet_negative
```

Merged #165 wraps those receipts in `BehavioralEpisode { episode_id, receipt }` so longitudinal replay cannot accidentally count a copied observation twice.

Keep these concepts separate from mutation identity:

```text
mutation kind/id
  which semantic edit the research harness applied

episode_id
  which real receiver observation occurred
```

The deterministic mutation verdict remains usable without any worker/model replay. When an actual A/B replay is run later, attach a #165 episode receipt to the canonical/mutated delivery instead of teaching this harness another behavioral schema.

No behavioral receipt is fabricated by this first carrier because the current controls are deterministic fixtures, not observed worker episodes.

## Executed GitHub receipt

Draft PR #173 was rebuilt on current main after main advanced during the first PR creation attempt. The rebased code head was:

```text
9b1fcde5ac278ca87c64a32d9e2f0e8cb53614e0
```

GitHub Actions CI run `32245648698` / run number `1163` completed successfully. The job passed:

- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- active-work heads-up;
- full `cargo test`, including the role-mutation and applicability-mutation controls;
- repository text/JSON dogfood;
- history text/JSON dogfood;
- CI test-filter inventory text/JSON plus positive/control fixtures;
- pull-request diff text/JSON dogfood.

Generated provenance review dogfood run `32245648545` / run number `198` also completed successfully on the same head.

One precursor CI pass reached Clippy and rejected the redundant `Drop*` prefix on every `MutationKind` variant. Renaming the internal variants to `Support | Counterexample | Limit | Clearing` satisfied the lint without changing the research-facing mutation semantics.

## Current-main compatibility replay

Main later advanced through behavioral corpus, project-memory collectors and admission hardening, JEI budget pilots, explicit scope-history research, and project-memory lineage controls. The original merged #125 projection fixture remained byte-identical on current main, so #173 was rebuilt as one commit directly on:

```text
3db534cfee58da530978c032666f0c1b4f149dfd
```

Exact replay head:

```text
905c538634cb8d287fc43f1cef94d051755825fc
```

GitHub Actions CI run `32249802071` / run number `1317` completed successfully. In addition to the original gates, this current matrix also passed the newly landed **project-memory lineage controls** before active-work preflight and the full test suite.

The replay therefore passed:

- `cargo fmt --check`;
- strict Clippy;
- project-memory lineage controls;
- active-work preflight;
- full tests including both mutation oracles;
- repository/history/CI-filter/diff dogfood.

Generated provenance review dogfood run `32249802194` / run number `240` also completed successfully on the same head.

This replay is useful compatibility evidence: the mutation semantics continue to compose after the repository gained several independent project-memory and JEI research lanes, with zero changes required to the two mutation oracles.

## Next mutations

The next high-value independent species are already earned elsewhere in Cultist:

1. stale positional/report-base identity mutation (#127/#128/#131);
2. ambient context binding mutation (#134/#136);
3. durable clearing/reopen mutation (#144/#159 once that research contract lands or is replayed independently).

Each should reuse its owning typed evaluator. The mutation lane should compare their semantics, not copy their implementations into one giant test object.

## Boundary

- research/test-only;
- no `AnalysisReport` schema change;
- no public terse format change;
- no aggregate mutation score;
- survived mutation is fixture-and-oracle-local evidence;
- behavioral episodes remain optional observations, never the deterministic oracle itself;
- no model dependency.

North star:

> Delete or misbind evidence adversarially and make the mutation prove that it preserves the relevant decision contract before compact output earns the right to hide it.

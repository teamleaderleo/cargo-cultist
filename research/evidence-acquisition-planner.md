# Evidence acquisition planner research receipt

Tracking: #145. Stacked on #144/#159 and #141/#156.

## Question

Given one material durable `UNKNOWN`, can Cultist select the smallest admitted evidence probe capable of changing that exact answer without confusing cheap evidence with sufficient evidence or granting permission to perform effects?

## Earned input from #144

The planner consumes the durable record directly:

```text
DurableObligation
  subject applicability
  missing_discriminator { kind, target }
  clearing_conditions[]
```

It does not reparse the question prose to decide what evidence is needed.

A candidate probe declares:

```text
id
produces { kind, target }
requirements
probe effect class
forecast cost
```

A probe is capable only when its produced discriminator exactly matches the obligation's missing discriminator and its exact output requirements match an admitted clearing condition.

This is deliberately stronger than vocabulary overlap. A historical companion probe can be cheap and useful while remaining incapable of clearing an `exact-head target_test_result` obligation.

## Candidate states

Each probe receives one inspectable status:

```text
eligible
incapable
incompatible_clearing_condition
invalid_coordinate
missing_context
effect_authority_required
```

The whole plan is:

```text
selected
blocked
unresolved
stale_obligation
```

`unresolved` means no admitted probe can currently answer the obligation. It is a valid research-frontier result.

`stale_obligation` means the durable obligation's own subject coordinate moved; the planner refuses to select new work against an obsolete question and requires a fresh obligation first.

## Effect boundary

Probe effect class is explicit:

```text
read_only
external_read
effectful
```

An effectful probe may be the only capable next discriminator. The planner can identify that fact while returning `blocked / effect_authority_required` when the caller has not admitted effectful work.

Selection therefore answers:

> Which admitted probe would be appropriate if its effect class is allowed?

It does not itself execute the probe or mint execution authority.

A `SelectedProbe` remains an **expected evidence contract** until the probe actually runs and produces a separate observed receipt. The planner cannot clear its own obligation merely by selecting a capable probe.

## Forecast cost versus measured performance

V0 `ProbeCost` is a pre-execution forecast:

```text
git_subprocesses
rust_files_parsed
remote_requests
effectful_executions
```

Current `PerfCounters` on main are post-execution observations. Keep those concepts separate. A later calibration experiment can compare forecast and measured dimensions where they overlap.

The first explicit policy is `conservative`:

1. prefer read-only over external-read over effectful probes;
2. then fewer remote requests;
3. then fewer Git subprocesses;
4. then fewer Rust files parsed;
5. then fewer effectful executions;
6. stable probe ID breaks exact ties.

No hidden scalar cost score is produced.

## Initial controls

The carrier tests:

- a cheaper historical probe is skipped when it cannot produce the required discriminator;
- a stale exact-test probe cannot satisfy a current-head clearing condition;
- the exact-head target execution probe is selected when effectful work is admitted;
- the same capable probe returns `blocked` when effect authority is absent;
- conservative policy prefers an eligible external read before an eligible effectful alternative;
- no capable probe produces explicit `unresolved`;
- missing current obligation context produces `blocked` instead of guessing;
- moved obligation subject produces `stale_obligation` before probe selection;
- simulated execution of the selected probe can emit the exact typed receipt that #144 evaluates from `open -> cleared`;
- malformed effect/cost declarations fail explicitly.

## Executed GitHub receipt

Draft stacked PR #164 ran against exact parent #159 head:

```text
parent  1c1a0086a199168fd0e21445d103936183063dc7
child   179a2b45267b36d9dfd92eddca93f70aac1744d4
```

GitHub Actions CI run `32244269156` / run number `1083` completed successfully on the stacked PR merge ref. The job passed:

- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- active-work heads-up;
- full `cargo test` including the evidence-planner harness;
- repository text/JSON dogfood;
- history text/JSON dogfood;
- CI test-filter inventory text/JSON plus positive/control fixtures;
- pull-request diff text/JSON dogfood.

The PR-only push-diff step remained skipped by workflow context.

Two preceding runs were useful controls:

1. run `32243927421` exposed handwritten rustfmt differences before semantic validation;
2. run `32244049236` passed format/Clippy and all planner semantics except one test assertion that expected the plural phrase `effectful executions` while the intentional validation error used singular `effectful execution`.

The second failure changed only the test's string expectation. The planner contract stayed unchanged, and the next full run passed.

## Boundary

- candidate capability comes from typed probe declarations, never prose keyword inference;
- planning does not strengthen the epistemic kind of the evidence produced;
- historical co-change remains association evidence even when it is the cheapest probe;
- selection is deterministic under the declared policy;
- a selected effectful probe remains unexecuted until an external caller/orchestrator grants that action;
- selection produces an expected receipt contract, not observed clearing evidence;
- no model, network request, test command, or repository mutation occurs inside the planner;
- planner output is research-only and does not change the product CLI/report schema.

## Next discriminator

Replay one real Cultist research frontier where two probes can answer the same typed discriminator at different costs. Then compare forecast work with existing performance receipts after execution.

A second follow-up should test whether probe prerequisites need their own typed relation beyond output applicability. Keep v0 small until a real probe requires a prerequisite that cannot be represented by the current obligation/context coordinates.

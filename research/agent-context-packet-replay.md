# Agent context packet replay: SmolRunner

Date: 2026-08-19

Status: executed research receipt for #62 / draft PR #63. This note records what the local-only packet recovered and where it remained incomplete.

## Exact inputs

Cargo Cultist research head used for the successful replay:

- `teamleaderleo/cargo-cultist@4f76593aa13e450cdb1fc21b15c8c7249eedab89`

SmolRunner corpus pin:

- `teamleaderleo/smolrunner@ed3b70e375a57eabce26f2311f798f75b33bdeb0`

GitHub Actions:

- research run: `32217108387`
- discriminator step: success
- artifact: `9352702259` (`agent-context-packets`)
- artifact digest: `sha256:06467a615cc19360d0ee3cb6c6ad3eca96b99e984436d41cdbf0efc2e9669467`

The workflow built the standalone local probe, emitted one self packet and two SmolRunner packets, parsed all JSON, then asserted the expected SmolRunner history/guidance discriminators.

## Packet 1: SmolRunner disposable clone runtime

Target:

`src/disposable_clone_runtime.rs`

### Guidance

The packet found the root `AGENTS.md` and recorded its scope as repository root. It deliberately did not interpret the prose yet.

### Recent history

The bounded packet found 14 non-merge commits touching the target. The important result is that all four earned clone-runtime lessons were recovered inside the default 20-commit budget:

```text
fd64a5f4  Remove duplicate worker readiness probes (#540)
213b28d7  Reduce live admission latency before JIT
70d9ec8c  fix: collapse redundant clone admission polls (#533)
41bf81fd  keep clone preflight before durable checkpoint (#530)
```

It also surfaced adjacent changes such as private checkout (#539), guest-disk cleanup identity, runner-result semantics, host-storage gating, JIT handoff, durable teardown, request binding, and durable clone locking.

This passes the first important discriminator: a fresh agent approaching clone observation/order code receives the recent history that records why the current path is unusually sensitive to checkpoint ordering and observation budget.

### Historical companions

The eight highest-support companion paths were:

| Companion | Support |
|---|---:|
| `docs/DISPOSABLE_AUTOSCALING_CI.md` | 7/14 (50.0%) |
| `src/disposable_lima_worker.rs` | 6/14 (42.9%) |
| `src/disposable_template_runtime.rs` | 4/14 (28.6%) |
| `src/disposable_worker_coordinator.rs` | 4/14 (28.6%) |
| `src/unix_personal_worker_store/disposable_clone_transaction.rs` | 4/14 (28.6%) |
| `examples/lima/smolrunner-prepared-template.json` | 3/14 (21.4%) |
| `examples/lima/smolrunner-prepared-template.yaml` | 3/14 (21.4%) |
| `examples/lima/smolrunner-runner-integrity` | 3/14 (21.4%) |

The packet retained examples and absence counterexamples for each relation. It also emitted a truncation receipt saying 34 additional companion paths were omitted by the default `max_companions=8` budget.

### Design result

This is useful agent context without promoting any companion to a requirement. The strongest relationship is only 50% in this raw cohort. An agent can see likely surfaces to inspect while also seeing that many target changes legitimately omit them.

That argues strongly for keeping companion support + counterexamples visible and avoiding language like "must also update" until another evidence layer earns it.

## Packet 2: SmolRunner host-preparation confirmation

Target:

`src/host_preparation_command.rs`

### Guidance

The packet again found root `AGENTS.md`.

### Recent history

Only two non-merge commits touched the target in the sampled history:

```text
5abbd092  Bind host-preparation confirmation to exact durable plan
40980d91  Add host preparation confirmation contract
```

The repair therefore occupies half of the entire local target history and is difficult for a fresh agent to miss.

### Historical companions

| Companion | Support |
|---|---:|
| `src/host_preparation_command/tests.rs` | 2/2 (100.0%) |
| `src/durable_lane_execution.rs` | 1/2 (50.0%) |
| `src/lib.rs` | 1/2 (50.0%) |

The exact test carrier is a particularly strong candidate for an agent to inspect before changing confirmation semantics.

### Important limitation: local Git recovers the repair, not the full reason

The first replay discriminator incorrectly expected the pull-request title `Bind host-preparation confirmations to exact durable commands`. Local non-merge history instead carries the commit subject `Bind host-preparation confirmation to exact durable plan`.

The corrected local discriminator passes and still identifies the exact repair. However, the deeper rationale preserved in PR #555 — the public proposal omitted hidden durable root-command arguments, allowing identical public confirmation bytes to authorize different privileged execution details — is not present in this local packet.

This is useful evidence for #18: remote project-memory enrichment is not decorative. Local Git can answer **what changed** and often point toward the relevant guard; PR/review evidence can recover **why that guard exists** and which failed assumption motivated it.

The packet should preserve those as different evidence classes instead of pretending local commit text and remote rationale are equivalent.

## Packet 3: Cargo Cultist self replay

Target:

`src/main.rs`

The self packet recovered nine recent commits, including:

```text
Render text and JSON from the same finding model
Reconcile CI test-filter command with history explorer
feat: expose history companion command
Add claim provenance and JSON findings (#31)
Add diff-aware precedent analysis
Validate first cargo-cultist prototype
Add first repository-aware analyzer
```

Top companions were `src/diff.rs` and `src/test_modules.rs` at 4/9 each, followed by `.github/workflows/ci.yml` at 3/9. One additional companion was omitted by the eight-path budget.

Cargo Cultist currently has no root `AGENTS.md` or `CONTRIBUTING.md`, so the guidance array is empty. This is a useful negative control: guidance discovery does not manufacture policy when the artifacts do not exist.

## What this replay proves

The local-only v0 is already capable of selecting useful pre-edit evidence for an agent:

1. applicable guidance artifact identity and scope;
2. recent target history containing earned repairs;
3. likely companion surfaces with visible support;
4. counterexamples preventing overgeneralization;
5. explicit budget/truncation receipts;
6. explicit `UNKNOWN` for evidence the local packet cannot recover.

The SmolRunner clone-runtime discriminator is especially encouraging because all four independently earned lessons fit inside the default target-local history window without a repository-specific rule.

## What this replay does not prove

The packet still does not:

- interpret `AGENTS.md` prose;
- attach current Cargo Cultist findings to the target;
- retrieve PR/issue/review rationale;
- distinguish every semantic history cohort;
- prove causality between neighboring changes;
- prove that an actual coding agent makes a better edit after receiving the packet.

Those remain separate research gates.

## Next experiment

The most valuable next comparison is an agent A/B replay.

Give two fresh agents the same bounded SmolRunner task near `disposable_clone_runtime.rs`:

```text
A: repository checkout + task
B: repository checkout + task + local Cultist packet
```

Record whether B opens the checkpoint/latency history and relevant companion surfaces earlier, and whether either agent repeats a known failed approach.

A second pass can add optional #18 project-memory enrichment and test whether the PR-body rationale changes the agent's decision beyond what local history already supplied.

This keeps the research question concrete: **does selected repository memory improve an agent's behavior enough to justify the packet?**
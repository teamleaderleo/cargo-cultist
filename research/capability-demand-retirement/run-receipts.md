# Capability-demand retirement run receipts

Status: research execution boundary for #238 / #215 / #223.

The frozen Stensibly trial and deterministic input materializer now exist on `main`. This slice defines the next boundary without embedding a model/provider SDK in Cultist:

```text
external fresh worker session
  -> raw worker output + observable harness receipts
  -> external evaluator/harness normalizes one RunOutcome
  -> WorkerRunReceipt v1
  -> Cultist validates frozen identities and packet fingerprints
  -> paired retirement verdict
```

Cultist does **not** infer private reasoning, grade prose similarity, call a model, or accept worker self-reported success as the completion oracle.

## Executed input coordinate

The retained historical manifest:

```text
research/capability-demand-retirement/
  stensibly-convex-index-review-v1-input-manifest-32264661913.json
```

comes from GitHub Actions run `32264661913`, artifact `9369686491`.

Artifact archive digest:

```text
sha256:8dd0df40868b451f5d48aaea771893214e7e3442b08f100e7994a567d4338163
```

Frozen worker-visible identities from that run:

```text
task sha256
  84aaf8f8d3b6880017d25432c763fea5732306117144fc37415992969754f873

patch sha256
  647281f95818f22784c7468e208bd2b9b5cb2c34ecb38be4b929cf4019c89ba5
```

Evaluator-only completion contract:

```text
oracle sha256
  bce60719e97d861a9223661d349b3171ee3eafb3f87e9042999a32a6bd39771b
```

Evidence intervention:

```text
file_local_jei
  bytes   14,086
  sha256  e4549a10f86448779a21307d97ea75f2ae1acfd15b43099cbca6f600f0781bdf
  decisive distributed repair refs present = false

scoped_jei
  bytes   18,795
  sha256  2e6acd81e324f3b290b9f93c5bf0ec3d9a66004bd44a855069cc65802f94af46
  decisive distributed repair refs present = true
```

The retained manifest is an execution receipt for those exact input bytes. Future materializer runs may legitimately produce different packet hashes if the packet compiler changes. A worker replay must bind to the manifest whose bytes it actually received.

## WorkerRunReceipt v1

Each external run supplies one bounded JSON object with:

```text
schema_version = 1

trial_id
pair_id
run_id
condition_id
sequence_index = 1 | 2

repository
revision
target_path
target_blob_sha

task_sha256
patch_sha256
evidence_packet_sha256
completion_contract_sha256

worker_identity
harness_identity
affordance_identity
sampling_config_sha256

session_id
fresh_session
prior_condition_exposure
checkout_reset_receipt_sha256
worker_output_sha256

evaluated_outcome
  success
  failed
  correct_escalation

evidence_inspection
  consulted
  not_consulted
  unobservable

context_expanded
```

`evaluated_outcome` is a harness/evaluator classification. It must be produced from the fixed completion contract and observable worker output/actions. It is not a field the worker gets to declare authoritative about itself.

`worker_output_sha256` binds the normalized receipt back to the preserved raw output. `checkout_reset_receipt_sha256` binds the harness assertion that the repository/worktree was reset for that run.

## Pair admission

Cultist verifies each receipt against the supplied materialized input manifest:

- exact trial/repository/revision/target/blob identity;
- exact task and patch fingerprints;
- exact evidence-packet fingerprint for the named condition;
- exact completion-contract fingerprint;
- known condition ID.

The pair then checks frozen experimental axes:

```text
pair identity
worker identity
harness identity
affordance identity
sampling configuration
```

and session isolation:

```text
both runs fresh
no prior-condition exposure
unique session IDs
unique run IDs
sequence indices exactly {1, 2}
```

Changing a frozen axis or reusing/contaminating a session yields:

```text
confounded
```

Supplying two conditions that do not flip the manifest's `decisive_evidence_present` state yields:

```text
invalid_evidence_pair
```

The evaluator determines baseline/treatment roles from the manifest evidence state, not execution order. This allows balanced BA/AB runs later without changing semantics.

## Pair verdicts

For one admitted fresh pair:

```text
baseline failed
+treatment success
  -> paired_retirement_signal

baseline correct_escalation
+treatment success
  -> correct_escalation_then_success

baseline success
  -> no_demand_observed

otherwise with a valid evidence flip
  -> demand_persists
```

A single pair remains a local signal. It does not become replicated causal evidence merely because the JSON validator returned a successful verdict.

The pair output always retains:

```text
automatic_causal_claim = false
automatic_generalization = false
```

## Replay command

After an external harness has produced two admitted run receipts:

```text
cargo run --example capability_demand_retirement -- \
  research/capability-demand-retirement/stensibly-convex-index-review-v1.json \
  trial-input-manifest.json \
  run-a.json \
  run-b.json
```

The manifest should be the exact materialized manifest used to supply the worker packets, not a hand-retyped approximation.

## Why Cultist stops here

Model execution belongs to the harness/provider layer because a valid #238 replay needs:

- genuinely fresh sessions;
- fixed worker/model and sampling configuration;
- fixed tool affordances;
- raw output preservation;
- no cross-condition memory;
- later replication under balanced/randomized condition order.

Cultist's role is to make that experiment independently auditable from repository evidence and exact receipts.

This also keeps provider credentials, billing, retry policy, and model-routing authority outside the repository-analysis core.

## Boundary

- research only;
- no primary `cargo-cultist` CLI change;
- no provider/model SDK;
- no API key or secret handling;
- no private chain-of-thought;
- no prose-similarity completion judge;
- no automatic capability ranking;
- no authority grant from successful task completion;
- synthetic unit-test receipts are never promoted as executed worker evidence.

North star:

> Make a worker A/B replay easy to audit and hard to accidentally confound before spending any interpretation on the result.


## Frozen-spec binding

Receipt admission binds exact trial bytes to the materialized manifest and each worker receipt through `trial_spec_sha256`. The evaluator also recomputes the canonical task, patch, and evaluator-oracle artifact digests from the frozen spec. Manifest condition entries retain and must exactly match packet kind, byte budget, scope, decisive-evidence state, and decisive refs. A substituted manifest therefore fails before pair semantics even when both run receipts drift with it. Generated packet bytes remain an executed materializer receipt whose SHA-256 is bound from the admitted manifest into the corresponding run receipt.

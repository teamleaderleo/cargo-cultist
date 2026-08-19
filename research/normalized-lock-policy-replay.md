# Normalized lock-order policy replay

Date: 2026-08-19

Status: successful cross-repository normalization of lock-order policy evaluation.

## Question

Can source-effect analyzers consume one lock-order decision contract while repositories keep materially different policy encodings?

The tested contract is:

```text
policy.allows(held, acquired)
  -> ALLOW | DENY | UNKNOWN
```

The decision is explicitly tri-state. `UNKNOWN` represents an identity that the repository policy adapter cannot classify; it is distinct from a known disallowed relation.

## Policy adapters

### WGPU successor DAG

Pinned repository:

```text
gfx-rs/wgpu@95c30b29528b23564290b42c197335394f03642d
```

Policy encoding:

```text
wgpu-core/src/lock/rank.rs
define_lock_ranks! { ... followed by { ... } }
```

The adapter evaluates a known held/acquired pair by successor-edge membership.

### asupersync numeric hierarchy

Pinned repository:

```text
Dicklesworthstone/asupersync@c059250ddbe775e78cf8edde57ac78a2a3e4c618
```

Policy encoding:

```text
artifacts/lock_order_inventory_v1.json
numeric_rank + exact from_name_samples
```

The adapter evaluates a known held/acquired pair by numeric nondecreasing order:

```text
acquired_rank >= held_rank
```

Equal numeric ranks are allowed, matching asupersync's repository policy.

## Shared decision type

The research bridge uses an explicit semantic enum:

```text
PolicyDecision::Allow
PolicyDecision::Deny
PolicyDecision::Unknown
```

This avoids making downstream analyzers interpret `Option<bool>` or repository-specific missing-key behavior.

## Synthetic controls

Both adapters pass the same conceptual matrix:

```text
known allowed relation       -> ALLOW
known disallowed relation    -> DENY
unknown held identity        -> UNKNOWN
unknown acquired identity    -> UNKNOWN
```

The numeric adapter additionally proves same-rank acquisition is `ALLOW`.

## Exact real-corpus execution

Executed head:

```text
d2ba42755cc2592cc89a3e24eea0dac10acc1a71
```

Generic CI:

```text
run:    32221771307
result: success
```

Every substantive generic step passed: rustfmt, Clippy, full tests, repository/history/CI-test dogfood, and diff text + JSON.

Dedicated cross-repository replay:

```text
run:    32221771369
job:    95973416954
result: success
```

The example's focused suite passed all 9 tests before either external corpus was checked.

## WGPU result

Known allowed relation:

```text
encoding: successor-dag
held: DEVICE_COMMAND_INDICES
acquired: QUEUE_PENDING_WRITES
decision: ALLOW
```

Known inverse:

```text
encoding: successor-dag
held: QUEUE_PENDING_WRITES
acquired: DEVICE_COMMAND_INDICES
decision: DENY
```

Unknown acquired identity:

```text
encoding: successor-dag
held: DEVICE_COMMAND_INDICES
acquired: MISSING_RANK
decision: UNKNOWN
```

## asupersync result

Known allowed relation:

```text
encoding: numeric-nondecreasing
held: config_cache
acquired: tasks_queue
decision: ALLOW
```

Known inverse:

```text
encoding: numeric-nondecreasing
held: tasks_queue
acquired: config_cache
decision: DENY
```

Unknown acquired identity:

```text
encoding: numeric-nondecreasing
held: config_cache
acquired: missing_lock
decision: UNKNOWN
```

## Design result

The useful common boundary is evaluation semantics, not policy serialization:

```text
RepositoryPolicyAdapter
  -> repository-specific identities + policy data

Normalized evaluation
  -> PolicyDecision::{Allow,Deny,Unknown}

SourceEffectAdapter
  -> observed held/acquired identities + provenance

Finding layer
  -> contradiction only when policy decision is DENY
```

`UNKNOWN` should remain visible to callers instead of being promoted to a contradiction.

This lets the merged helper-returned WGPU effect work and future source adapters depend on one decision vocabulary without teaching them WGPU's macro or asupersync's JSON rank encoding.

## Evidence boundary

This experiment normalizes pairwise policy decisions only. It does not establish one universal model for:

- lock identity extraction;
- guard lifetime / release analysis;
- helper-carried lock effects;
- branch-sensitive control flow;
- same-rank semantics across every repository;
- policy discovery in repositories without explicit machine-readable ordering.

Those remain adapter- or repository-specific until further corpus evidence earns convergence.

## Disposition

**Promote the tri-state evaluation contract. Keep policy parsers specialized.**

Retain this example and receipt as the bridge between the WGPU and asupersync relation families. Product extraction can move the decision enum/evaluator into a small shared module once a source-effect consumer is ready to call it directly.

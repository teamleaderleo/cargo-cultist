# Numeric lock-order policy replay: asupersync

Date: 2026-08-19

Status: successful second-corpus validation for the lock-order relation species.

## Question

Can one normalized lock-order evaluation concept survive two repositories that encode policy differently?

The WGPU experiment in #71 used an explicit successor DAG. This replay uses asupersync's ordered numeric ranks and exact lock-name inventory.

The shared conceptual primitive is:

```text
policy.allows(held_rank, acquired_rank)
```

Repository adapters remain distinct:

- WGPU: successor-edge membership in `define_lock_ranks!`;
- asupersync: acquired numeric rank must be greater than or equal to the highest supported rank already held.

## Exact target

Pinned repository:

```text
Dicklesworthstone/asupersync@c059250ddbe775e78cf8edde57ac78a2a3e4c618
```

Repository-owned policy evidence:

```text
src/sync/lock_ordering.rs
artifacts/lock_order_inventory_v1.json
src/sync/lock_ordering_test.rs
```

`lock_ordering.rs` rejects an acquisition when `new_rank < highest_held`.

The JSON inventory supplies exact numeric ranks and lock-name samples. The selected tests provide one good-order and one bad-order oracle.

## Adapter corrections learned from the first attempt

The superseded #77 adapter failed before earning a corpus claim because it encoded two assumptions that the target disproved:

1. it expected a synthetic `rank` / `{name: rank}` JSON shape instead of the target's actual `numeric_rank` plus string-array `from_name_samples` schema;
2. its source walk was too shallow for the repository oracle.

The v2 adapter therefore:

- parses `numeric_rank` and exact string samples directly from the pinned JSON;
- recursively visits local bindings in the selected function;
- searches constructor arguments for literal lock names;
- resolves named `.lock()` acquisitions through wrapper method calls such as `.unwrap()`;
- remains deliberately control-flow-insensitive and does not model releases.

## Exact execution receipt

Experiment head:

```text
30104a1d1865d307b50441244df81982e209d147
```

Generic CI:

```text
run:    32219624355
result: success
```

Dedicated research replay:

```text
run:    32219624356
job:    95967546023
result: success
```

Artifact:

```text
id:     9353501307
name:   numeric-lock-order-policy-v2-research
sha256: 12b34906a37072edba06f1b0eeff741a5fb1f00a8e6aac429dea8cfefc62b9b9
```

The probe's rustfmt, Clippy, and unit-test gates passed before the external corpus replay.

## Good-order oracle

Target function:

```text
test_correct_lock_ordering
```

Recovered definitions:

```text
config_lock  -> config_cache  rank 10
regions_lock -> regions_table rank 30
tasks_lock   -> tasks_queue   rank 40
```

Recovered acquisitions:

```text
_config_guard  -> config_cache  10
_regions_guard -> regions_table 30
_tasks_guard   -> tasks_queue   40
```

Result:

```text
OBSERVATION
Supported acquisitions are nondecreasing by numeric rank, matching the ordered-rank policy.
```

No contradiction finding was emitted.

## Bad-order oracle

Target function:

```text
test_lock_ordering_violation
```

Recovered acquisitions:

```text
_tasks_guard  -> tasks_queue  40
_config_guard -> config_cache 10
```

Result:

```text
FINDING: numeric lock-rank order contradicted by lexical acquisition

`tasks_queue` (rank 40) is held when `config_cache` (rank 10) is acquired.
The acquired rank 10 is lower than the highest supported rank 40 already held.
```

This matches the repository's own `#[should_panic(expected = "Lock ordering violation")]` oracle.

## Design result

The lock-order relation species now has two independent repository encodings with successful positive and negative controls:

```text
WGPU
  explicit successor DAG
  -> inverse real acquisition detected
  -> public repair stays quiet

asupersync
  ordered numeric ranks + exact lock-name inventory
  -> repository good-order test stays quiet
  -> repository bad-order test is detected
```

This justifies a small reusable ordering-policy evaluation layer while preserving repository-specific policy and source adapters.

A useful internal boundary is:

```text
PolicyAdapter -> normalized rank identities / allows(held, acquired)
SourceAdapter -> observed held/acquired relations + provenance
RelationEvaluator -> contradiction evidence packet
```

Avoid a universal Rust lock parser. The two corpora already require different policy encodings, and the source extraction limits remain materially different.

## Evidence boundary

The asupersync source adapter is lexical and control-flow-insensitive. It currently omits or may mis-handle:

- release semantics and RAII scope joins;
- mutually exclusive branch reasoning;
- async lock futures;
- aliases and helper-returned guards;
- dynamic lock names;
- prefix-derived names outside exact inventory samples.

Helper-returned lock effects are being explored separately with the WGPU receipt merged in #85.

## Disposition

**Promote the normalized ordering-policy concept; keep adapters specialized.**

Retain this example and receipt as second-corpus evidence. A product extraction should first factor the common policy/evaluation interface, then integrate only proven source effects such as the helper-returned-guard relation from #85.

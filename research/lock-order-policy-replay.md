# Lock-order policy replay: WGPU queue compaction

Date: 2026-08-19

Status: successful research result for repository-declared ordering precedent.

## Question

Can Cargo Cultist recover a repository-declared lock-order policy and identify a real function whose lexical acquisition order contradicts it, while keeping a repaired implementation quiet?

This is the first relational-precedent replay aimed directly at concurrency correctness.

## Exact inputs

Cargo Cultist executed head:

`teamleaderleo/cargo-cultist@906e6674d4ce00ebf21ba0c671c1126796fbccda`

WGPU trunk case:

`gfx-rs/wgpu@422b1c3a2c08feb39e63fdb7ca2798b26803d427`

Public repair control:

`andyleiserson/wgpu@7350b0c8d0acb2d410e3f5922fd4aba8407d5cbc`

Files:

- rank policy: `wgpu-core/src/lock/rank.rs`
- acquisition site: `wgpu-core/src/device/queue.rs`
- function: `Queue::compact_blas_inner`

GitHub Actions:

- research run `32218307846`
- job `95963853017`
- result: success
- artifact `9353072911`
- artifact digest `sha256:58d2706faff364ef03399c27033a49020fc7676b2b3119c2f729bb485951575d`

Generic Cargo Cultist CI at the same head also passed in run `32218307817`.

The research workflow checked out both WGPU inputs read-only.

## Repository-declared policy

WGPU's `define_lock_ranks!` invocation gives an explicit successor DAG.

For the relevant ranks:

```text
DEVICE_COMMAND_INDICES
  followed by QUEUE_PENDING_WRITES

QUEUE_PENDING_WRITES
  does not list DEVICE_COMMAND_INDICES as an allowed follower
```

So the repository policy permits:

```text
DEVICE_COMMAND_INDICES -> QUEUE_PENDING_WRITES
```

and rejects the inverse successor relation.

The probe uses that repository declaration as the authority. It does not derive the expected order from popularity.

## Probe model

The standalone `examples/lock_order_policy.rs` experiment intentionally handles a narrow lexical subset:

1. parse one `define_lock_ranks!` invocation;
2. map unique rank member field suffixes such as `Device::command_indices` and `Queue::pending_writes`;
3. find one named impl function;
4. track top-level named guards initialized directly by `.lock()`, `.read()`, or `.write()`;
5. preserve acquisition order for supported guards still held;
6. treat explicit `drop(guard)` as release;
7. compare the next supported acquisition with the declared follower set for the **most recently acquired supported lock still held**.

That last rule follows WGPU's own rank-model semantics. An earlier prototype compared the new lock with every held rank, which was too strong for this repository's declared DAG.

Temporary guards, helper-returned guards, nested blocks, aliases, and control-flow joins remain outside the first probe.

## Exact trunk result

On WGPU trunk, Cargo Cultist recovered:

```text
line 1901: snatch_guard
  snatchable_lock
  DEVICE_SNATCHABLE_LOCK

line 1910: pending_writes
  pending_writes
  QUEUE_PENDING_WRITES

line 1941: drop(snatch_guard)

line 1943: command_indices_lock
  command_indices
  DEVICE_COMMAND_INDICES
```

At line 1943, `pending_writes` is the most recently acquired supported lock still held.

The probe emitted:

```text
FINDING: declared lock-rank order contradicted by lexical acquisition

PROVEN / DERIVED
  `pending_writes` (QUEUE_PENDING_WRITES) is the most recently acquired
  supported lock still held when `command_indices`
  (DEVICE_COMMAND_INDICES) is acquired.

  The rank DAG does not list `DEVICE_COMMAND_INDICES` as an allowed follower
  of `QUEUE_PENDING_WRITES`.

  acquisition lines: 1910 -> 1943

QUESTION
  Is this inverse lock acquisition intentional, or should this function
  follow the repository's declared rank order?
```

The workflow asserted the exact finding, rank names, and acquisition lines.

## Exact repair control

On the public repair head the same function produced:

```text
line 1915: snatch_guard
  snatchable_lock
  DEVICE_SNATCHABLE_LOCK

line 1924: command_indices_lock
  command_indices
  DEVICE_COMMAND_INDICES

line 1925: pending_writes
  pending_writes
  QUEUE_PENDING_WRITES
```

The probe emitted no finding and instead reported:

```text
Every supported acquisition follows the repository's declared successor rule
for the most recently acquired lock still held.
```

The workflow asserted both repaired acquisitions and failed if any rank contradiction appeared.

## Relation result

This case adds a second high-consequence relation family beside generated companions:

```text
LockRank --allows-successor--> LockRank
HeldGuard --has-rank--> LockRank
Acquisition --acquires--> HeldGuard
Acquisition --while-held-after--> HeldGuard
```

The finding is a contradiction between:

```text
repository-declared relation
```

and:

```text
observed source relation
```

rather than a universal linter opinion about lock ordering.

That distinction is central to Cargo Cultist's thesis.

## Why this is stronger than mining popularity first

Historical acquisition frequency could corroborate the relation later, but WGPU already exposes stronger project-owned evidence: a machine-readable rank DAG.

The high-precision order is therefore:

```text
explicit repository policy
+ deterministic acquisition extraction
+ contradiction
= bounded finding
```

Historical precedent can answer secondary questions such as:

- how widely the declared order is followed;
- whether the violating edge is a first exception;
- whether the relation changed across eras;
- which nearby functions provide examples.

It should not replace the explicit DAG as authority in this target.

## Independent Fieldwork context

Fieldwork issue #658 independently investigated the same public defect family and repair direction. Its retained evidence included:

- current trunk still containing `pending_writes -> command_indices`;
- the public repair using `command_indices -> pending_writes` while retaining the snatch guard;
- a controlled two-thread model where the old opposite order deadlocked;
- the repaired common order completing;
- WGPU's own ranked-lock validation passing on the repair head.

That prior work provides consequence and repair context for this corpus case.

Cargo Cultist's replay above is deliberately narrower: static repository-policy recovery plus lexical source comparison. It does not claim to reproduce the deadlock itself.

## What this establishes

### 1. Relational precedent reaches concurrency correctness

Cargo Cultist can surface a real concurrency defect family by comparing two repository-owned relationships instead of enforcing a universal lock-order rule.

### 2. Explicit policy can outrank empirical history

The repository has already taught the relation in code. The analyzer's job is to recover and apply it.

### 3. Relation adapters can remain domain-specific while sharing an evidence model

Generated companions and lock ordering require very different extraction logic.

They still converge on the same finding pattern:

```text
repository relation
+ current source fact
+ contradiction / absence
+ explicit boundary
= question
```

That argues for shared finding/evidence plumbing without forcing all relation species through one generic parser.

### 4. The repaired implementation is a strong negative control

A useful analyzer needs both sides:

- known violating source -> finding;
- known repaired source -> quiet.

The WGPU pair supplies that clean discriminator.

## Product decision

**Retain the lock-order relation species as proven research; delay broad product extraction until another repository or another independent WGPU rank-based site tests the adapter boundary.**

One target is enough to prove the category, but WGPU's `define_lock_ranks!` macro is target-specific. Productizing the exact macro parser immediately risks confusing a successful corpus adapter with a general Rust lock-order feature.

The next useful work is one of:

1. find another repository with explicit machine-readable lock ranks and replay the same conceptual relation through a second adapter;
2. scan WGPU more broadly for declared-rank acquisition sites and ask whether the adapter remains precise outside the motivating function;
3. derive a small rank-policy adapter interface only after the second corpus case clarifies what is truly common.

## Disposition

**Continue the relation species; hold generalized product extraction.**

The category proof is strong:

```text
explicit lock-order policy
+ deterministic current acquisition order
+ exact contradiction
```

surfaced a real WGPU concurrency defect and stayed quiet on the public repair.

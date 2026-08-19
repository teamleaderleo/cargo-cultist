# WGPU lock helper-effect replay

Date: 2026-08-19

Status: successful research result for issue #37. This receipt records a narrow interprocedural lock-effect experiment; it does not promote the broad proof carrier as product code.

## Question

Can Cargo Cultist recover a lock-order relation when one acquisition occurs inside a helper and the returned value keeps that guard alive while the caller acquires the next lock?

The first supported pattern is deliberately small:

```text
helper:
  let guard = self.some_lock.lock/read/write();
  ReturnedValue { guard, ... }

caller:
  let token = self.helper(...);
  let next = self.other_lock.lock/read/write();
```

The probe summarizes the helper as returning a value that owns the locally acquired rank, then carries that held-rank effect into the caller until an explicit `drop(token)` or function end.

## Synthetic controls

The research example included two unit cells:

```text
declared A -> B
helper returns A
caller acquires B
  -> zero violations

declared A -> B
helper returns B
caller acquires A
  -> exactly one helper-carried violation
```

Both passed in the dedicated research run.

## WGPU calibration target

Pinned repository:

```text
gfx-rs/wgpu@95c30b29528b23564290b42c197335394f03642d
```

Files:

```text
wgpu-core/src/lock/rank.rs
wgpu-core/src/device/queue.rs
```

Helper / caller:

```text
allocate_submission
flush_pending_writes
```

WGPU's helper acquires `Device::command_indices` and stores the resulting write guard inside `PendingSubmission`. The caller keeps that returned `PendingSubmission` alive and subsequently acquires `Queue::pending_writes`.

The declared rank DAG permits:

```text
DEVICE_COMMAND_INDICES -> QUEUE_PENDING_WRITES
```

## Exact execution receipt

Cargo Cultist research head:

```text
e1cc93343a1508974a98bd78517750cdf833dd26
```

GitHub Actions:

```text
workflow: Lock helper effects research
run:      32219698739
job:      95967747026
result:   success
```

Synthetic tests:

```text
running 2 tests
carries_guard_stored_in_helper_return ... ok
reports_inverse_helper_carried_edge ... ok
```

WGPU output:

```text
HELPER SUMMARY
  `allocate_submission` returns while `command_index_guard` / `command_indices`
  remains owned by the return value (rank `DEVICE_COMMAND_INDICES`).

CALLER EVENTS
  line 1441: direct acquisition
             `snatchable_lock` -> `DEVICE_SNATCHABLE_LOCK`
  line 1443: helper result `submission`
             `command_indices` -> `DEVICE_COMMAND_INDICES`
  line 1446: direct acquisition
             `pending_writes` -> `QUEUE_PENDING_WRITES`

OBSERVATION
  Every supported direct or helper-carried acquisition follows the declared
  successor rule for the most recently acquired rank still held.
```

This recovers a repository-declared relation that a purely lexical two-lock scan of the caller cannot see: the `command_indices` acquisition lives inside `allocate_submission`, while its guard lifetime extends into `flush_pending_writes` through the returned value.

## Relationship to the lexical lock probe

The earlier lock-order experiment recovered direct same-function acquisitions and compared them with WGPU's rank DAG. It also surfaced a real lexical inverse in `compact_blas_inner` on a later WGPU pin and verified a public repair.

This helper-effect experiment extends the fact model in a different direction: **return-value ownership can carry a held-rank effect across a function boundary**.

That suggests a useful incremental progression:

```text
lexical named guard
-> helper-returned named guard effect
-> selected ownership transfers
-> broader summaries only when evidence warrants them
```

No full alias graph is required for the first interprocedural win.

## Evidence boundary

Current supported facts are intentionally narrow:

- top-level named lock/read/write acquisitions;
- helper calls bound to locals;
- locally acquired guards stored into struct expressions in the helper;
- explicit `drop(binding)` release in the caller;
- unique field-name -> rank mapping from the declared DAG.

Outside the current slice:

- guards moved through additional helpers;
- aliases;
- conditional/nested lifetimes;
- guards stored in collections;
- field projections from returned values;
- arbitrary ownership transfers;
- general Rust lifetime or alias analysis.

The proof branch still contains one rustfmt fold and one unused import caught by ordinary quality gates. Those packaging issues are separate from the successful semantic replay and are intentionally not hidden in this receipt.

## Disposition

**Continue.** The helper-returned-guard effect is valuable enough to keep as the next lock-order research primitive. A future implementation should extract the effect-summary logic into the existing lock policy work instead of landing the broad proof carrier unchanged.

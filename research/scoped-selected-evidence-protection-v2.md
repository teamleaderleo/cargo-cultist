# Selected scope evidence protection under byte budget

Tracking: #281. Builds on #191 scoped packet compilation, the #196/#197 pressure test, the #205 exact-ref comparator, and merged #280 final-visible-counterexample preservation.

## Earned policy

Selection and byte budgeting are separate authorities:

```text
upstream selector
  -> exact selected evidence identities

byte-budget compiler
  -> may evict unselected evidence
  -> preserves supplied selected evidence
  -> fails closed when selected evidence + independently protected core cannot fit
```

The compiler does not infer selection from chronology, locality, path ancestry, commit subjects, or a scalar relevance score.

## Promoted scoped surface

The existing command remains the owner:

```text
cargo run --example scoped_agent_context_packet -- FILE --scope DIR
```

Protected mode adds repeated exact refs:

```text
--protect-scope-sha <lowercase-40-hex-sha>
```

When no protection flag is present, the wrapper calls the original `run_scoped()` implementation directly. The default serialized behavior therefore follows the existing #191 path unchanged.

Protected mode:

1. canonicalizes duplicate requested SHAs through a sorted set;
2. requires each selected SHA to exist in the admitted scope-history window after target-history deduplication;
3. receipts the exact selected SHA list under `protected_scope_shas`;
4. evicts unprotected scope rows first;
5. then uses the ordinary nested file-packet semantic eviction ladder;
6. retains every selected scope row while the packet survives;
7. returns `ProtectedCoreTooLarge` when further compliant compression is impossible.

Merged #280 remains independently active inside step 5: a retained historical relation keeps one visible counterexample until that relation itself is evicted.

## Exact external replay

The durable workflow:

```text
.github/workflows/scoped-selected-evidence-replay.yml
```

executes against the pinned pre-guard Stensibly state:

```text
repository  teamleaderleo/stensibly
revision    85cecf2608ad9e734a67518577fa85b9a08a550c
target      convex/schema.ts
scope       convex
```

Upstream-selected exact evidence:

```text
85cecf2608ad9e734a67518577fa85b9a08a550c
  Keep Gmail mailbox-disposition indexes deployable (#1573)

ca5d2c7fdf89666e523972ab6e81610d17b9611b
  Keep Gmail semantic-admission indexes deployable (#1571)
```

Executed receipt:

```text
run       32279758262
job       96155475185
artifact  9375390884
archive   sha256:85eb9cf53137fb09dbda07f3aa9e6f26c9a294137f21b6c58d4f33eee9b320df
result    success
```

## Budget discriminator

### 32,768 bytes

Both ordinary and protected modes retain the full 18-row deduplicated scope history.

```text
ordinary   18,794 bytes
protected  18,894 bytes
```

Protected mode receipts both selected SHAs explicitly.

### 16,000 bytes

The ordinary control still retains the two selected scope rows:

```text
ordinary
  serialized_bytes      15,956
  scope rows             2
  scope evictions        16
  selected lessons       both present
```

Protected mode also keeps both selected rows and spends one nested historical-support example to absorb the explicit protection receipt:

```text
protected
  serialized_bytes      15,938
  scope rows             2
  scope evictions        16
  file evictions         historical_support_example x1
  selected lessons       both present
```

### 15,750 bytes — positive discriminator

The landed unprotected policy removes all scope history and loses both selected lessons:

```text
ordinary
  serialized_bytes      15,575
  scope rows             0
  scope evictions        18
  selected lessons       both absent
```

The promoted protected policy keeps exactly the selected scope rows:

```text
protected
  serialized_bytes      15,721
  scope rows             2
  scope evictions        16
  file evictions         historical_support_example x2
  selected lessons       both present
  protected_scope_shas   exact two requested SHAs
```

This is the executed counterexample to `scope-history age/locality order alone is sufficient for byte eviction`.

### 4,096 bytes — compressed selected floor

The ordinary control fits at 4,044 bytes only by deleting every scope row, including the selected lessons.

Its nested file packet retains two target-history rows after:

```text
historical_support_example      x16
historical_counterexample        x8
historical_companion_relation    x8
recent_history_summary          x15
```

Protected mode fits at **3,911 bytes** while preserving both exact selected scope rows and receipts:

```text
protected
  serialized_bytes               3911
  scope rows                         2
  scope evictions                   16
  selected lessons                both present
  target recent-history rows         0

nested file evictions
  historical_support_example      x16
  historical_counterexample        x8
  historical_companion_relation    x8
  recent_history_summary          x17
  companion_exclusion_detail       x1
```

This is the current measured protected selected-evidence floor for the retained replay.

### 3,900 bytes — fail-closed control

The ordinary policy still emits a 3,856-byte packet after losing both selected lessons.

Protected mode refuses to erase selected evidence:

```text
scoped-agent-context-packet: protected scoped packet evidence requires 3911 bytes, exceeding max_serialized_bytes=3900
```

The new exact protected floor is therefore 3,911 bytes for this retained case. The earlier #205 comparator measured 3,953 bytes; the promoted wrapper is smaller while retaining an explicit SHA receipt.

## Admission controls

Local compiler tests require:

- exact lowercase 40-hex protected SHAs;
- duplicate flags canonicalize deterministically;
- selected SHA must exist after target-history deduplication;
- unprotected sibling scope rows evict before protected rows;
- selected core remains present on fail-closed paths;
- protected output receipts the exact requested SHA set.

The unprotected command path delegates to the original scoped compiler directly, avoiding a parallel default implementation.

## Boundary

- research/compiler promotion only;
- exact Git SHA identifies the selected evidence object for this execution, not durable semantic lineage;
- protected refs grant preservation only, not correctness or authority;
- no automatic scope inference;
- no automatic evidence selection;
- no scalar relevance score;
- no model/provider invocation;
- no claim that all selected evidence should always fit a caller's byte budget.

North star:

> Once upstream research has explicitly selected exact evidence, byte budgeting may compress around that evidence or fail closed; it may not silently delete the selected evidence.

Refs #67 #106 #125 #137 #139 #158 #182 #186 #189 #191 #196 #197 #205 #238 #280 #281 #282.

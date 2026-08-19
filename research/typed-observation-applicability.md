# Typed discriminator applicability repair

Tracking: #195. Stacked on the green #201 negative control over #194/#187.

## Counterexample that earned the repair

#201 composes the real shared applicability evaluator with the v1 discriminator observation and observation frontier types.

It proves both of these states were admitted:

```text
value_state = KNOWN(syntax_changed)
shared applicability = INVALID after exact-head movement
frontier = CURRENT
```

and:

```text
value_state = KNOWN(syntax_changed)
shared applicability = UNKNOWN because current revision is missing
frontier = CURRENT
```

#201 head `1614cc2ae82df50ec3c8b5c4a9e428ad01c1d50f` passed CI run `32250242114` / #1341, so the ambiguity was executable.

The cause is precise: v1 carries only an opaque `applicability_ref`, while generic currentness is decided entirely from `value_state=KNOWN`.

## Disposition: separate axes

The repair chooses #195 Model B.

V2 discriminator observations carry two independent supplied facts:

```text
value_state
  known { value_ref }
  unknown { reason_ref }

applicability
  status = applies | unknown | invalid
  receipt_ref
```

`INVALID` leaves the value-knowledge axis. Stale/coordinate invalidity belongs to applicability.

The generic module still performs no Git/repository/source-domain applicability evaluation. A source adapter supplies the typed applicability state plus the exact receipt that earned it.

## Shared combination rule

One source-owned helper combines the two axes for both generic consumers:

```text
applicability INVALID
  -> INVALID
  preserve known old value when present

applicability UNKNOWN
  -> UNKNOWN
  preserve known old value when present

applicability APPLIES + value UNKNOWN
  -> UNKNOWN

applicability APPLIES + value KNOWN
  -> CURRENT
```

Both discriminator partition enumeration and observation frontier evaluation call this same classifier. The rule therefore cannot drift between #187 and #194.

## Wire/version change

Because the v1 observation meaning allowed KNOWN to outrun applicability, this is an incompatible research wire change:

```text
DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION: 1 -> 2
OBSERVATION_FRONTIER_SCHEMA_VERSION:        1 -> 2
```

The retained three-family observation corpus is rewritten to v2 and gives every current observation an explicit `applicability.status = applies` plus its source receipt.

## Adversarial controls

The repaired standard harness requires:

- retained selected #179 discriminators are current only through `KNOWN + APPLIES`;
- value UNKNOWN with APPLIES remains unknown;
- known value + applicability UNKNOWN remains unknown and preserves the known value;
- known value + applicability INVALID remains invalid and preserves the known value;
- missing applicability receipt rejects;
- duplicate/conflicting observation identity controls remain intact;
- equal values from different source receipts remain distinct observations;
- opaque value spelling still grants zero authority/disposition;
- #194 wrong-subject isolation and mixed-state precedence remain intact;
- the exact #201 moved-head and missing-current-revision cases now produce INVALID/UNKNOWN frontiers instead of CURRENT.

## Executed GitHub receipt

The v2 repair was compacted to one semantic commit directly on #201's exact green counterexample head.

Exact semantic head:

```text
5c84155eb1616b6774f0c5ac764f17ac5d61fa9b
```

GitHub Actions CI run `32251969947` / run number `1417` completed successfully. The job passed:

- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- active-work preflight;
- full `cargo test`, including the v2 discriminator enumeration controls, exact-subject frontier controls, and the two #201 closure tests using the real shared applicability evaluator;
- repository text/JSON dogfood;
- history text/JSON dogfood;
- CI test-filter inventory text/JSON plus positive/control fixtures;
- pull-request diff text/JSON dogfood.

The first repair attempt stopped only at rustfmt before Clippy or tests. The exact formatter deltas touched the three v2 test files only. After those mechanical edits, the intermediate head passed the full gate; the branch was then compacted back to the one semantic commit above and revalidated through CI #1417.

Because #210 is a stacked research PR, the ordinary CI workflow is its authoritative gate; generated-provenance PR dogfood is not part of this stack's trigger set.

The central closure is now executable:

```text
KNOWN old value + applicability INVALID
  -> frontier INVALID
  -> known old value preserved

KNOWN old value + applicability UNKNOWN
  -> frontier UNKNOWN
  -> known old value preserved
```

Stale knowledge can no longer suppress a current observation frontier.

## Phase B consequence

#190 probe planning must consume the repaired frontier semantics.

A historical known value can remain useful as a receipt while current acquisition still proceeds:

```text
KNOWN old value
+ applicability INVALID
-> frontier INVALID
-> source adapter may propose current evidence acquisition
```

The known value is preserved in the noncurrent receipt; it simply cannot suppress current work.

## Boundary

- source adapters supply applicability state; generic modules do not execute the shared evaluator;
- applicability receipt strings remain opaque references after the typed state is supplied;
- no implicit mapping to #145 probe capability;
- no evidence strength, authority, or disposition is inferred from either axis;
- v1 receipts remain retained as the historical counterexample/earlier experiment.

North star:

> Preserve what was learned from stale evidence while making current usability an explicit typed fact that stale knowledge cannot silently override.

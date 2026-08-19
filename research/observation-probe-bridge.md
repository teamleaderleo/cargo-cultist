# Observation frontier to probe bridge

Tracking: #190 Phase B. Composes the green #210 v2 observation/frontier semantics with the green #164 evidence planner.

## Question

When a selected analyzer refinement needs one current observation `D@S`, and the v2 frontier says that observation is missing, unknown, or invalid, how can Cultist ask the existing #145 planner for bounded evidence work without assuming the observation discriminator vocabulary and probe vocabulary are identical?

## Source-owned mapping

V0 adds one explicit record:

```text
ObservationProbeBridge
  bridge_id
  observation_discriminator_id
  observation_subject_ref
  probe_discriminator
    kind
    target
  clearing_requirements
  source_receipt
```

The bridge is supplied evidence that one #145 probe discriminator and one exact clearing coordinate are admitted producers for the required observation.

No mapping is inferred from:

- equal or similar strings;
- the known/old observation value;
- path ancestry;
- probe cost;
- chronology;
- source-receipt prose.

One exact `D@S` may have at most one admitted v0 bridge. Multiple competing mappings remain a future source-selection question instead of becoming hidden generic ranking.

## Composition

For a noncurrent frontier with one exact bridge, the adapter creates one ordinary `DurableObligation`:

```text
missing_discriminator = bridge.probe_discriminator
subject               = bridge.clearing_requirements
clearing condition     = same discriminator + same requirements
```

It then calls the existing #164 planner unchanged.

The planner still owns:

- capability matching;
- exact clearing-condition matching;
- applicability/current-context checks;
- conservative cost ordering;
- read-only / external-read / effectful boundaries;
- selected / blocked / unresolved / stale outcomes.

The bridge grants zero execution authority.

## V2 currentness boundary

#201 proved the old observation model could represent:

```text
KNOWN old value
+ INVALID or UNKNOWN applicability
-> CURRENT frontier
```

#210 repaired that. This bridge consumes the repaired frontier directly:

```text
CURRENT
  -> already_current; no acquisition plan

MISSING / UNKNOWN / INVALID
  -> exact bridge lookup
  -> existing evidence planner
```

A retained old value under an INVALID frontier may justify planning a current-head refresh, but the bridge does not reclassify that old observation. The returned bridge plan preserves the original frontier status until a source actually emits a new current observation.

## Controls

The standard test carrier covers:

1. exact mapped source probe is selected while a cheaper similarly named probe is `incapable`;
2. similarly named probe with no explicit bridge leaves the frontier `no_admitted_mapping`;
3. bridge for another subject cannot map the required frontier;
4. CURRENT frontier produces no acquisition plan;
5. UNKNOWN value at an applicable coordinate can select deeper evidence work;
6. INVALID old value can select a current-head refresh while the frontier remains INVALID;
7. missing current revision stays blocked through the existing planner;
8. effectful mapped probe remains `effect_authority_required` when authority is absent;
9. duplicate exact `D@S` mappings reject;
10. incoherent frontier receipts reject before planning;
11. request JSON round-trip and a 1 MiB input bound are explicit.

## Reader

```text
cargo run --example observation_probe_plan < request.json
```

The reader revalidates the bounded request before planning.

## Boundary

- research only;
- no source analyzer execution;
- no implicit `discriminator_id == probe.kind` convention;
- no conversion from `value_ref` to evidence strength;
- no generic source relevance score;
- no effect authorization;
- no automatic claim that a planned probe cleared the observation;
- source adapters own the mapping receipt;
- #145 planner semantics remain unchanged.

## Execution receipt

The explicit composition base was created by merging green #164 evidence-planner head `3284895e50831811af53d60a9436e0d4ffb3c267` into a branch pinned to #210 v2 frontier head `7b5c4e5add71356e2d58ac234680e26ed0fc8ba9`. The merge commit is `32cef81ac5e57c4d8285cce00fd644265a67e5fd`.

The first Phase B CI attempt stopped only at `cargo fmt --check`; the formatter delta changed line wrapping and module ordering only.

Formatted semantic head:

```text
959d1712235a4f7c7e1f48e524baeecbcf1df0a3
```

GitHub Actions CI run `32256195538` / run number `1453` completed successfully. It passed:

- `cargo fmt --check`;
- strict Clippy;
- active-work preflight;
- full `cargo test`, including mapped/unmapped/wrong-subject/current/UNKNOWN/INVALID/missing-context/effect-authority bridge controls;
- repository text/JSON dogfood;
- history text/JSON dogfood;
- CI test-filter inventory text/JSON and positive/control fixtures;
- pull-request diff text/JSON dogfood.

Main advanced independently while this ran, but the intervening five main commits changed only CI/reference-policy tooling and `AGENTS.md`; they did not touch the observation/frontier/planner/bridge files.

North star:

> Turn a noncurrent observation into bounded investigation only when the source explicitly proves which admitted probe can produce the current observation.

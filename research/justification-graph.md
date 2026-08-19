# Justification graph research receipt

Tracking: #141.

## Question

When one evidence object becomes invalid or unknown at a new repository/work/revision coordinate, can Cultist identify exactly which conclusions need reconsideration while preserving independent support, counterexamples, limits, and explicit clearing conditions?

## v0 model

The first carrier deliberately uses a typed bipartite graph:

```text
EvidenceNode
  -> ClaimNode
  -> relation: support | counterexample | limit | dependency

EvidenceNode
  -> ObligationNode
  -> relation: clearing
```

Evidence nodes carry the existing `EvidenceRequirements` coordinate contract. Evaluation delegates applicability to the shared evaluator from #123/#124/#130.

This means v0 has no claim-to-claim or obligation-to-claim edges. Cycles are excluded by representation instead of being accepted and then interpreted heuristically.

## Semantics under test

### Independent support

```text
E1 --support--> C1
E2 --support--> C1
```

If `E1` becomes INVALID while `E2` still APPLIES, `C1` remains `supported` and both receipts stay visible.

If the sole support becomes INVALID or UNKNOWN, the claim returns to `unknown`. The graph never turns missing applicable justification into a false claim.

### Dependencies

A dependency is a hard prerequisite in this v0 experiment. Any INVALID or UNKNOWN dependency keeps the claim justification `unknown` even when an ordinary support receipt still applies.

This is intentionally narrower than a general logical justification language. Alternative dependency sets / AND-OR derivations remain future research.

### Counterexamples and limits

Applicable `counterexample` and `limit` edges remain explicit receipts. They do not automatically negate a supported claim or grant a stronger disposition. A downstream JEI/review/projection consumer can decide how those roles affect the current action while preserving the evidence that changes what may be concluded.

### Clearing obligations

An obligation is:

```text
open
  no clearing evidence currently applies

unknown
  candidate clearing evidence has UNKNOWN applicability

cleared
  at least one clearing evidence object applies
```

An open obligation is valid with **zero clearing edges**. The immediately stacked #144 experiment exposed this as a necessary case: a durable missing discriminator can exist before any clearing evidence object has arrived. Requiring a clearing edge would manufacture prospective evidence as if it had already been observed.

When exact-head clearing evidence later becomes INVALID after revision movement, the obligation reopens.

## Reevaluation receipt

`reevaluate_graph(before_context, after_context)` evaluates evidence applicability at both coordinates and emits only:

```text
changed evidence applicability
-> affected claim/obligation targets
```

An unrelated claim whose evidence coordinate did not change is absent from the affected-target receipt.

This is the first discriminator for a future truth-maintenance engine: invalidate/reconsider downstream conclusions instead of treating the entire packet as one stale blob.

## Adversarial controls in the standard test suite

The carrier includes controls for:

- one invalidated support + one independent support -> claim remains supported;
- sole invalidated support -> claim becomes unknown;
- counterexample and limit receipts remain visible;
- an obligation can remain open before any clearing evidence exists;
- exact-head clearing evidence clears an obligation and head movement reopens it;
- reevaluation names only targets downstream of changed evidence;
- UNKNOWN dependency prevents a supported projection;
- relation/target mismatch rejects rather than inventing cyclic/general graph semantics.

## Executed GitHub receipt

Draft PR #156 ran the ordinary repository gates after the open-obligation correction and fixture-only Clippy containment.

Exact head under test:

```text
9399f29ebcb96e5b306e88b6ad12c0189f0e6f62
```

GitHub Actions CI run `32243151354` / run number `1018` completed successfully on the PR merge ref. The job passed:

- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- active-work heads-up;
- full `cargo test` including the justification integration harness;
- repository text/JSON dogfood;
- history text/JSON dogfood;
- CI test-filter inventory text/JSON plus positive/control fixtures;
- pull-request diff text/JSON dogfood.

The PR-only push-diff step remained skipped by workflow context.

Generated provenance review dogfood run `32243151304` / run number `160` also completed successfully for the same head.

Two earlier CI attempts were useful mechanical controls rather than semantic failures: the first exposed rustfmt differences; the second reached Clippy and exposed one fixture-only unused applicability constant through the standalone example. Both were repaired before this executed receipt.

## Boundary

- claim provenance remains separate from justification relation;
- applicability remains separate from epistemic strength and authority;
- v0 graph identity is local to the exact research object;
- no stable semantic lineage is inferred from IDs;
- no mutation, merge, deployment, or review authority is granted;
- no model is required;
- no `AnalysisReport` schema change is proposed by this carrier.

## Next discriminator after this carrier

The first follow-up is already concrete through #144: preserve open obligations and their future clearing conditions without pretending expected evidence is observed evidence.

Beyond that, the next useful truth-maintenance question is whether real Cultist evidence needs **alternative derivation groups** such as:

```text
(E1 AND E2) OR E3 -> C1
```

before justification edges deserve canonical IR representation. Keep the v0 bipartite model until a real replay demonstrates that richer derivation semantics change the justified next action.

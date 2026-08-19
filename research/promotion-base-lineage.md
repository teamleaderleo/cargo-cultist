# Promotion base-lineage receipts

Tracking: #278. Child of merged #284 promotion receipt reuse.

## Trigger

The first complete historical reconstruction of #194 exposed a case the initial forward-range model cannot honestly represent.

Early successful merge-view receipt:

```text
run          32249440070
branch head  b54ed3213994c96ec818ef36bb9728b0dc1f7eb6
base         a6026e084ae6f1d88e8d7f2322473872f82c3278
```

Final exact-main run:

```text
run          32269578803
branch head  6706755e434a9bb533d77655587b01cbdd3fa1e8
base         0c7da4f3ad148c4d14bce127c75cfe4b03b658c8
```

The two bases diverge at:

```text
c2133131038f33a98f7bc7d206ca6e4284be420a
```

with:

```text
tested-base-only commits   4
current-base-only commits 34
```

A single forward `intervening_commits[]` list would erase which side owned a changed object.

## Two kinds of divergent evidence

This child keeps merged #284's `PromotionReceiptRequest` as the branch/check identity object and adds two independent base-compatibility receipts.

### 1. Complete base-range path receipts

```text
merge_base_sha
base_path_receipts_complete = true

tested_base_only?
  base_sha
  head_sha
  commit_count
  changed_paths[]

current_base_only?
  base_sha
  head_sha
  commit_count
  changed_paths[]
```

The derived relation is:

```text
same_tree
forward
rewind
diverged
```

Range endpoints bind the declared common ancestor to the exact tested/current base commits. When base trees differ, `base_path_receipts_complete=true` is mandatory caller evidence that the supplied compare-range path inventories are complete. A changed-base request without that attestation rejects before promotion interpretation.

Range paths answer one bounded question in v0:

> Did either base lineage side touch a path owned by the branch change set itself?

Every branch-path overlap preserves its side:

```text
tested_base_only | current_base_only
range_head_sha
changed_path
declared_path
kind = branch_path
```

### 2. Exact endpoint compatibility-object receipts

Consumed contracts and applicable policy gates use exact tested/current Git object identity instead of merge-base changed-path union:

```text
compatibility_objects[]
  path
  kind = consumed_contract | applicable_policy
  tested_object_sha?
  current_object_sha?
```

The object list must exactly cover every declared:

```text
promotion.consumed_contract_paths[]
promotion.applicable_policy_paths[]
```

A state may have one missing endpoint to represent an added or removed object. At least one endpoint must exist. Equal base trees cannot claim different compatibility-object identities.

A compatibility object changes only when:

```text
tested_object_sha != current_object_sha
```

This distinction is necessary under divergent ancestry. The same file may appear in both merge-base delta path sets because each side independently carried it, while the final tested/current objects are still byte-identical.

## Decision priority

The disposition remains compatible with merged #284:

```text
conflict / nonmergeable
change-set fingerprint changed
exact effective merge-tree identity
same base tree
branch-owned path collision OR endpoint compatibility-object change
complete compatibility scope with no relevant change
semantic independence UNKNOWN
```

Exact effective merge-tree identity remains the strongest receipt and can transfer across divergent ancestry. Otherwise:

```text
branch path collision
  -> rerun_required / branch_path_overlap

changed consumed endpoint object
  -> rerun_required / consumed_contract_overlap

changed policy endpoint object
  -> rerun_required / applicable_policy_overlap
```

A divergent changed-path occurrence alone cannot claim a contract/policy change; endpoint object identity owns that judgment.

## First real #194 finding

The #194 branch compatibility payload remained byte-equivalent across the reanchor.

Its direct consumed discriminator contract remained exact across both base endpoints:

```text
src/discriminator_observation.rs

tested base a6026e08...
  6f370c3ba20ebb7220c8c075f2c2df56f650fd83

current base 0c7da4f3...
  6f370c3ba20ebb7220c8c075f2c2df56f650fd83
```

That path also appears in both divergent compare ranges. Endpoint identity proves the contract itself stayed unchanged.

The applicable CI policy changed:

```text
.github/workflows/ci.yml

tested base
  26c4f3e3b4dc68bc5e166c72eab394a6548c9ceb

current base
  7e1f1b966fcd1e45930d0ff12a68532cba802502
```

So the final #194 rerun was justified by an actual compatibility input change:

```text
same branch compatibility payload
+ same consumed discriminator contract
+ changed applicable CI policy
-> rerun_required
  reason = applicable_policy_overlap
```

The evidence does not reduce to `main moved`.

## Controls

`tests/promotion_base_lineage.rs` covers:

- divergent disjoint base delta + unchanged endpoint compatibility objects -> `inspect_semantic_overlap`;
- the same contract path touched on both divergent sides with equal endpoint object IDs -> no false contract-change reason;
- changed policy endpoint object -> rerun for policy only;
- removed tested-side consumed contract -> rerun for consumed contract;
- actual branch-owned path collision -> rerun and preserve divergent side;
- explicitly complete compatibility scope + no relevant change -> reusable;
- exact effective merge-tree identity -> reusable even when endpoint object changes remain visible;
- explicit forward and rewind relations;
- equal base trees require equal compatibility-object receipts;
- compatibility-object list must exactly cover declared contracts/policies;
- incomplete changed-base path receipt attestation -> reject;
- range endpoint mismatch -> reject.

## Reader

```text
cargo run --example promotion_base_lineage < request.json
```

The reader is pure evaluation. A later GitHub adapter can collect:

```text
Actions merge-view tested/current base commits
GitHub compare merge-base + side ranges
complete changed-path inventories
exact contract/policy object IDs at both endpoints
canonical branch change-set fingerprints
```

## Next dogfood

#201 remains the next retained target. Its one-file compatibility payload is already proven byte-identical across retained early/final heads. A complete historical classification requires exact later one-file merge-view receipts rather than treating the earlier stacked #194 child as a current-main rerun.

After enough retained cases, #278 can report:

```text
receipt_reusable count
rerun_required count + exact reasons
inspect_semantic_overlap count
```

without crediting path disjointness or ancestry churn as proof.

## Boundary

- research only;
- exact compare-range receipts, no invented commit story;
- complete range-path attestation is explicit caller evidence;
- compatibility object identity is exact Git object identity, not semantic equivalence;
- complete compatibility scope remains explicit caller evidence;
- no merge/rebase/rerun side effect;
- no automatic use of commit ancestry as semantic authority.

North star:

> Preserve both sides of a rewritten base, compare the exact compatibility objects that reached each endpoint, and rerun only when a compatibility input actually changed or remains unresolved.

Refs #96 #137 #194 #201 #278 #279 #284.

# Promotion receipt reuse

Tracking: #278. This is the executable research counterpart to `docs/concurrent-work-promotion.md`.

## Question

When `main` moves after a successful PR merge-view run, can Cultist distinguish a changed repository coordinate from evidence that actually invalidates the prior compatibility receipt?

The evaluator emits one of:

```text
receipt_reusable
rerun_required
inspect_semantic_overlap
```

Path disjointness alone never proves semantic independence.

## Two different identities

A force-reanchor onto a new base changes the full commit tree even when every branch-owned executable/test blob is unchanged. #194/#201 reproduced this repeatedly.

V0 therefore preserves two different receipts:

```text
full Git identity
  head_sha
  tree_sha
  base_sha
  base_tree_sha
  effective_merge_tree_sha

branch compatibility payload
  change_set_sha256
```

Full Git trees remain audit coordinates. They answer which repository state was tested or is currently proposed.

`change_set_sha256` answers a narrower question: are the branch-owned compatibility bytes being transferred across the reanchor the same?

`src/promotion_change_set.rs` defines the canonical collector for this receipt. It accepts a bounded unique set of exact:

```text
(repository_relative_path, git_blob_sha)
```

sorts by path/blob identity, and hashes domain-separated bytes:

```text
cultist-promotion-change-set-v1\0
path\0blob_sha\n
...
```

The collector rejects duplicate paths, noncanonical repository-relative paths, and malformed blob SHAs.

A promotion adapter should derive the compatibility payload deliberately. Receipt/provenance output written *after* successful validation may evolve separately when it records the new run. That distinction is executable in the retained #194 fixture: the source/test/example blobs stayed identical across the reanchor while `research/observation-frontiers.md` changed to append newer execution evidence.

## Evaluation input

A bounded request preserves tested/current coordinates:

```text
tested
  head_sha
  tree_sha
  change_set_sha256
  base_sha
  base_tree_sha
  effective_merge_tree_sha
  successful_check_refs[]

current
  head_sha
  tree_sha
  change_set_sha256
  base_sha
  base_tree_sha
  effective_merge_tree_sha
  mergeable
  conflict
```

and compatibility evidence available to the caller:

```text
branch_changed_paths[]
consumed_contract_paths[]
applicable_policy_paths[]
intervening_commits[] { sha, changed_paths[] }
compatibility_scope_complete
```

Declared paths are canonical repository-relative path scopes. Exact files and directory prefixes use the same overlap relation.

`compatibility_scope_complete=true` is a strong caller assertion: supplied branch/contract/policy scopes completely cover compatibility for this promotion decision. The evaluator preserves the assertion; it never infers completeness.

## Decision priority

### Rerun / reconcile first

```text
current conflict
current not mergeable
change_set_sha256 changed
```

These beat overlap inference. A different full `tree_sha` alone does not imply branch semantic change after a reanchor.

### Exact receipt transfer

Strongest v0 identity:

```text
tested effective merge tree == current effective merge tree
  + same change-set fingerprint
  -> receipt_reusable
```

A head/base metadata rewrite also reuses the receipt when:

```text
same change-set fingerprint
same base tree
```

The differing full head/tree/base commit identities remain visible in the output.

### Relevant intervening change

When base tree changed, the caller supplies intervening commit receipts. Any changed path overlapping:

```text
branch_changed_paths
consumed_contract_paths
applicable_policy_paths
```

produces `rerun_required` and retains exact overlap rows:

```text
commit_sha
changed_path
declared_path
kind
```

Multiple overlap kinds remain visible together.

### Disjoint movement

With unchanged change-set fingerprint and zero declared overlap:

```text
compatibility_scope_complete = true
  -> receipt_reusable

compatibility_scope_complete = false
  -> inspect_semantic_overlap
  -> semantic_independence_unknown
```

This is the deliberate negative control from #279: disjoint paths justify inspection. Reuse requires stronger evidence; automatic rerun also requires stronger evidence.

## Synthetic controls

`tests/promotion_receipt.rs` covers:

- same effective merge-tree identity + same change set -> reusable;
- reanchor whose full head tree changes while change set and base tree stay identical -> reusable;
- same change set + changed base + disjoint paths -> semantic-overlap inspection rather than tree-based rerun;
- branch path overlap -> rerun;
- consumed-contract prefix overlap -> rerun;
- applicable policy/CI path overlap -> rerun;
- explicitly complete compatibility scope + no overlap -> reusable;
- conflict/nonmergeable -> rerun;
- changed change-set fingerprint -> rerun;
- malformed change-set digest -> reject;
- changed base tree without intervening commit receipts -> malformed request;
- output preserves exact tree/head/base/change-set/check/intervening coordinates and overlap reasons.

## Retained real identity controls

`tests/promotion_change_set_real.rs` records exact historical objects from two earlier promotion-churn episodes.

### #201 known/stale applicability negative control

Earlier successful head:

```text
1614cc2ae82df50ec3c8b5c4a9e428ad01c1d50f
tree a90a3317c50d5d7d693b948cc9414315056c628f
CI   32250242114
```

Final promoted head:

```text
3cf9090dfb474adaac6ab773c357627c37c3f9e6
tree 889e34a998fe268986718bf21e72263503a1a05b
```

The exact one-file compatibility payload is byte-identical across both heads:

```text
tests/known_stale_observation_frontier.rs
blob c5945b4cfee5f6ea43f782d0c5b68fa8a9125ef4
```

The full trees differ; the canonical change-set fingerprint is equal.

### #194 observation frontier

Earlier successful semantic head:

```text
b54ed3213994c96ec818ef36bb9728b0dc1f7eb6
tree d4a62bdd14700de18abadf1b593309bb2683107c
CI   32249440070
```

Final exact-main validation head:

```text
6706755e434a9bb533d77655587b01cbdd3fa1e8
tree d7607be315f0920b8437ed35e458aa9a8289109e
CI   32269578803
```

Executable/test payload blobs remained exact:

```text
src/observation_frontier.rs
  23b142040cb244cb09e0428d162b6fcfaf787e67

tests/observation_frontier.rs
  d809ae08481e642e50818adc1395e9c4b4827563

examples/observation_frontiers.rs
  41d63d6b92f66d9faaa2c97af1bc6f06505c501a
```

The provenance note changed as newer execution evidence was appended:

```text
early cddd9e76252c19cd82c781b472f2f06a8e84b45d
final 21b89394a82ea07d31441afa7aa77917b3d4b1a2
```

This is a positive reason to model compatibility payload identity separately from receipt output identity.

These retained cases prove byte-transfer identity only. They do not invent a complete historical intervening-commit sequence, so V0 does not label those full historical reanchors `receipt_reusable` yet. The remaining #278 dogfood is to collect complete intervening commit/contract/policy receipts and count avoidable reruns.

## Reader

```text
cargo run --example promotion_receipt < request.json
```

The reader performs no GitHub call. Collection of merge-tree/path/contract facts and canonical change-set entries remains an adapter concern; this lane evaluates supplied evidence.

## Next dogfood

Continue the real #194/#201/#156 sequence with complete promotion receipts:

- reconstruct discarded current-main tested heads where retained;
- collect the complete intervening `main` commit path range;
- mark direct consumed contracts and applicable policy gates;
- compute canonical compatibility payload fingerprints;
- count which historical reruns become reusable;
- retain the known path-disjoint project-memory contract family as `inspect_semantic_overlap` until compatibility-scope completeness is actually earned.

## Boundary

- research only;
- no automatic semantic-independence inference;
- no GitHub mutation or merge authority;
- no branch promotion side effect;
- caller-declared compatibility payload and complete compatibility scope are evidence, not model guesses;
- provenance-only outputs may be excluded from the compatibility payload only when their role is explicit;
- policy/contract dependencies may extend beyond changed files and must be declared before changed-base reuse.

North star:

> Preserve successful compatibility evidence when the branch-owned compatibility payload is proven equivalent, rerun when a compatibility input changed, and keep semantic independence UNKNOWN when the evidence stops at path disjointness.

Refs #96 #137 #156 #194 #201 #278 #279.

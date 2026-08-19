# Concurrent work and promotion discipline

Cultist is frequently developed by several workers at once. A branch can remain semantically sound while `main` advances repeatedly underneath it, so branch promotion needs to distinguish a changed repository coordinate from a changed compatibility claim.

This policy exists to prevent two opposite failures:

- merging stale work after a relevant concurrent change;
- repeatedly rebuilding and rerunning byte-identical work merely because an unrelated `main` commit landed.

## Default promotion authority

For an open pull request, the normal compatibility authority is the **current GitHub merge view plus its successful required checks**.

A successful merge-view run proves the branch was tested with the `main` state represented by that merge view. A later `main` commit does not automatically erase that receipt.

Before rerunning because `main` moved, inspect the intervening change and classify it.

### Rerun required

Rerun or rebuild when the intervening change affects a compatibility input for the branch, including:

- a changed path also changed by the branch;
- a directly consumed module, schema, fixture, generated owner/input, CLI/report contract, or test helper;
- a repository policy or CI gate that applies to the branch;
- branch mergeability changed or GitHub reports a conflict;
- the branch was rebased/rebuilt and its effective tree changed;
- evidence needed to establish independence is missing.

### Receipt may remain usable

A prior successful merge-view receipt may remain the promotion authority when all of the following are established:

- branch semantic tree is unchanged;
- the successful run covered the current branch tree;
- the intervening `main` change is inspected;
- no relevant path, consumed contract, policy, generated relation, or test dependency changed;
- the pull request remains mergeable.

Path disjointness is useful evidence, but it is never sufficient by itself. Producer/consumer contracts can collide across different files.

### Exact tree identity

When a branch head SHA changes only because commit metadata or ancestry was rewritten, compare repository tree identity before starting another expensive run.

If the gated commit and current head have the same tree SHA and the same relevant base semantics, the executable repository state is identical. Preserve that fact in the promotion receipt instead of treating the new commit SHA as a semantic change.

## Reanchoring

Reanchoring a branch onto current `main` is appropriate when it simplifies stale ancestry, removes already-landed parent commits, resolves a real conflict, or is required by a relevant concurrent change.

Do not reanchor byte-identical work solely to chase the newest `main` SHA.

When flattening an old research stack:

1. identify the exact files/blobs owned by the child;
2. verify replacements against current-main preconditions before overwriting existing files;
3. rebuild only that semantic delta on the landed parent;
4. run the normal merge-view gate;
5. preserve prior executed research receipts as historical evidence, while naming the current merge-view run as promotion authority.

## Parallel ownership

Before starting a new branch for an active research or implementation lane, check current branches and pull requests.

If another worker already owns the same lane:

- review or repair that carrier;
- preserve unique useful evidence from competing work;
- close or abandon duplicate scaffolding;
- avoid parallel implementations that will later require semantic reconciliation without adding an independent control.

## Long autonomous runs

For broad instructions such as “keep working,” work in coherent milestones.

A milestone is complete when one useful unit has reached a durable boundary such as:

- a bug is reproduced and repaired with a regression;
- one research discriminator has a positive and negative control;
- one branch is cleanly promoted;
- one stacked child is reduced to its true semantic delta and left green;
- one external replay produces a durable receipt;
- one dead/duplicate carrier is retired safely.

After a milestone:

1. record the durable result;
2. leave the next lane with an exact handoff;
3. avoid recursively promoting every available descendant merely because it is adjacent;
4. continue immediately only when the requested task still requires that next milestone or the user explicitly wants a sustained sweep.

This is a scope-control rule, not a wall-clock rule. The objective is to prevent a useful task from silently turning into an unbounded repository-wide promotion campaign.

## Fail-safe principle

Concurrency classification is itself evidence-bearing:

- `PROVEN`: exact path/tree/merge/conflict facts;
- `OBSERVED`: a concurrent change touched a known consumed contract;
- `UNKNOWN`: semantic independence when direct evidence is absent.

Use `UNKNOWN` to trigger inspection. Do not convert it automatically into either “safe to merge” or “rerun everything.”

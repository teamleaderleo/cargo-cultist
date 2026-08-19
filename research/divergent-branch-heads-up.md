# Divergent branch heads-up research

Date: 2026-08-19

## Question

Can Cultist treat **unpublished divergent remote branches** as a second live-work source without confusing old/squash-merged branches with active work or increasing the normal interruption rate?

This extends the PR-only active-work experiment from #98/#100 under the broader concurrent-change proposal in #96.

The branch layer stays advisory and reuses the same `WorkItem` inventory vocabulary:

```text
open PR metadata
+ bounded divergent remote branches
-> active-work inventory
-> exact path overlap / pre-edit focus
```

Branch-name similarity is never used as evidence.

## Adapter candidate

The research adapter lives in `scripts/divergent_branch_inventory.py`.

It starts from the landed GitHub PR inventory and adds remote branches only when they satisfy explicit lifecycle/evidence gates.

### PR heads win over duplicate branch identities

If an open PR already represents a branch head, the branch is excluded. The richer PR item carries provider URL, draft state, and PR freshness, so a second `branch:*` work item would be duplicate evidence.

### Git divergence is necessary but not sufficient

Candidate refs come from:

```text
git branch -r --no-merged origin/main
```

and their changed paths are measured from the exact merge base with `origin/main`.

This catches real unpublished branch work, but the first dogfood exposed a major Git-only lifecycle trap: **squash-merged PR heads remain topologically unmerged**.

PR #103 was squash-merged to `main`; its source branch `feature/preflight-active-inventory` therefore still appeared under `git branch --no-merged` even though the work had already been integrated.

The first research run failed precisely because the original negative control assumed Git ancestry would retire that branch.

### Provider lifecycle repairs squash-merge ambiguity

The adapter now also retrieves the most recently updated 100 closed/merged PR heads.

A branch candidate is retired only when its **current branch name + exact current head SHA** matches a closed/merged PR head from that observed window.

```text
same branch name + same head SHA as closed/merged PR
  -> retire exact head

same branch later advances to a new SHA
  -> eligible again
```

This keeps provider lifecycle evidence separate from Git ancestry.

The closed-PR head window is explicit and bounded. If GitHub reports more than 100 closed/merged PRs in the observed window, the adapter records truncation rather than pretending old heads were exhaustively classified.

### Branch candidate budget

After open-PR and exact closed-PR-head exclusions, candidates are ordered by exact commit timestamp and capped at 20.

The inventory records:

- base ref;
- number of open PR heads excluded;
- recent closed PR heads seen;
- whether the closed-PR window truncated;
- exact closed-PR branch heads retired;
- candidate count before branch cap;
- returned branch count;
- omitted branch count;
- branch cap;
- selection order;
- explicit `branch_name_similarity_used = false`.

## Executed dogfood

### First run: useful failure

Run:

```text
run: 32232628351
job: 96005566152
```

The adapter correctly discovered unpublished `experiment/divergent-branch-heads-up`, but the control expecting `feature/preflight-active-inventory` to be absent failed.

That was not a false implementation failure; the **control premise had gone stale**. PR #103 had since merged via squash, so the feature branch was no longer identical to current `main` by commit ancestry even though its exact work had already been accepted.

This produced the stronger lifecycle rule above.

### Final branch inventory receipt

Final research run:

```text
run:      32233054642
job:      96006879211
artifact: 9357939745
sha256:   45860982f7ed855b5f992b894a9360294cf16110ce7283f603ebb034e67e16a9
```

Observed branch receipt:

```text
open PR heads excluded:              3
recent closed/merged PR heads seen: 48
closed-PR head window truncated:    false
exact closed-PR branch heads retired: 23
unmerged non-PR branch candidates:   7
branch candidates returned:          7
branch candidates omitted:           0
max branch candidates:              20
branch-name similarity used:        false
```

So in this repository the retirement window was complete for the observed closed/merged PR set, and no candidate was lost to the branch cap.

### Natural carrier stayed quiet

The #107 carrier changed only its research workflow. With the combined open-PR + branch inventory, the normal research advisory emitted no heads-up.

The same combined inventory, after removing the research-only `adapter_receipts` field, was accepted by the **product** command:

```text
cargo cultist preflight --inventory /tmp/product-active-work.json --format json .
```

Product result:

```text
analysis: preflight-active-inventory
findings: []
```

This proves branch work items can reuse the landed strict product inventory schema without inventing a second product format.

### Pre-edit focus found real unpublished work

At observation time, `experiment/divergent-branch-heads-up` had **no PR** and changed:

```text
scripts/divergent_branch_inventory.py
```

A focus query before the carrier touched that path emitted:

```text
branch:experiment/divergent-branch-heads-up
d99c27e5
2026-08-19T16:28:01+08:00
focus overlap: scripts/divergent_branch_inventory.py
```

This is the intended pre-PR heads-up: exact branch identity, exact head, exact freshness, exact path, no ownership/duplication conclusion.

### Cross-PR noise matrix

The same inventory snapshot was replayed with each open PR substituted as `current`:

```text
#107 heads-up count: 0
#102 heads-up count: 0
#95  heads-up count: 0
```

Seven branch candidates were present, yet none produced a natural direct-path interruption for the three open PRs in the snapshot.

That is encouraging quietness evidence, not proof that branch awareness is universally low-noise.

## Product boundary discovered: scope is not intent

The merged product interface:

```text
cargo cultist preflight --inventory FILE [PATH]
```

uses `[PATH]` as an **analysis scope over observed `current.changed_paths`**.

The pre-edit research query instead accepts caller-supplied focus paths that may not have been changed yet.

These are different claims:

```text
current.changed_paths
  -> provider/work-item observation

focus path
  -> caller-supplied intended inspection/edit target
```

Do not encode pre-edit intent by lying in `current.changed_paths`.

A future product-facing pre-edit interface needs a separate intent/focus input rather than overloading observed work metadata.

## Why branch awareness is not auto-enabled yet

The current sample is good:

- exact unpublished branch positive;
- quiet natural carrier;
- quiet matrix across all open PRs;
- product inventory compatibility;
- squash-merge lifecycle false positive fixed;
- no branch-budget truncation in the observed repo.

But one important bound still needs repeated dogfood: exact closed-PR-head retirement currently uses a bounded recent-100 provider window. The current repository had only 48 observed closed/merged PR heads, so the run was complete; larger repositories may need a more targeted branch-to-PR lifecycle query before this becomes a default always-on source.

For now this adapter is research instrumentation that can feed the landed product inventory format.

## Next discriminators

1. Run the adapter on repositories with more than 100 closed PRs and prove truncation remains visible rather than turning old squash-merged branches into silent truth.
2. Evaluate a targeted provider query for each selected branch head so lifecycle lookup scales with the bounded branch candidate set rather than repository-wide closed-PR history.
3. Keep pre-edit focus separate from observed changed paths; decide whether it belongs in `brief`, a `preflight --focus`, or a future intent-aware inventory version.
4. Continue classifying naturally surfaced branch heads-ups as:

```text
useful
irrelevant
missing stronger evidence
misleading
```

## Product principle

> Git says whether histories merged. Provider metadata says whether a work item was accepted, closed, or superseded. Live coordination needs both.

Refs #96, #98, #100, #101, #103, #62, #74.

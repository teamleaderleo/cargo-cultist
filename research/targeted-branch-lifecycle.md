# Targeted branch lifecycle research

Date: 2026-08-19

## Question

Can unpublished-branch lifecycle classification scale with the **bounded branch candidates Cultist might actually surface**, instead of querying a global recent-closed-PR window?

This follows `research/divergent-branch-heads-up.md`, which proved useful pre-PR branch awareness but retained one awkward bound: exact squash-merged branch heads were retired by scanning the most recent 100 closed/merged PR heads.

The targeted experiment removes that repository-global history window.

## Candidate evidence flow

```text
open PR inventory
-> exclude branch names already represented by open PRs
-> Git remote refs not merged into origin/main
-> order by exact commit timestamp
-> select newest 20 lifecycle candidates
-> one batched provider-current branch-ref query
-> require provider head SHA == local fetched head SHA
-> inspect PRs associated with each exact head commit
-> exact branch name + exact head SHA + CLOSED/MERGED => retire
-> exact OPEN PR discovered in the lifecycle query => suppress duplicate branch item
-> truncated associated-PR evidence without a conclusion => UNKNOWN / stay quiet
-> surviving candidates become branch WorkItems
```

The adapter never uses branch-name similarity as intent evidence.

## Why provider-current ref verification belongs here

The local checkout is a snapshot. A branch may advance or disappear between fetch and lifecycle analysis.

The targeted adapter therefore refuses to use local changed-path evidence unless GitHub's provider-current branch ref still points at the same SHA.

```text
local remote SHA == provider-current SHA
  -> local merge-base/path evidence remains applicable

local remote SHA != provider-current SHA
  -> stale snapshot; omit branch and record mismatch
```

This is a stronger freshness contract than the previous global closed-PR lookup.

## Associated-PR bound

For each selected branch head, the query asks for at most 20 PRs associated with that exact commit.

Retirement/open-PR conclusions require exact:

```text
headRefName == branch
headRefOid  == provider-current branch SHA
```

If the associated-PR connection reports another page and no exact lifecycle conclusion is already present, the branch remains unknown and is omitted rather than guessed active.

The bound is therefore visible in the adapter receipt and fail-closed for heads-ups.

## Executed replay

Research carrier: PR #112.

Workflow:

```text
run:      32233938112
job:      96009623433
artifact: 9358251377
sha256:   a6f2107f8f2b9374ff65eafac8f4a51b6f3fd616d5b25b359c8ae9043c15dadc
```

The inventory build completed in roughly 1.87 seconds in the replay job.

Observed receipt:

```text
open PR heads excluded:                 5
unmerged non-PR branch candidates:     32
lifecycle candidates selected:          20
lifecycle candidates omitted:           12
max branch candidates:                  20
max associated PRs per candidate:       20
provider refs missing:                   0
provider head mismatches:                0
provider open-PR races:                  0
exact retired branch heads excluded:     9
lifecycle association truncated unknown: 0
branch candidates returned:             11
branch-name similarity used:            false
```

The important change from the previous adapter is what is **absent**:

```text
no recent_closed_pr_heads_seen
no recent_closed_pr_head_limit
no global recent-closed-PR history window
```

Lifecycle evidence is now proportional to the bounded candidate set.

## Squash-merge control

`feature/preflight-active-inventory` still appears topologically unmerged after squash-merged PR #103.

The targeted query nevertheless retired it using the exact associated PR evidence for that current branch head.

So the stronger rule survives without scanning unrelated closed PRs:

```text
Git: this history is not merged
provider-current ref: exact SHA S
associated PR: same branch + head S is closed/merged
=> retire S
```

## Quiet natural carrier

The #112 carrier remained quiet with the targeted combined PR + branch inventory.

This shows that the targeted lifecycle change did not force a heads-up merely by adding more branch evidence.

## Product compatibility

After removing the research-only `adapter_receipts` field, the exact same combined inventory was accepted by the landed product command:

```text
cargo cultist preflight --inventory FILE --format json .
```

Result:

```text
analysis: preflight-active-inventory
findings: []
```

No second product inventory format is required for branch work items.

## Pre-edit focus positive

The implementation branch `experiment/targeted-branch-lifecycle` remained unpublished while #112 ran.

Before the carrier touched the adapter path, focus mode surfaced:

```text
branch:experiment/targeted-branch-lifecycle
9bc78f73
2026-08-19T16:41:52+08:00
focus overlap: scripts/divergent_branch_inventory.py
```

This preserves the intended pre-PR use case after replacing the lifecycle mechanism.

## Remaining default-activation question

The global lifecycle-history blocker is gone. One different question remains before bare branches deserve the same default treatment as open PRs:

> Does "divergent, unpublished, provider-current" mean sufficiently *active* to interrupt by default?

An open PR has an explicit OPEN lifecycle. A bare branch may simply be abandoned.

The adapter already reports exact commit freshness and selects only the newest 20 candidates, but that is a bounded relevance sample rather than proof of active execution.

For now, branch work remains a research/optional evidence source. The next useful discriminator should test branch freshness/intent against naturally abandoned and genuinely live unpublished branches instead of inventing a hidden age score.

## Product boundary: focus remains intent

Product `preflight --inventory FILE [PATH]` scopes **observed current changed paths**.

Research focus paths are caller-supplied pre-edit intent.

Do not encode intent as fake observed changes. A product-facing `--focus`/brief input should keep those claims distinct.

## Product principle

> Query lifecycle where the possible interruption lives. Keep provider-current identity, Git divergence, observed work, and caller intent as separate evidence.

Refs #96, #107, #103, #112, #62, #74.

# Active-work heads-up research

Date: 2026-08-19

## Question

Can Cultist cheaply surface live concurrent work that a coding agent would reasonably want to know about, without becoming a scheduler, lock manager, or GitHub-dependent core?

The first experiment asks only:

> Does another currently open work item modify an exact repository path that this work item already modifies or explicitly intends to inspect/change?

That is intentionally smaller than semantic conflict detection. The goal is to dogfood the interruption policy before broadening the evidence model.

## Adapter boundary

Remote project state and local evidence analysis stay separate:

```text
GitHub / orchestrator / other provider
  -> active-work inventory JSON

Cultist research analyzer
  -> validate + normalize inventory
  -> exact path intersection
  -> advisory heads-ups
```

The Rust analyzer does not call GitHub and does not need credentials or network access.

The GitHub adapter records, for each open PR:

- PR identity, title, and URL;
- exact head ref and head SHA;
- GitHub `updated_at` freshness receipt;
- draft state;
- complete changed-path inventory.

The common case is retrieved in one GraphQL request: up to 100 open PRs and the first 100 files per PR. Additional requests happen only when those explicit pagination bounds are exceeded. The supplying adapter's completeness and freshness remain separate evidence from the overlap result.

## Current and intended paths remain distinct

The analyzer accepts two path sources:

```text
current.changed_paths
  -> provider-observed paths already changed by this work item

FOCUS_PATH arguments
  -> caller-supplied intended targets before the first edit
```

The report retains `changed_overlap_paths` and `focus_overlap_paths` separately. A focus path is intent evidence supplied by the caller; it does not prove that the current work will eventually modify that path.

This distinction lets the same analyzer serve both:

```text
BEFORE
  I am about to touch src/foo.rs. Is active work already there?

DURING
  My diff now touches src/foo.rs. Is active work already there?
```

## First evidence contract

Exact repository-path overlap is `PROVEN` relative to the supplied inventory and focus set:

```text
(current changed paths ∪ focus paths) ∩ active work changed paths != empty
```

The analyzer does **not** infer:

- duplicate intent;
- ownership;
- incompatibility;
- required coordination;
- whether the other worker is still actively executing;
- semantic independence when paths do not overlap.

A heads-up asks only:

> Is there anything worth reconciling before continuing?

## Quietness contract

No overlap means no heads-up entry.

The normal human-facing result is simply:

```text
No direct active-work path overlap worth surfacing.
```

The first experiment caps returned heads-ups and reports omissions so a busy repository cannot silently turn into an unbounded context dump.

For CI, quiet runs use an even cheaper demand-driven path: GitHub's provider-normalized repository paths are intersected first. If those exact sets are disjoint, the Rust analyzer is skipped. A possible overlap still goes through the Rust evidence model before any heads-up is rendered.

## Why PR-relative paths instead of `preflight --against` every PR head?

PR comparison and arbitrary-ref comparison are related but not identical evidence problems.

`cargo cultist preflight --against REF` compares two Git change sets from their common merge base. That is the right deterministic local primitive for two refs whose relationship is the question.

For live PR awareness, GitHub exposes each PR's own changed-path set relative to its configured base. Using that metadata avoids accidentally treating unrelated base-branch movement as one worker's change when two PR heads have different freshness or ancestry.

The remote adapter therefore supplies PR-relative changed paths directly. Future adapters can supply equivalent work-item change sets from other systems.

## Executed dogfood

### Organic quiet negative

Workflow run:

```text
run:      32230058489
job:      95997754356
artifact: 9356901801
sha256:   9d218b3d2e534ac52ace2361d4088bd5d38927eaaaa86609dcd8e7f2cf31a34f
```

At observation time, #98 and #95 were both open. The adapter reported:

```text
inventory: 2 open PR(s), current #98, 3 current path(s)
examined: 1 active candidate(s); self excluded: 1
No direct active-work path overlap worth surfacing.
```

This is a useful negative control: another active work item existed, but its changed paths were disjoint from #98, so the analyzer did not manufacture a coordination warning.

### Live pre-edit focus positive

Workflow run:

```text
run:      32230448378
job:      95998910861
artifact: 9357034687
sha256:   a3fc0d5df95c7a0f22484b7bf3b296a57759d642a6fa9db7493b303c3767a6d7
```

The ordinary advisory stream in that run stayed quiet. A separately labeled positive control then used real active #95 metadata.

#95 changed:

```text
.github/workflows/decision-memory-accepted-ref-research.yml
research/decision-memory/current-only-authority-fixture.json
```

Before the current lane touched the first path, focus mode reported:

```text
focus control: before touching '.github/workflows/decision-memory-accepted-ref-research.yml',
heads-up surfaced active #95 at 7fbfd6ca
```

This proves the pre-edit question against a real active work item without fabricating an overlapping PR.

The positive control was removed after capture so routine dogfood measures natural signal rather than forced positives.

### Exact-head repository CI for the first slice

Head:

```text
3173e650695e86eeb3e82f078504ed78dff10ed8
```

CI run `32230688380` passed rustfmt, Clippy with warnings denied, full tests, repository/history/CI-test dogfood, and PR diff text + JSON dogfood.

## Post-activation cost dogfood

The first activation used a separate workflow and paid a cold Cargo build for a tiny advisory. That was useful for proving behavior, but too expensive to leave as the normal carrier.

PR #100 moved the advisory into the existing PR CI job and then reduced the common-case cost in three steps.

### Integrated, serial REST inventory

Run `32231395340`, job `96001778887`:

```text
2 open PRs
active-work step: about 7.84 seconds
result: quiet
```

The same CI job passed all ordinary quality and dogfood gates.

### Batched GraphQL inventory

Run `32231795244`, job `96003016417`:

```text
4 open PRs
active-work step: about 6.02 seconds
result: quiet
```

The adapter retrieves the common inventory in one GraphQL call and paginates only explicit overflow cases.

### Demand-driven quiet path

Run `32231944337`, job `96003475258`:

```text
5 open PRs
inventory: 1.87 seconds
Rust analyzer: 0.00 seconds (skipped after exact provider-path prefilter)
result: quiet
```

The exact head still passed the full CI and dogfood suite.

This is the desired continuous-dogfood cost model: quiet runs pay for a bounded metadata snapshot and a set intersection; potential overlaps pay the additional deterministic analyzer cost.

## Repository-coordinate freshness dogfood

During this work the GitHub repository was renamed from:

```text
teamleaderleo/cargo-cultist
```

to:

```text
teamleaderleo/cultist
```

Old repository references redirected for many operations, but exact file access exposed the stale coordinate. The CI adapter remained correct because it reads `GITHUB_REPOSITORY` at execution time; subsequent runs checked out and queried `teamleaderleo/cultist` without a hard-coded repository identity.

This is a useful future project-memory rule: repository/provider coordinates are freshness-sensitive evidence. A consumer should preserve the observed coordinate and refresh it rather than silently treating an old identifier as timeless truth.

## Active branches: two useful controls

PR inventory is only one live-work source. The checkout exposed active branch names too, but branch existence alone is not enough evidence for a heads-up.

Negative control:

```text
feature/preflight-active-inventory
  compared with main: identical
  ahead: 0
  behind: 0
```

Despite its relevant-looking name, it carries no divergent work and should not interrupt anyone.

Positive inventory seed:

```text
rename/cultist-product-brand
  compared with main: ahead by 3 commits
  changed paths:
    Cargo.toml
    README.md
    ROADMAP.md
```

That branch is genuine divergent work, but it is disjoint from the #100 CI/adapter lane, so even a future branch-aware heads-up should stay quiet for #100.

A branch adapter therefore needs at least a divergent head/change-set plus freshness evidence; branch-name similarity is not authority or intent.

## Normal dogfood activation

After #100, the advisory lives as one non-blocking step in the existing PR CI job rather than a separate workflow.

Properties:

- PR-only;
- read-only GitHub permissions;
- adapter/analyzer failures use `continue-on-error` and cannot block ordinary development;
- a heads-up itself never changes the job result;
- quiet runs use the demand-driven prefilter and skip Rust;
- possible overlaps are validated by the Rust analyzer;
- results are printed in logs and rendered into the GitHub Actions job summary;
- the adapter discovers the current repository coordinate from runtime metadata.

The standalone research workflow was retired after its behavior receipts were captured.

## Evaluation labels

For each naturally surfaced interruption, record one of:

```text
useful
irrelevant
missing stronger evidence
misleading
```

Useful future enrichments should be added one evidence class at a time with negative controls:

- explicit coordination/intent references;
- divergent active branches;
- symbol overlap;
- historical companions + counterexamples;
- generated ownership relationships;
- explicit policy/oracle overlap;
- decisions / known exceptions;
- head freshness changes that invalidate earlier evidence.

Do not collapse these into one hidden conflict score.

Issue #99 owns a real agent-heavy corpus for explicit coordination/intent relationships such as `depends on`, `blocks`, `supersedes`, and evidence-coordinate sequencing. Those relationships should remain a separate evidence layer from direct path overlap.

## Success criterion

This experiment earns product promotion if repeated real work shows that its heads-ups change investigation or coordination behavior usefully while quiet runs remain common.

The desired product property is:

> Increase worker awareness without reducing worker autonomy.

Refs #96, #99, #62, #74, #41.

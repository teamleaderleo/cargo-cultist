# Active-work heads-up research

Date: 2026-08-19

## Question

Can Cargo Cultist cheaply surface live concurrent work that a coding agent would reasonably want to know about, without becoming a scheduler, lock manager, or GitHub-dependent core?

The first experiment asks only:

> Does another currently open work item modify an exact repository path that this work item already modifies or explicitly intends to inspect/change?

That is intentionally smaller than semantic conflict detection. The goal is to dogfood the interruption policy before broadening the evidence model.

## Adapter boundary

Remote project state and local evidence analysis stay separate:

```text
GitHub / orchestrator / other provider
  -> active-work inventory JSON

Cargo Cultist research analyzer
  -> validate + normalize inventory
  -> exact path intersection
  -> advisory heads-ups
```

The core analyzer does not call GitHub and does not need credentials or network access.

The current GitHub Actions research adapter records, for each open PR:

- PR identity, title, and URL;
- exact head ref and head SHA;
- GitHub `updated_at` freshness receipt;
- draft state;
- complete changed-path inventory retrieved through paginated PR-files API calls.

The supplying adapter's completeness and freshness remain separate evidence from the overlap result.

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

The machine report still preserves explicit `UNKNOWN` boundaries, but the workflow's normal human-facing line is simply:

```text
No direct active-work path overlap worth surfacing.
```

The first experiment caps returned heads-ups and reports omissions so a busy repository cannot silently turn into an unbounded context dump.

## Why PR-relative paths instead of `preflight --against` every PR head?

PR comparison and arbitrary-ref comparison are related but not identical evidence problems.

`cargo cultist preflight --against REF` compares two Git change sets from their common merge base. That is the right deterministic local primitive for two refs whose relationship is the question.

For live PR awareness, GitHub already exposes each PR's own changed-path set relative to its configured base. Using that metadata avoids accidentally treating unrelated base-branch movement as one worker's change when two PR heads have different freshness or ancestry.

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

The positive control was then removed from the normal workflow after capture so routine dogfood measures natural signal rather than forced positives.

### Exact-head repository CI

Head:

```text
3173e650695e86eeb3e82f078504ed78dff10ed8
```

CI run `32230688380` passed:

- rustfmt;
- Clippy with warnings denied;
- full tests;
- repository text + JSON dogfood;
- history text + JSON dogfood;
- CI-test inventory and fixtures;
- PR diff text + JSON dogfood.

The active-work workflow on the same head also completed successfully.

## Normal dogfood activation

The research workflow is designed to run on PR open/synchronize/reopen/ready-for-review events.

It is intentionally advisory:

- a heads-up never fails the job;
- the job uses read-only GitHub permissions;
- old runs for the same PR are cancelled when a new head appears;
- the research job is marked `continue-on-error` so adapter/probe failure cannot block ordinary development;
- only the raw inventory and natural advisory report are retained after the positive-control receipt.

This makes it cheap enough to leave enabled while signal quality is measured.

## Evaluation labels

For each naturally surfaced interruption, record one of:

```text
useful
irrelevant
missing stronger evidence
misleading
```

Useful future enrichments should be added one evidence class at a time with negative controls:

- exact issue / explicit intent references;
- symbol overlap;
- historical companions + counterexamples;
- generated ownership relationships;
- explicit policy/oracle overlap;
- decisions / known exceptions;
- head freshness changes that invalidate earlier evidence.

Do not collapse these into one hidden conflict score.

Issue #99 now owns a real agent-heavy corpus for explicit coordination/intent relationships such as `depends on`, `blocks`, `supersedes`, and evidence-coordinate sequencing. Those relationships should remain a separate evidence layer from direct path overlap.

## Success criterion

This experiment earns product promotion if repeated real work shows that its heads-ups change investigation or coordination behavior usefully while quiet runs remain common.

The desired product property is:

> Increase worker awareness without reducing worker autonomy.

Refs #96, #99, #62, #74, #41.

# Active-work heads-up research

Date: 2026-08-19

## Question

Can Cargo Cultist cheaply surface live concurrent work that a coding agent would reasonably want to know about, without becoming a scheduler, lock manager, or GitHub-dependent core?

The first experiment asks only:

> Does another currently open work item modify an exact repository path that this work item also modifies?

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

## First evidence contract

Exact repository-path overlap is `PROVEN` relative to the supplied inventory:

```text
current changed paths ∩ active work changed paths != empty
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

## Why start with PR paths instead of `preflight --against` every branch?

PR comparison and arbitrary-ref comparison are related but not identical evidence problems.

`cargo cultist preflight --against REF` compares two Git change sets from their common merge base. That is the right deterministic local primitive for two refs whose relationship is the question.

For live PR awareness, GitHub already exposes each PR's own changed-path set relative to its configured base. Using that metadata avoids accidentally treating unrelated base-branch movement as one worker's change when two PR heads have different freshness or ancestry.

The remote adapter therefore supplies PR-relative changed paths directly. Future adapters can supply equivalent work-item change sets from other systems.

## First dogfood workflow

Draft PRs carrying `.github/workflows/active-work-heads-up-research.yml`:

1. query all currently open PRs;
2. fetch paginated changed paths for every PR;
3. build a schema-versioned inventory snapshot;
4. run `examples/active_work_heads_up.rs` locally over that snapshot;
5. print only direct overlap heads-ups;
6. upload the raw inventory and report as a research receipt;
7. never fail merely because a heads-up exists.

## Evaluation labels

For each surfaced interruption, record one of:

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

## Success criterion

This experiment earns product promotion if repeated real work shows that its heads-ups change investigation or coordination behavior usefully while quiet runs remain common.

The desired product property is:

> Increase worker awareness without reducing worker autonomy.

Refs #96, #62, #74, #41.

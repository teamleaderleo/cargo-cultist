# Held-out generated-companion replay: Oxc linter registries

Date: 2026-08-19

Status: successful held-out positive + exact historical negative-control result for issue #35 / PR #68.

## Question

Can Cargo Cultist combine independently recovered repository evidence into a useful missing-generation question on a real change that is excluded from the history used to learn the relationship?

The evidence layers are intentionally separate:

```text
current Rust syntax change
+ literal generator source -> output ownership
+ generated-file declarations
+ directional semantic history cohort
+ current output absence
= bounded question
```

## Held-out target

Oxc target commit:

```text
228e8e0f85c0e7aeded02c5e27fd810004d3b41a
fix(linter): resolve inactive React compiler rules (#25830)
```

First parent:

```text
568203e24f090f2cc3f945d611e605b864842bf0
```

The real target commit changed `crates/oxc_linter/src/rules.rs` and both core generated registries:

```text
crates/oxc_linter/src/generated/rule_runner_impls.rs
crates/oxc_linter/src/generated/rules_enum.rs
```

The replay deliberately withheld those generated changes.

## Counterfactual construction

The workflow:

1. checked out the target with full history;
2. saved only the target-parent -> target patch for `crates/oxc_linter/src/rules.rs`;
3. reset the repository to the target's first parent;
4. applied only that source patch;
5. verified the worktree diff contained exactly `rules.rs`;
6. ran the composed analyzer from the parent state.

Therefore the target commit itself was absent from the historical cohort used by the analyzer. The generated outputs were absent from the current diff by construction, while their current parent-state markers, `.gitattributes` entries, generator code, Cargo aliases, and earlier history remained available.

## Exact Cargo Cultist receipt

Experiment head:

```text
746d0ab4e6786099baa618563653c8ddc62c4660
```

Generic CI:

```text
run:    32217601800
result: success
```

Held-out research workflow:

```text
run:    32217601874
job:    95961927494
result: success
```

Artifact:

```text
id:     9352863272
name:   generated-missing-companion-research
sha256: faac0d7d5cd66f1f28353562b2b1ccbcda1577be031ecd1dfc03d118f93f258d
```

The experiment's own fmt, Clippy, and unit-test gates passed before the corpus replay.

## Held-out positive result

The source-only counterfactual was classified as:

```text
source path changed in worktree: true
source Rust syntax changed: true
```

The analyzer then emitted:

```text
FINDING: generated companions absent from a source-syntax change
```

### Explicit / derived evidence

It independently recovered:

```text
generate_rule_runner_impls
  reads  crates/oxc_linter/src/rules.rs
  writes crates/oxc_linter/src/generated/rule_runner_impls.rs

generate_rules_enum_file
  reads  crates/oxc_linter/src/rules.rs
  writes crates/oxc_linter/src/generated/rules_enum.rs
```

For both outputs it also found:

```text
.gitattributes: linguist-generated=true
line 1: // Auto-generated code, DO NOT EDIT DIRECTLY!
```

and the repository Cargo alias:

```text
cargo lintgen -> run -p oxc_linter_codegen
```

### Observed directional precedent

Using only history available at the target parent:

```text
Rust syntax-change cohort: 99 comparable commits
comments/docs-only commits: 1
unclassified commits: 0

rule_runner_impls.rs  99/99  100.0%
rules_enum.rs         99/99  100.0%
```

### Current negative space

Both owned generated outputs were absent from the current source-only worktree diff.

The final output preserved intent uncertainty:

```text
UNKNOWN
  Repository evidence establishes generation ownership and historical precedent,
  but it does not establish whether this current absence is intentional.

QUESTION
  Was `cargo lintgen` intentionally deferred for this source change,
  or are the generated companions stale?
```

## Exact historical negative control

The same workflow then replayed the known historical exception:

```text
5e113baf716b9f3781331b268b4142d23cac0541
docs(linter): add license notices for ported ESLint plugins (#22768)
```

It again withheld every companion and applied only the `rules.rs` patch to that commit's first parent.

The analyzer classified:

```text
source path changed in worktree: true
source Rust syntax changed: false

NO FINDING
  The source path changed, but its normalized Rust syntax is unchanged
  after comments/docs/whitespace are removed.
```

The workflow explicitly failed if a generated-companion finding appeared in this control. It stayed quiet.

## Why this result is stronger than the earlier synthetic packet

The earlier composition experiment manually removed one known rule registration from a pinned Oxc checkout. That proved the evidence layers could be assembled.

This replay withholds a real future commit from the analyzer's history and asks the detector to rediscover the missing outputs from the target's parent state. The actual target commit independently confirms that both generated registries really changed.

The exact historical docs-only exception supplies the opposite cell.

So the first held-out matrix is:

```text
real semantic source change whose actual commit regenerated outputs
  -> source-only counterfactual emits bounded finding

real docs/license-only source change whose actual commit omitted those outputs
  -> source-only counterfactual emits NO FINDING
```

## Evidence boundary

This result establishes value for the Oxc-style relation family under the current literal adapters. It does not justify a universal generated-file rule.

Remaining limits include:

- generator paths built dynamically rather than from recognized literals;
- source/output relations that cross helper calls or scripts/languages outside the Rust adapter;
- syntax changes that genuinely leave generator output bytes unchanged;
- relationship eras where ownership moved between generators;
- current intentional deferral or split-commit workflows.

The finding should therefore keep explicit evidence, cohort counts, counterexamples, and `UNKNOWN` intent even after product extraction.

## Disposition

**Promote the idea, extract the implementation.**

The research carrier is intentionally broad and duplicates logic from earlier examples. The held-out result is strong enough to justify a smaller reusable product module and a `diff`-level evidence packet. Preserve this experiment as the proof specimen; avoid making the 800-line example the long-term implementation boundary.

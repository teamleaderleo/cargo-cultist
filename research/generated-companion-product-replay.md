# Generated-companion product replay: held-out Oxc diff

Date: 2026-08-19

Status: successful product extraction of the held-out generated-companion proof into `cargo cultist diff`.

## Product question

Can the normal diff analyzer emit a bounded missing-generation finding by composing independently recovered repository evidence, while staying quiet for the exact historical docs-only exception?

The first product slice requires all of these facts:

```text
current source path changed
+ normalized Rust syntax changed
+ Cargo alias names a generator package
+ generator entrypoint literally reads source and writes output
+ output self-identifies as generated
+ exact output path is linguist-generated=true
+ output is absent from the current diff
+ output changed in every comparable source-syntax history commit
= provenance-bearing question
```

Intent and output-byte effect remain `UNKNOWN`.

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

The actual target independently changed:

```text
crates/oxc_linter/src/rules.rs
crates/oxc_linter/src/generated/rule_runner_impls.rs
crates/oxc_linter/src/generated/rules_enum.rs
```

The replay saved only the target-parent -> target `rules.rs` patch, reset to the parent, and applied the source patch alone. The target commit was therefore absent from the history used to evaluate precedent, and both generated outputs were deliberately absent from the worktree diff.

## First extraction failure: removal-only diff

The initial product build passed rustfmt, Clippy, unit tests, and ordinary Cargo Cultist dogfood, but returned zero generated-companion findings on the held-out target.

A diagnostic replay on the same counterfactual proved the independent evidence adapters were still healthy:

```text
cargo lintgen -> run -p oxc_linter_codegen

generate_rule_runner_impls
  reads  crates/oxc_linter/src/rules.rs
  writes crates/oxc_linter/src/generated/rule_runner_impls.rs

generate_rules_enum_file
  reads  crates/oxc_linter/src/rules.rs
  writes crates/oxc_linter/src/generated/rules_enum.rs

semantic cohort:
  rule_runner_impls.rs  99/99  100.0%
  rules_enum.rs         99/99  100.0%
```

The loss was in diff plumbing. The held-out `rules.rs` patch is removal-only. The pre-existing test-module analyzer intentionally defined its Rust worklist from **added lines**, so `build_diff_analysis_report` reached an early return before the generated-companion analyzer ran.

The repair separated the two facts:

```text
changed path
  -> generated negative-space analysis

file with parsable added lines
  -> test-module naming analysis
```

Generated-companion analysis now runs before the added-line early return. The legacy test-module analyzer keeps its existing added-line semantics.

## Clean product receipt

Clean product head after that repair and removal of the temporary write-capable wiring workflow:

```text
50f50fbad9c299f8156263670fe86c06dd3b036b
```

Generic CI:

```text
run:    32218582111
job:    95964645017
result: success
```

That run passed:

- rustfmt;
- Clippy with warnings denied;
- all unit tests;
- repository text + JSON dogfood;
- history text + JSON dogfood;
- `ci-tests` text + JSON + stale/matching fixture;
- diff text + JSON dogfood.

So the path-level generated analysis did not disturb the original added-line analyzer.

## Held-out product replay

Read-only product replay:

```text
run:    32218582180
job:    95964645621
result: success
```

The workflow first re-proved generator ownership and the 99/99 syntax cohort on the exact same parent-only counterfactual, then invoked the real product binary:

```text
cargo-cultist diff --format json corpus/oxc
```

The product emitted exactly two findings:

```text
generated-companion-missing
  -> crates/oxc_linter/src/generated/rule_runner_impls.rs

generated-companion-missing
  -> crates/oxc_linter/src/generated/rules_enum.rs
```

Each finding carried claim kinds in this order:

```text
derived
derived
observed
observed
unknown
```

The evidence included:

- current normalized Rust syntax change and output absence;
- `cargo lintgen` / `oxc_linter_codegen` generator ownership;
- exact generated marker and `.gitattributes` declaration;
- `99/99 comparable Rust syntax-changing commits`;
- `UNKNOWN` current intent / whether the source edit changes emitted bytes.

## Exact negative control

The same product replay then checked out:

```text
5e113baf716b9f3781331b268b4142d23cac0541
docs(linter): add license notices for ported ESLint plugins (#22768)
```

It again applied only that commit's `rules.rs` patch to its first parent and withheld every companion.

The normalized Rust syntax fingerprint was unchanged, and the real product command produced zero `generated-companion-missing` findings.

The workflow explicitly failed if any such finding appeared.

## Evidence boundary

This is a high-precision first slice, not a universal generated-file detector.

Current product limits include:

- Cargo aliases that do not resolve to `cargo run -p PACKAGE`;
- generator packages without a default Rust `src/main.rs`;
- paths built dynamically or passed across helper/function boundaries;
- non-Rust generator implementations;
- generated files without both strong header evidence and exact `.gitattributes` declaration;
- relation eras or migrations;
- histories with any syntax-cohort counterexample;
- source syntax edits that legitimately leave generated bytes unchanged.

Those limits are deliberate. The first product finding asks a question only when explicit generator ownership, generated identity, current negative space, and zero-counterexample semantic precedent all agree.

## Product lesson

Diff analyzers should preserve the semantic distinction between:

```text
path changed
added line exists
removed line exists
syntax changed
```

They are related facts with different consumers. Collapsing them into one generic 'changed Rust file' concept caused the first product extraction to silently skip a valid removal-only negative-space case.

## Disposition

**Product behavior validated.** Retire the temporary network replay after this receipt, run cleanup CI on the current branch, then merge the bounded analyzer. Follow with code consolidation separately so the held-out behavior stays fixed while shared Rust/history adapters are reduced.

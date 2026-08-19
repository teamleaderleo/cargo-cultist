# Rust syntax cohort replay: Oxc linter registries

Date: 2026-08-19

Status: successful research result for issue #35 / PR #54. This note records a cohort-refinement experiment; it does not turn 100% support into a universal regeneration rule.

## Question

Can file-level historical precedent become more specific by separating Rust edits that change lexical syntax from edits that only change comments, doc attributes, or whitespace?

The seed result from the earlier history replay was unusually clean:

```text
anchor: crates/oxc_linter/src/rules.rs
core generated companions: 99/100 each
sole absence counterexample:
  docs(linter): add license notices for ported ESLint plugins (#22768)
```

## Exact receipt

Cargo Cultist PR head:

```text
dd4ebc41add552699d7b5cf251e55f4b77c955df
```

Pinned target:

```text
oxc-project/oxc@8783524015b1e6ff1c39ccf426df0bb07cbbc588
```

GitHub Actions:

```text
workflow: Rust syntax cohort research
run:      32216114110
job:      95957849669
result:   success
artifact: 9352452984
digest:   sha256:38f8afa02bb33046ececea0b8302a301230bdbc9b243f70a3962c46672f5d305
```

Generic Cargo Cultist CI on the same head also passed in run `32216114098`.

## Probe semantics

`examples/rust_syntax_cohort.rs` compares the anchor at each focused commit with its first parent, tokenizes both versions with `proc_macro2`, strips doc attributes recursively, and compares normalized token streams.

Current classes:

- `SyntaxChanged`
- `CommentsOrWhitespaceOnly`
- `Unclassified`

Ordinary comments disappear during tokenization. `#[doc = ...]` attributes are explicitly removed. Other attributes, including `#[cfg(...)]`, remain part of the syntax fingerprint.

This is lexical Rust syntax identity. It is not an AST-semantic equivalence test and does not attempt to decide whether two different token streams have equivalent behavior.

## Forward result: source registry -> generated registries

Anchor:

```text
crates/oxc_linter/src/rules.rs
```

Cohort:

```text
discovered non-merge commits:        100
focused commits:                     100
syntax-changing commits:              99
comments/docs/whitespace-only:         1
unclassified:                          0
excluded reverts:                      0
excluded broad commits:                0
```

The one comments/docs-only commit was exactly the known counterexample:

```text
5e113baf  docs(linter): add license notices for ported ESLint plugins (#22768)
```

After removing that one lexical-nonsemantic edit from the cohort:

```text
crates/oxc_linter/src/generated/rule_runner_impls.rs
  syntax cohort: 99/99  100.0%
  all focused:   99/100  99.0%

crates/oxc_linter/src/generated/rules_enum.rs
  syntax cohort: 99/99  100.0%
  all focused:   99/100  99.0%
```

The lower-support companion tiers changed only slightly because the removed commit did not own their missing updates:

```text
config.generated.ts              59/99  59.6%
configuration_schema.json        59/99  59.6%
website schema snapshot          37/99  37.4%
linter timing snapshot           10/99  10.1%
```

## Reverse control: generated registry -> source registry

Anchor:

```text
crates/oxc_linter/src/generated/rules_enum.rs
```

All 100 sampled edits changed lexical Rust syntax. The important reverse support stayed unchanged:

```text
rule_runner_impls.rs  94/100  94.0%
rules.rs              94/100  94.0%
```

So the cohort refinement did not manufacture symmetry. The previously observed directionality remains:

```text
P(generated changes | source syntax changes) = 99/99 = 100%
P(source changes | generated syntax changes) = 94/100 = 94%
```

## Design consequence

This result strongly favors **cohort refinement over a higher universal frequency threshold**.

The raw 99% relationship already carried a visible counterexample. Classifying the anchor edit removed exactly the known irrelevant source change and produced a perfect forward cohort while preserving the reverse asymmetry.

A future negative-space finding can therefore compose evidence in layers:

```text
changed anchor has relevant Rust syntax change
+ companion declares itself generated
+ explicit generator/source ownership (future adapter)
+ directional historical cohort has strong support
+ companion absent from current diff
-> evidence-backed regeneration question
```

Each layer should remain separately labeled. The 100% historical result is empirical evidence, not proof of generator ownership or intent.

## Boundaries

- File creation/deletion and revisions that cannot be parsed are currently `Unclassified`.
- Macro/token changes count as syntax changes even when their downstream semantic effect is uncertain.
- Comments and doc attributes can occasionally influence tooling outside ordinary Rust semantics; this probe deliberately treats them as a separate cohort class, not as globally irrelevant.
- Rename following, era detection, generator ownership, and current-diff integration remain separate work.

## Disposition

**Continue.** The first semantic-cohort experiment succeeded cleanly on its intended discriminator. The next useful step is to combine this result with explicit generator ownership rather than promote a numeric threshold by itself.

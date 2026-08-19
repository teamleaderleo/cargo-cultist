# Generated-companion provenance restoration replay

Date: 2026-08-19

Status: successful product restoration for issue #80 / PR #90.

## Question

Can `generated-companion-missing` return to the public diff analyzer while generator ownership uses the same proven repository-root semantics as the research adapter that passed the Oxc replay in #70?

The restoration gate required one canonical ownership implementation, adversarial root-provenance controls, the original held-out semantic Oxc positive, and the exact docs/license historical exception.

## Canonical ownership boundary

Generator ownership now lives in one physical implementation:

```text
src/generator_ownership.rs
```

Both consumers compile that exact file:

```text
examples/rust_generator_ownership.rs
  -> research view over canonical ownership

src/generated_diff.rs
  -> product consumer of canonical GeneratorRelation values
```

The product diff analyzer no longer contains an independent repository-path ownership parser.

The canonical module owns:

- explicit repository-root provider recognition;
- literal repository-relative `.join("...")` paths only from proven root bindings;
- local path bindings derived from those proven roots;
- exact `fs::{read,read_to_string,write}` qualification;
- Cargo alias -> `cargo run -p PACKAGE` resolution;
- default generator package `src/main.rs` discovery;
- source -> generated-output relation assembly;
- exact `.gitattributes` `linguist-generated=true` discovery.

The first recognized root-provider vocabulary remains deliberately narrow:

```text
project_root::get_project_root()
```

Unresolved receivers are omitted instead of inferred from variable names.

## Adversarial regression controls

The canonical module carries the #70 provenance controls directly into product compilation:

```text
explicit supported repository-root provider
  -> accepted

arbitrary PathBuf parameter named `root`
  -> rejected

collection/string `.join(...)`
  -> rejected

dynamic join suffix
  -> rejected

non-filesystem `read` / `write` lookalikes
  -> rejected
```

The same module also retains package/alias and `.gitattributes` parsing controls.

## Exact held-out positive

Oxc target:

```text
228e8e0f85c0e7aeded02c5e27fd810004d3b41a
fix(linter): resolve inactive React compiler rules (#25830)
```

First parent:

```text
568203e24f090f2cc3f945d611e605b864842bf0
```

The workflow saved only the target-parent -> target patch for:

```text
crates/oxc_linter/src/rules.rs
```

It then reset to the first parent, applied only that source patch, and verified the worktree diff contained exactly `rules.rs`. The real target's two generated changes were withheld, and the target commit itself remained absent from the history used for precedent.

## Canonical ownership result on the counterfactual

The shared research view recovered:

```text
generator source: tasks/linter_codegen/src/main.rs
generator package: oxc_linter_codegen

cargo lintgen -> run -p oxc_linter_codegen

function generate_rule_runner_impls
  reads  crates/oxc_linter/src/rules.rs
  writes crates/oxc_linter/src/generated/rule_runner_impls.rs
         [.gitattributes: linguist-generated=true]

function generate_rules_enum_file
  reads  crates/oxc_linter/src/rules.rs
  writes crates/oxc_linter/src/generated/rules_enum.rs
         [.gitattributes: linguist-generated=true]
```

The evidence boundary printed by the probe explicitly states that path relations require a recognized repository-root provider and that unresolved roots are omitted.

## Product result

The public binary was invoked directly:

```text
cargo-cultist diff --format json corpus/oxc
```

It emitted exactly two `generated-companion-missing` findings:

```text
crates/oxc_linter/src/generated/rule_runner_impls.rs
crates/oxc_linter/src/generated/rules_enum.rs
```

Each finding carried claim kinds in this exact order:

```text
derived
derived
observed
observed
unknown
```

For both outputs the product recovered:

- current normalized Rust syntax changed in `rules.rs`;
- generated output absent from the current diff;
- canonical `cargo lintgen` / `oxc_linter_codegen` source -> output ownership;
- line-1 generated marker `// Auto-generated code, DO NOT EDIT DIRECTLY!`;
- exact `.gitattributes` generated declaration;
- `99/99` comparable Rust syntax-changing commits (`100.0%`);
- one comment/doc/whitespace-only historical source commit excluded from the cohort;
- explicit `UNKNOWN` current intent and current generated-byte effect.

Each finding ended with a bounded stale-or-deferred question.

## Exact historical negative control

Known exception:

```text
5e113baf716b9f3781331b268b4142d23cac0541
docs(linter): add license notices for ported ESLint plugins (#22768)
```

The workflow again applied only that commit's `rules.rs` patch to its first parent and withheld every generated companion.

The public product emitted:

```text
findings: []
generated findings: 0
```

So the restored product remains quiet for the exact docs/license exception that sharpened the historical cohort in #54.

## Exact execution receipt

Restoration head that executed the matrix:

```text
4ba397d7927d954f52095608d26a635a8f814604
```

Generic CI on the same head:

```text
run:    32220853227
result: success
```

Every substantive generic step passed:

- rustfmt;
- Clippy with warnings denied;
- full tests;
- repository text + JSON dogfood;
- history text + JSON dogfood;
- CI-test text + JSON + fixture;
- diff text + JSON dogfood.

Dedicated held-out replay:

```text
run:    32220853243
job:    95970901277
result: success
```

Artifact:

```text
id:     9353882794
name:   generated-provenance-restoration-research
sha256: e778aabec7d8dcc6b9570b06335027de498d22339db93e6f3d5f839613d9343c
```

Artifact contents:

```text
oxc-generator-ownership.txt
oxc-held-out-product.json
oxc-docs-only-product.json
```

## Design result

The containment in #82 did its job: product findings stayed silent until the stricter provenance semantics were physically shared with the successful research adapter.

The restoration now has this implementation boundary:

```text
canonical generator ownership
  -> proven repository-root relations

current diff + generated identity + semantic history
  -> generated-companion finding evaluation
```

This removes the previous failure mode where research and product could drift into different notions of repository-relative path ownership.

## Evidence boundary

The restored detector remains intentionally narrow. Current omissions include:

- repository-root providers outside the explicitly recognized vocabulary;
- dynamic or helper-crossing path construction;
- generator implementations outside the current Rust/default-main adapter;
- Cargo aliases outside the supported `run -p/--package` form;
- relation eras or generator ownership migrations;
- histories with semantic-cohort counterexamples;
- source syntax edits that legitimately leave generated bytes unchanged.

Intent stays `UNKNOWN` even when every deterministic prerequisite agrees.

## Disposition

**Restore the bounded product finding.**

The #80 acceptance matrix is satisfied on the executed branch head. Retire the temporary external replay workflow, run final source-only CI, and merge the canonical ownership restoration if that cleaned head remains green.

# Generated-companion provenance restoration replay

Date: 2026-08-19

Status: successful product restoration for issue #80 / PR #90, including a review-dogfood provenance attack found after the first successful held-out matrix.

## Question

Can `generated-companion-missing` return to the public diff analyzer while generator ownership uses the same proven repository-root semantics as the research adapter that passed the Oxc replay in #70?

The final restoration gate required:

- one canonical ownership implementation;
- adversarial root-provenance controls;
- an additional derived-subdirectory provenance attack from review dogfood;
- the original held-out semantic Oxc positive;
- the exact docs/license historical exception;
- ordinary repository CI and dogfood.

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

The product diff analyzer contains no independent repository-path ownership parser.

The canonical module owns:

- explicit repository-root provider recognition;
- repository-root versus derived repository-path provenance;
- literal repository-relative `.join("...")` composition;
- local path bindings derived from proven roots or proven relative paths;
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

It also rejects repository-relative literal paths containing `..` parent-directory escapes.

The same module retains package/alias and `.gitattributes` parsing controls.

## Review dogfood found a stronger provenance bug

After the first held-out Oxc matrix had already passed, the repository's generated-provenance review dogfood added this counterexample:

```rust
let tasks = project_root::get_project_root()?.join("tasks");
let source = std::fs::read_to_string(tasks.join("src/rules.rs"))?;
let target = tasks.join("generated/rules.rs");
```

The first canonical implementation treated any initializer containing the recognized root provider as a repository-root binding. That promoted `tasks` itself to repository root and incorrectly reported:

```text
reads  src/rules.rs
writes generated/rules.rs
```

instead of preserving the `tasks/` prefix.

The dogfood run failed explicitly with:

```text
derived subdirectory was incorrectly promoted to repository root
```

### Provenance repair

Path provenance now distinguishes:

```text
RepositoryRoot
DerivedRepositoryPath("tasks")
```

A literal join composes those values instead of replacing root identity:

```text
RepositoryRoot.join("tasks")
  -> DerivedRepositoryPath("tasks")

DerivedRepositoryPath("tasks").join("src/rules.rs")
  -> DerivedRepositoryPath("tasks/src/rules.rs")
```

The canonical unit suite now requires:

```text
reads  tasks/src/rules.rs
writes tasks/generated/rules.rs
```

and explicitly rejects the prefix-losing forms.

Final review-dogfood run:

```text
run:    32221343868
result: success
```

The `derived-root` job passed both the canonical research-view build and the derived-subdirectory rejection control.

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

The evidence boundary printed by the probe states that path relations require a recognized repository-root lineage and that unresolved roots are omitted.

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

## Final exact execution receipt

Final semantic-restoration head that executed the full matrix after the derived-path provenance repair:

```text
4796ed643e1ed8bfe4e0b8fe61df099214aee80d
```

Generic CI on the same head:

```text
run:    32221343983
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

Review-dogfood provenance control:

```text
run:    32221343868
result: success
```

Dedicated held-out replay:

```text
run:    32221343877
job:    95972249274
result: success
```

Artifact:

```text
id:     9354031900
name:   generated-provenance-restoration-research
sha256: 9143c1d2940fb5f056b814816253dcfc8569e362c93156d7963d870602534c8a
```

Artifact contents:

```text
oxc-generator-ownership.txt
oxc-held-out-product.json
oxc-docs-only-product.json
```

## Design result

The containment in #82 did its job: product findings stayed silent until the stricter provenance semantics were physically shared with the successful research adapter.

The review dogfood then found a second failure class after the first product replay had already gone green. Keeping that review gate active strengthened the path model before release.

The restoration now has this implementation boundary:

```text
canonical generator ownership
  -> proven root / derived-path relations

current diff + generated identity + semantic history
  -> generated-companion finding evaluation
```

This removes the previous failure mode where research and product could drift into different notions of repository-relative path ownership, and it preserves path prefixes across derived repository directories.

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

The #80 acceptance matrix and the additional derived-root review attack are satisfied on the final executed branch head. Retire the temporary external replay workflow, run one final source-only CI/review-dogfood pass, and merge the canonical ownership restoration if that cleaned head remains green.

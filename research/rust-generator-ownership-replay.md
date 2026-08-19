# Rust generator ownership replay: Oxc linter registries

Date: 2026-08-19

Status: successful research result for explicit source-to-generated ownership recovery.

## Question

Can Cargo Cultist recover a real source -> generated-output relation from deterministic repository code without hard-coding the target repository?

Earlier Cargo Cultist experiments had already established two independent Oxc facts:

1. `crates/oxc_linter/src/rules.rs` has a 99/99 = 100% directional historical relationship with both core generated registries when the cohort is restricted to Rust syntax-changing source edits;
2. both generated registry files explicitly identify themselves as generated and are marked `linguist-generated=true` in `.gitattributes`.

Those facts still leave one important question open: what repository mechanism actually owns the derivation?

## Exact inputs

Cargo Cultist executed probe head:

`teamleaderleo/cargo-cultist@5192a4aae16f2ae9e8d5f670abd969912cae03fb`

Pinned Oxc source:

`oxc-project/oxc@8783524015b1e6ff1c39ccf426df0bb07cbbc588`

Generator source:

`tasks/linter_codegen/src/main.rs`

GitHub Actions:

- workflow: `Rust generator ownership research`
- run: `32216773896`
- job: `95959664054`
- result: success
- artifact: `9352598772`
- artifact digest: `sha256:8147a985fef292a5c4eda48764b4fec86bb58feb738dbfbec643f6a65eab7cb4`

The workflow checked out Oxc read-only and asserted every expected relation with exact `grep` controls.

## Probe model

The standalone `examples/rust_generator_ownership.rs` experiment intentionally recognizes a narrow deterministic subset:

- literal repository paths passed through `.join("...")`;
- simple local bindings of those paths;
- `fs::read`, `fs::read_to_string`, and `fs::write` calls;
- reads and writes occurring in one Rust function;
- nearest Cargo package name from `Cargo.toml`;
- Cargo aliases whose command names that package;
- exact `.gitattributes` paths carrying `linguist-generated=true`.

After the first Clippy pass, the I/O matcher was narrowed to paths whose penultimate segment is literally `fs`, avoiding similarly named unrelated calls such as `custom::write`.

Dynamic path construction and broader dataflow remain outside this probe.

## Executed Oxc result

The probe recovered the generator package:

```text
generator package: oxc_linter_codegen
```

It independently recovered the repository command alias:

```text
cargo lintgen -> run -p oxc_linter_codegen
```

It recovered the first explicit source/output relation:

```text
function generate_rule_runner_impls
  reads  crates/oxc_linter/src/rules.rs
  writes crates/oxc_linter/src/generated/rule_runner_impls.rs
         [.gitattributes: linguist-generated=true]
```

It recovered the second relation:

```text
function generate_rules_enum_file
  reads  crates/oxc_linter/src/rules.rs
  writes crates/oxc_linter/src/generated/rules_enum.rs
         [.gitattributes: linguist-generated=true]
```

The workflow asserted all four path/alias lines exactly and passed.

## What this establishes

### 1. Repository code can supply explicit derivation evidence

The historical relationship is no longer only `rules.rs usually changes with generated files`.

There is deterministic repository code that reads the source registry and writes each generated registry.

That supports a typed relation such as:

```text
rules.rs --generator-input-to--> rule_runner_impls.rs
rules.rs --generator-input-to--> rules_enum.rs
```

### 2. Independent evidence channels reinforce each other

For the same Oxc relation Cargo Cultist can now recover:

```text
OBSERVED
  99/99 comparable Rust syntax-changing source edits changed both companions.

PROVEN / DERIVED
  both companions self-identify as generated.

PROVEN / DERIVED
  .gitattributes marks both exact paths generated.

DERIVED
  one Rust generator function reads rules.rs and writes rule_runner_impls.rs.

DERIVED
  another Rust generator function reads rules.rs and writes rules_enum.rs.

PROVEN / DERIVED
  Cargo alias `lintgen` invokes the generator package.
```

No one evidence source has to carry the whole conclusion.

### 3. Generator ownership still does not imply every source edit changes output

The explicit read/write relation establishes mechanism ownership and data movement.

It does not establish that every edit to `rules.rs` changes emitted bytes. A docs-only edit already provides the counterexample.

That is why the syntax-change cohort remains useful beside explicit derivation evidence.

### 4. The likely missing-companion finding now has a strong evidence packet

For a future changed diff:

```text
current diff changes Rust syntax in rules.rs
+ generator reads rules.rs
+ generator writes both generated registries
+ both outputs are explicitly generated
+ historical syntax cohort is 99/99 for each output
+ one or both outputs are absent from the current diff
```

Cargo Cultist can ask:

```text
Was `cargo lintgen` intentionally deferred, or are generated registry companions stale?
```

This question comes from repository evidence rather than a universal Oxc rule embedded in Cargo Cultist.

## Boundary

The current adapter recognizes only a narrow Rust-generator idiom. It intentionally misses:

- dynamic output paths;
- helper functions that obscure path ownership;
- non-Rust generators;
- subprocess generators;
- build.rs ownership;
- templates/manifests that indirectly determine outputs;
- generators whose source and output relation requires broader dataflow.

Those should be added from corpus demand rather than speculative generality.

## Disposition

**Continue and compose.**

The explicit ownership experiment succeeded on the intended real repository discriminator. Its strongest next use is a controlled missing-generation diff finding composed with the merged syntax-cohort and generated-marker evidence.

The generic relation concept that appears to be emerging is:

```text
changed entity
+ typed repository relation
+ cohort relevance
+ companion absence
+ counterexample / exception search
= bounded question
```

# CI test-filter relation research

Date: 2026-08-19

Status: the syntax-only analyzer now has both synthetic controls and a real external acceptance witness for issue #36 / PR #46.

## Question

Can Cargo Cultist identify a repository-maintained CI test selector whose intended test appears to have disappeared or moved, while preserving uncertainty when source syntax alone cannot enumerate the runtime test inventory?

The implemented slice is deliberately narrow:

```text
literal GitHub Actions command
cargo [ +TOOLCHAIN ] test --lib FILTER
        |
        v
explicit Rust #[test] names
+ declared Rust module names
```

A syntax miss produces a question with an explicit `UNKNOWN` for macro/build-time test generation. The syntax analyzer itself does not claim proven zero execution.

## External control: exrs defends against the class

Pinned source:

```text
johannesvollmer/exrs@63b4ef472537d38374d9856f469f16d5b1fcc714
.github/workflows/rust.yml
```

The SIMD SDE job passes matrix filters such as `avx2` and `sse2` directly to the compiled Rust test binary, then asserts the exact expected number of passing tests.

The workflow documents the reason for the count assertion: a libtest name filter that matches nothing can still exit successfully with a `0 passed` result. The count check converts silent zero-selection into a red job.

This remains a useful negative/control case. Its selector reaches the test binary through matrix variables, so it sits outside the literal Cargo-command parser in PR #46.

## Real positive witness: Tantivy zero-selection stayed green

Exact retained carrier:

```text
teamleaderleo/tantivy@b92909ef3d5ac5695d1c85b1b0cb52a03ee51e49
.github/workflows/fieldwork-prepare-commit-generation-fence.yml
```

The workflow contains this literal command:

```text
cargo +1.88.0 test --lib test_rollback --locked --no-default-features
```

Historical execution receipt:

```text
workflow run: 30513367302
job:          90777910777  characterize
conclusion:   success
```

The adjacent positive control selected and passed three `test_prepare_` tests. The rollback selector then executed successfully with:

```text
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1120 filtered out
```

The GitHub Actions job still concluded `success`.

That is the exact failure family this analyzer is meant to surface: a repository-maintained focused test command stayed green while selecting zero intended tests.

## Cargo Cultist replay on the same carrier

PR #46 added support for a simple Cargo toolchain selector (`+1.88.0`) so the exact Tantivy command remains inside the deterministic parser rather than being rewritten for the experiment.

Pinned Cargo Cultist replay:

```text
cargo-cultist head: 3a32359a19be24819f54e2ab0648d52d88ddb19f
workflow:           Tantivy zero-test replay
run:                32216675650
result:             success
```

The replay required exactly one `ci-test-filter-inventory-miss` containing `test_rollback`, located at:

```text
.github/workflows/fieldwork-prepare-commit-generation-fence.yml
```

Generic CI on the same Cargo Cultist head also passed in run `32216675619`.

This establishes the acceptance chain without inflating the analyzer's claim category:

```text
historical Tantivy execution proves zero-selection stayed green
+
current Cultist syntax analyzer identifies that exact selector/location
```

The finding still carries `UNKNOWN` for runtime/generated test inventory because ordinary `ci-tests` is read-only. The external execution receipt supplies the stronger validation for the corpus case.

## Synthetic end-to-end discriminator

Permanent Cargo Cultist CI keeps one disposable repository fixture for the exact supported syntax family.

Fixture A:

```text
src/lib.rs:
  #[test]
  fn existing_test() {}

.github/workflows/test.yml:
  - run: cargo test --lib stale_filter
```

Expected JSON:

- exactly one `ci-test-filter-inventory-miss` finding;
- claim order `proven -> observed -> unknown`;
- question points back to `stale_filter`.

Fixture B changes only the workflow filter to `existing_test` and must produce zero findings.

### Self-dogfood findings

The fixture and review process caught two analyzer mistakes before promotion:

1. the first workflow scanner missed ordinary YAML list-item syntax, `- run: cargo test ...`;
2. an early qualifier heuristic treated arbitrary Rust filesystem path components as possible test-name evidence, which could suppress real misses.

The final slice strips an optional YAML list-item prefix and uses declared Rust module identifiers as conservative qualifier hints.

## Shared rendering integration

While this experiment was open, repository and diff output moved to a shared `AnalysisReport` renderer. `ci-tests` now builds the same provenance-bearing report for text and JSON, preventing the two presentation paths from drifting independently.

## Promotion result

Issue #36's first-slice success criteria are satisfied:

- a real green zero-selection case exists and is pinned;
- the parser stays deliberately narrow;
- the finding exposes workflow location plus explicit syntax-inventory evidence;
- generated/dynamic inventory uncertainty remains visible.

The next research layer is execution-aware selection evidence:

```text
CI selector
-> resolved Cargo target/test binary
-> libtest --list / executed result count
-> repository expectation / precedent
```

That work can prove selection count directly when the user explicitly opts into repository code execution, while ordinary `ci-tests` remains local, read-only analysis.

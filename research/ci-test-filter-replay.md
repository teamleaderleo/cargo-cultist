# CI test-filter relation research

This note records the first external and synthetic controls for issue #36 / PR #46.

## Question

Can Cargo Cultist identify a repository-maintained CI test selector whose intended test appears to have disappeared or moved, without pretending syntax alone proves what Cargo/libtest executes?

The first implementation slice is deliberately narrow:

```text
literal GitHub Actions command
cargo test --lib FILTER
        |
        v
explicit Rust #[test] names
+ inline module names
+ Rust source path components
```

A syntax miss produces a question with an explicit `UNKNOWN` for macro/build-time test generation. It is not proof that the command executes zero tests.

## External control: exrs

Pinned source:

```text
johannesvollmer/exrs@63b4ef472537d38374d9856f469f16d5b1fcc714
.github/workflows/rust.yml
```

The SIMD SDE job uses matrix filters such as `avx2` and `sse2` against the compiled Rust test binary. The workflow then asserts the exact expected number of passing tests.

The workflow documents why that extra assertion exists: a libtest name filter that matches nothing can still exit successfully with a `0 passed` result. The count check converts that silent selection failure into a red job.

This is a valuable negative/control case for the product idea:

- the repository has an explicit local defense against green zero-selection;
- the selector targets module-name precedent (`avx2_tests` / `sse2_tests`);
- an exact count is stronger evidence than inferring intent from names alone.

It is intentionally outside PR #46's parser. The selector is passed directly to the compiled test binary through workflow matrix variables rather than appearing as a literal `cargo test --lib FILTER` command. Supporting that form requires a broader command/data-flow model or execution evidence.

## Search result

A targeted public search found many ordinary examples of successful zero-test Cargo/libtest output and examples of filtered test runs, but no clean retained case yet where a literal GitHub Actions `cargo test --lib FILTER` became stale, selected zero tests, and stayed green.

That is an evidence boundary, not a reason to manufacture a showcase.

Issue #36 should remain open until a real positive case is recovered or a broader execution-aware experiment produces one.

## Synthetic end-to-end discriminator

PR #46 CI keeps one disposable repository fixture for the exact supported syntax family.

Fixture A:

```text
src/lib.rs:
  #[test]
  fn existing_test() {}

.github/workflows/test.yml:
  - run: cargo test --lib stale_filter
```

Expected JSON:

- one `ci-test-filter-inventory-miss` finding;
- claims include `proven`, `observed`, and `unknown`;
- question points back to `stale_filter`.

Fixture B changes only the workflow filter to `existing_test`.

Expected JSON:

- zero findings.

This proves the analyzer can distinguish its own positive and negative cells without claiming the synthetic fixture is external evidence.

### Self-dogfood finding

The first execution of this fixture failed for an analyzer reason rather than a product reason: the workflow scanner recognized bare `run:` lines and block-scalar contents, but missed the ordinary GitHub Actions list-item form:

```text
- run: cargo test --lib stale_filter
```

The stale-selector fixture therefore produced zero findings. The parser now strips an optional YAML list-item prefix before recognizing `run:`, and the list-item form has a unit control. This is retained because the fixture already demonstrated its second job: challenge the analyzer's own evidence collection before external promotion.

## Shared rendering integration

While this experiment was open, `main` changed repository and diff output so text and JSON render from one shared `AnalysisReport` model. PR #46 was reconciled onto that model as well: `ci-tests` now builds one provenance-bearing report and delegates both presentation formats to the shared renderer.

That is the desired direction for future analyzers. Evidence collection and claim classification belong to the analyzer; terminal prose and JSON are views over the same findings.

## Promotion direction

If this family keeps paying off, the stronger analyzer should move toward authoritative selection evidence:

```text
CI selector
-> resolved Cargo target/test binary
-> test listing or executed result count
-> repository expectation / precedent
```

That could cover direct test-binary invocation, wrappers, matrix variables, generated tests, and explicit expected-count defenses like exrs while preserving the current evidence taxonomy.

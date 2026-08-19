# CI test-selection listing replay

Date: 2026-08-19

Status: successful execution-aware follow-up to the merged syntax-only `ci-tests` analyzer.

## Question

Can the narrow supported CI selector family be verified against Cargo/libtest's own selected-test inventory without executing the selected tests?

## Safety boundary

This probe invokes Cargo. Compiling a test target may execute repository build scripts even though libtest receives `--list` and does not run selected tests.

For that reason this remains an explicit research/example path. Ordinary `cargo cultist ci-tests` stays read-only and reports runtime inventory as `UNKNOWN` when syntax cannot prove it.

## Exact Cargo Cultist receipt

Probe head:

```text
952e405d8c54d52e73a4a0e7614d4e936e763bd3
```

GitHub Actions:

```text
workflow: CI test selection listing research
run:      32216859850
job:      95959898941
result:   success
```

Generic CI on the same head also passed in run `32216859942`.

## Synthetic discriminator

The disposable Cargo repository contains one test:

```text
tests::existing_test
```

and a GitHub Actions selector:

```text
cargo test --lib stale_filter
```

### Ordinary filtered execution

Cargo compiled the library test target and exited successfully:

```text
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
```

This proves process success and useful test execution are separate facts.

### Listing verification

The execution-aware probe reconstructed the same target/filter and added libtest listing:

```text
.github/workflows/test.yml:7  filter `stale_filter`
workflow command: cargo test --lib stale_filter
listed selections: 0
```

Changing only the workflow filter to `existing_test` produced:

```text
filter `existing_test`
listed selections: 1
  tests::existing_test
```

So the same probe distinguishes an authoritative zero-selection cell from a positive one-test control.

## Real replay: Tantivy historical carrier

Pinned repository:

```text
teamleaderleo/tantivy@b92909ef3d5ac5695d1c85b1b0cb52a03ee51e49
```

The workflow contains two supported literal commands:

```text
cargo +1.88.0 test --lib test_prepare_ --locked --no-default-features
cargo +1.88.0 test --lib test_rollback --locked --no-default-features
```

The research run installed Rust 1.88.0 and regenerated the dependency lockfile under that recorded toolchain before asking libtest for selections.

The positive control resolved to exactly three tests:

```text
filter `test_prepare_`
listed selections: 3
  indexer::index_writer::tests::test_prepare_but_rollback
  indexer::index_writer::tests::test_prepare_with_commit_message
  indexer::segment_writer::tests::test_prepare_for_store
```

The stale rollback selector resolved to:

```text
filter `test_rollback`
listed selections: 0
```

This independently reproduces the historical execution receipt preserved by the merged syntax analyzer, where the same rollback command exited successfully with `0 passed` and 1120 filtered out.

## Evidence hierarchy

The two analyzers now have deliberately different authority:

```text
read-only syntax scan
  -> PROVEN literal selector exists
  -> OBSERVED no explicit source test/module hint
  -> UNKNOWN generated/runtime test inventory

explicit execution-aware listing
  -> Cargo resolves/builds selected library test target
  -> libtest enumerates exact matching tests
  -> zero or nonzero selection count is execution evidence
```

The stronger path carries a stronger side-effect boundary because Cargo compilation may run repository build scripts.

## Design consequence

A future opt-in verification mode can safely separate:

```text
selector process exited successfully
selector matched N tests
selected tests themselves passed/failed
```

Those are three different claims. The Tantivy case demonstrates why the first must never stand in for the second.

## Disposition

**Continue.** Keep the listing mechanism as explicit execution research until the user-facing opt-in contract, build-script warning, command reconstruction limits, and machine-readable provenance are designed together.

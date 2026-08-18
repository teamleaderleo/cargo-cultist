# cargo-cultist

**Find out why before you copy it.**

`cargo-cultist` is an experiment in repository-aware analysis for Rust codebases.

> Status: very early prototype. The current analyzer is deterministic, local, and read-only.

See [ROADMAP.md](ROADMAP.md) for the project thesis, design principles, and active research directions. The umbrella tracking issue is #19.

Traditional linters are strongest when a rule is already known. `cargo-cultist` starts one step earlier: it gathers facts about what a repository actually does, identifies deviations or unexplained patterns, and raises questions without pretending every observation is an error.

The long-term model is deliberately split into three layers:

- **Facts** — deterministic information extracted from source, Cargo metadata, and Git history.
- **Observations** — repository-specific patterns and deviations derived from those facts.
- **Explanations** — optional interpretation of why those patterns may exist. This layer may eventually use an LLM, but the tool should not require one to discover that something is interesting.

## Current checks

### Repository test-module conventions

The default command looks at `#[cfg(test)]` modules and reports the names a repository actually uses. It also calls out one-off names and files that mix multiple test-module names.

```text
$ cargo cultist

TEST MODULE CONVENTIONS
  unit_tests               24
  tests                    19
  special_tests             1

OBSERVATION
  `unit_tests` is the most frequent name (24 of 44 modules), but the repository uses 3 names.

ONE-OFF NAMES
  src/example.rs:120  mod special_tests

QUESTION
  Are these one-off names intentionally scoped, or accidental deviations from local precedent?
```

### Diff-aware precedent

`cargo cultist diff` looks only at test-module declarations added or renamed by the current change and compares them with both repository-wide and same-file precedent.

By default it compares staged and unstaged work against `HEAD`:

```bash
cargo cultist diff
```

For a branch or pull request, provide a base revision. `cargo-cultist` finds the merge base between that revision and `HEAD`, then compares the current working tree from that point so the result follows branch/PR semantics and still includes local staged or unstaged work:

```bash
cargo cultist diff --base origin/main
```

A finding can look like:

```text
FINDING 1: test-module precedent
  pci/src/vfio.rs:2675 adds `mod mmio_region_range_tests` behind a test cfg.

FACTS
  `mmio_region_range_tests` appears 1 time(s) across 53 test-gated modules.
  Repository counts: `unit_tests`=31, `tests`=21, `mmio_region_range_tests`=1.
  The same file also uses: `tests`.

OBSERVATION
  The new name differs from this file's existing precedent and is unique in the repository.

QUESTION
  Is the distinct module name intentional, or should it follow nearby precedent?
```

That distinction is the point: the primitive is a **finding**, not a lint error.

### Historical companions (experimental)

`cargo cultist history FILE` is research instrumentation for temporal precedent (#4) and negative-space associations (#7). It looks at recent non-merge commits touching one current file path and reports which other paths changed in the same considered commits.

```bash
cargo cultist history src/protocol.rs
cargo cultist history --max-commits 200 src/protocol.rs
cargo cultist history --format json src/protocol.rs
```

The first cohort filter removes revert commits and commits changing more than 100 paths. Output keeps support counts, representative examples, absence counterexamples, exclusions, and current limitations visible:

```text
HISTORICAL COMPANIONS
  anchor: src/protocol.rs
  cohort: 34 considered commit(s) from 37 discovered non-merge commit(s)

COMPANIONS
  generated/schema.json                                     31/34   91.2%
    example abc12345  2026-07-03T10:20:00Z  regenerate protocol clients
    example def45678  2026-06-12T08:05:00Z  add protocol field

COUNTEREXAMPLE SAMPLE
  generated/schema.json
    absent 789abcde  2026-05-01T14:10:00Z  refactor protocol comments

OBSERVATION
  These are historical co-change associations for the current path, before semantic cohort selection or finding thresholds.

QUESTION
  Which of these companions represent a repository custom worth comparing against a future diff?
```

This command intentionally reports association evidence before deciding which associations deserve findings. The next research step is to run it against real repositories where Fieldwork already recovered source/generated, implementation/test, and other companion relationships.

## Usage

From this repository while developing:

```bash
cargo run -- /path/to/a/rust/repository
cargo run -- diff --base origin/main /path/to/a/rust/repository
cargo run -- history /path/to/a/rust/repository/src/file.rs
```

After installing locally:

```bash
cargo install --path .
cd /path/to/a/rust/repository
cargo cultist
cargo cultist diff
cargo cultist history src/file.rs
```

You can also invoke the binary directly:

```bash
cargo-cultist /path/to/a/rust/repository
cargo-cultist diff --base origin/main /path/to/a/rust/repository
cargo-cultist history /path/to/a/rust/repository/src/file.rs
```

## Dogfooding

CI runs formatting, Clippy, and tests, then runs `cargo-cultist` against its own repository. Pull-request and push CI also run the diff analyzer against the relevant base commit.

The tool is expected to be able to inspect its own changes without special treatment.

## Near-term ideas

- Extend diff analysis beyond test-module names.
- Flag test-only global state and show nearby alternatives.
- Find duplicated local mechanisms when a common helper already exists.
- Connect unusual constants or workarounds to Git history.
- Separate output into **proven**, **derived**, **observed**, **inferred**, and **unknown** claims.
- Add optional explanations only after the deterministic evidence packet exists.

The goal is not to make Clippy fuzzier. The goal is to explore the space between a known lint rule and handing an entire repository to an AI and asking it to figure everything out.

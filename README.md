# cargo-cultist

**Find out why before you copy it.**

`cargo-cultist` is an experiment in repository-aware analysis for Rust codebases.

Traditional linters are strongest when a rule is already known. `cargo-cultist` starts one step earlier: it gathers facts about what a repository actually does, identifies deviations or unexplained patterns, and raises questions without pretending every observation is an error.

The long-term model is deliberately split into three layers:

- **Facts** — deterministic information extracted from source, Cargo metadata, and Git history.
- **Observations** — repository-specific patterns and deviations derived from those facts.
- **Explanations** — optional interpretation of why those patterns may exist. This layer may eventually use an LLM, but the tool should not require one to discover that something is interesting.

## First prototype

The first check looks at `#[cfg(test)]` modules and reports the names a repository actually uses. It also calls out one-off names and files that mix multiple test-module names.

Example:

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

That distinction is the point: the primitive is a **finding**, not a lint error.

## Usage

From this repository while developing:

```bash
cargo run -- /path/to/a/rust/repository
```

After installing locally:

```bash
cargo install --path .
cd /path/to/a/rust/repository
cargo cultist
```

You can also invoke the binary directly:

```bash
cargo-cultist /path/to/a/rust/repository
```

## Near-term ideas

- Compare a diff against nearby repository precedent.
- Flag test-only global state and show nearby alternatives.
- Find duplicated local mechanisms when a common helper already exists.
- Connect unusual constants or workarounds to Git history.
- Separate output into **proven**, **derived**, **observed**, **inferred**, and **unknown** claims.
- Add optional explanations only after the deterministic evidence packet exists.

The goal is not to make Clippy fuzzier. The goal is to explore the space between a known lint rule and handing an entire repository to an AI and asking it to figure everything out.

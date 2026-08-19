# cargo-cultist

**Find out why before you copy it.**

`cargo-cultist` is an experiment in repository-aware analysis for Rust codebases.

> Status: very early prototype. The public analyzer commands are deterministic, local, and read-only.

See [ROADMAP.md](ROADMAP.md) for the project thesis, design principles, and active research directions. The umbrella tracking issue is #19.

Traditional linters are strongest when a rule is already known. `cargo-cultist` starts one step earlier: it gathers facts about what a repository actually does, identifies deviations or unexplained relationships, searches for counterexamples, and raises questions without pretending every observation is an error.

The core evidence model distinguishes:

- **PROVEN** — exact machine facts or guarantees;
- **DERIVED** — deterministic conclusions from explicit facts;
- **OBSERVED** — empirical repository patterns and correlations;
- **INFERRED** — plausible interpretations;
- **UNKNOWN** — repository evidence is insufficient to recover the answer.

Human-readable and JSON output are rendered from the same provenance-bearing finding model where the command produces findings.

## Current commands

### Repository test-module conventions

The default command inspects `#[cfg(test)]` modules and reports the names a repository actually uses. It calls out one-off names and files that mix multiple test-module names without turning the majority spelling into a universal rule.

```bash
cargo cultist
cargo cultist --format json
```

The interesting primitive is a **finding**, not a lint error: repository statistics are evidence, and a local exception can be intentional.

### Diff-aware precedent

`cargo cultist diff` looks at test-module declarations added or renamed by the current change and compares them with existing repository-wide and file-local precedent.

```bash
cargo cultist diff
cargo cultist diff --base origin/main
cargo cultist diff --base origin/main --format json
```

With `--base REV`, Cargo Cultist finds the merge base between `REV` and `HEAD`, then analyzes the current working tree from that point so branch/PR semantics still include local staged or unstaged work.

When repository-wide and file-local precedent disagree, the analyzer surfaces **precedent tension** instead of silently choosing a winner. Changed-file parse failures remain explicit uncertainty instead of being converted into an observed absence claim.

### Historical companions

`cargo cultist history FILE` explores which paths repeatedly changed with one current file in recent non-merge history.

```bash
cargo cultist history src/protocol.rs
cargo cultist history --max-commits 200 src/protocol.rs
cargo cultist history --format json src/protocol.rs
```

The explorer:

- excludes revert commits and broad commits from its first comparison cohort;
- reports directional support/opportunity counts;
- preserves representative examples and absence counterexamples;
- annotates current companion files that explicitly identify themselves as generated;
- keeps limitations visible, including rename-history and semantic-cohort gaps.

Historical co-change remains correlation evidence. Real-repository replays have already shown why direction and cohort selection matter: in Oxc, Rust-syntax-changing edits to the source rule registry moved with two generated registries in 99/99 sampled commits, while the reverse generated-to-source relation was 94/100. See `research/history-companion-replay.md` and `research/rust-syntax-cohort-replay.md` for the retained receipts.

### CI test-filter inventory

`cargo cultist ci-tests` looks for a deliberately narrow GitHub Actions command family:

```text
cargo [ +TOOLCHAIN ] test --lib FILTER
```

and compares the literal selector with explicit Rust `#[test]` function names plus declared module names used as conservative qualifier hints.

```bash
cargo cultist ci-tests
cargo cultist ci-tests --format json
```

A zero syntax match is reported with separate claims:

```text
PROVEN
  The supported workflow command and filter exist.

OBSERVED
  The explicit source inventory has no matching test/module name.

UNKNOWN
  Macro-generated or build-time tests may exist outside the syntax inventory.
```

Unsupported shell forms, ambiguous package/integration/bin targets, unknown flags, and parse failures are skipped or surfaced conservatively instead of guessed through.

This check has a retained real-world witness: a pinned Tantivy workflow stayed green while `cargo test --lib test_rollback` selected zero tests, and Cargo Cultist identified that exact selector/location. The full acceptance receipt lives in `research/ci-test-filter-replay.md`.

## Research layer

The repository also contains standalone research examples and receipts that deliberately sit outside the public read-only command surface.

Current research includes:

- Rust-syntax cohorts for refining historical comparison sets;
- explicit generated-file and Rust generator-ownership evidence;
- execution-aware libtest `--list` verification of CI selectors;
- agentic-history corpora from Stensibly and SmolRunner;
- bounded repository-evidence packets for coding agents.

Some research examples intentionally execute repository tooling. In particular, Cargo/libtest listing can compile code and run build scripts. Those experiments carry an explicit effect boundary and are not silently invoked by the ordinary analyzer commands.

The research lifecycle is intentional:

```text
hypothesis
-> deterministic probe
-> real repository discriminator
-> counterexample / negative control
-> durable receipt
-> keep, weaken, split, reject, or promote
```

A successful experiment does not automatically become a lint or public feature.

## Usage

From this repository while developing:

```bash
cargo run -- /path/to/a/rust/repository
cargo run -- diff --base origin/main /path/to/a/rust/repository
cargo run -- history /path/to/a/rust/repository/src/file.rs
cargo run -- ci-tests /path/to/a/rust/repository
```

After installing locally:

```bash
cargo install --path .
cd /path/to/a/rust/repository
cargo cultist
cargo cultist diff
cargo cultist history src/file.rs
cargo cultist ci-tests
```

The binary can also be invoked directly:

```bash
cargo-cultist .
cargo-cultist diff --base origin/main .
cargo-cultist history src/file.rs
cargo-cultist ci-tests .
```

## Dogfooding

CI runs formatting, Clippy, and tests, then dogfoods the public analyzers and their JSON output against Cargo Cultist itself. Pull-request and push CI run diff analysis against the relevant base/current change.

The tool is expected to inspect its own changes without special treatment. Disposable fixtures and pinned external replays are used when a detector needs a stronger discriminator; temporary network-heavy research workflows are retired after their evidence is recorded.

## Active research directions

Near-term work is increasingly about composing independent evidence instead of adding broad heuristics:

- richer scoped and temporal precedent with explicit counterexamples;
- generated-artifact ownership and missing-companion questions;
- exception archaeology and expired-workaround evidence packets;
- helper/dependency intent and locally expanded idioms;
- explicit repository-guidance freshness and instruction lifecycle;
- bounded agent context packets that optimize selected evidence per byte instead of context volume;
- promotion of repeated, well-understood human consensus into deterministic policy.

Optional model-assisted explanation can sit on top of bounded evidence later. The deterministic finding must remain useful without a model.

The goal is to explore the space between a known lint rule and handing an entire repository to an AI and asking it to figure everything out.

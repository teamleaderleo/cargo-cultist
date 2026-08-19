# External repository dogfood

Cultist's larger evaluation corpus should live outside ordinary CI. The normal product loop stays small; external repositories are an on-demand fieldwork lane.

## Goal

Make another repository cheap to inspect before Cultist has any installation or configuration inside that repository.

The first harness supports:

```text
current Rust syntax/convention scan
current CI test-filter inventory
optional repeated scan for warm-cache measurement
optional bounded history for one explicitly named file
optional diff against one explicitly named base revision
```

Every Cultist invocation enables `CARGO_CULTIST_PERF=1` and preserves the JSON output, stderr, and performance receipt in one artifact directory.

## Safety boundary

The runner treats the target checkout as evidence.

It does not invoke target `cargo`, build scripts, tests, generators, package managers, or repository-provided commands. It writes Cultist caches and receipts outside the target checkout when the caller provides `--cache-dir` and `--output-dir` outside that checkout.

The GitHub workflow has read-only `contents` permission and is `workflow_dispatch` only. It therefore adds no network-heavy external corpus work to ordinary push or pull-request CI.

The first workflow is intended for public repositories and repositories the workflow token can read. A future private-repository adapter should use an explicit credential boundary instead of silently widening token authority.

## Progressive history cost

Repository history is a reservoir of evidence, not a mandatory startup cost.

The manual workflow defaults to a target checkout depth of 256 commits. A shallow checkout is recorded in `summary.json`; history results from that checkout should be read with that boundary in mind. Set checkout depth to `0` only when the replay actually needs complete history.

The history probe is additionally gated by:

- an explicitly supplied repository-relative file;
- a maximum commit count between 1 and 1000;
- Cultist's existing non-merge and broad-commit cohort rules.

This gives us a useful progression:

```text
new repository
  -> current scan + CI inventory

interesting target / current task
  -> bounded file history

specific change replay
  -> explicit base + diff

history question survives those gates
  -> deeper/full checkout deliberately
```

The aim is to learn from large histories without making every edit-loop invocation pay for them.

## Local use

Build Cultist once, then point the harness at any existing checkout:

```text
cargo build --release
python scripts/external_dogfood.py \
  --cultist target/release/cargo-cultist \
  --repo /path/to/other/repo \
  --output-dir /tmp/cultist-dogfood \
  --cache-dir /tmp/cultist-cache \
  --repeat-scan
```

Add one bounded history target when there is a reason:

```text
python scripts/external_dogfood.py \
  --cultist target/release/cargo-cultist \
  --repo /path/to/other/repo \
  --output-dir /tmp/cultist-dogfood \
  --history-file src/example.rs \
  --history-max 100
```

A specific change can add `--base REV` and run the existing diff analyzer against the checked-out target.

## First external carrier: SmolRunner

Issue #62 already established a useful pinned SmolRunner history replay at:

```text
teamleaderleo/smolrunner@ed3b70e375a57eabce26f2311f798f75b33bdeb0
src/disposable_clone_runtime.rs
```

That target is a good first carrier because it has known earned-history discriminators and counterexamples. The generic manual workflow defaults to the living `main` branch so it can also reveal drift; an exact SHA can be supplied whenever a reproducible historical replay is desired.

## Corpus direction

This harness is the execution primitive for #16 rather than a fixed list of repositories. Candidate cases already include:

- Cultist self-dogfood for quiet controls;
- SmolRunner for earned local history and agent-context work;
- Cloud Hypervisor for the canonical repository-vs-file precedent tension;
- Stensibly for longitudinal agentic churn and handoff/recovery questions;
- Linux Fieldwork for real bug-species controls.

The next useful layer is a small, reviewed manifest of pinned cases and expected questions. The harness should stay generic; corpus policy decides which cases deserve deeper history.

## Receipts

Each run writes:

- `summary.json` — exact target HEAD, shallow/full-history boundary, probe list, counts, and performance receipts;
- `<probe>.json` — raw machine report;
- `<probe>.stderr.txt` — non-performance stderr;
- the GitHub job summary — a compact table of findings and work units.

Findings do not make a dogfood run fail. Process failures, malformed JSON, or a missing/invalid performance receipt do, because those make the evaluation itself unreliable.

Refs #16 #48 #49 #62.

# Fieldwork

Fieldwork is Cargo Cultist's replay corpus for real repository cases.

The goal is not to maximize the number of findings. The goal is to preserve a small set of changes where we already understand what useful evidence should look like, then catch regressions toward noisy, overconfident, or semantically wrong output.

## Why pinned external cases?

Synthetic fixtures are useful for parser/unit behavior, but several Cultist ideas only become meaningful at repository scale:

- repository-wide vs file-local precedent;
- historical co-change cohorts;
- directional companion relationships;
- counterexamples that explain why a relationship is conditional.

Each Fieldwork case therefore pins:

- a public repository;
- an exact commit;
- any extra revision needed by the command;
- a Cargo Cultist JSON command;
- **semantic expectations** over the JSON result.

The harness does not compare golden terminal output. Wording can improve without breaking the corpus.

## Current cases

### Cloud Hypervisor PR #8734

Replays the test-module naming disagreement that motivated scope-aware precedent:

- repository-wide precedent favors `unit_tests`;
- existing `pci/src/vfio.rs` precedent favors `tests`;
- the changed declaration uses `unit_tests`;
- Cultist should report a `test-module-precedent-tension` finding and preserve both scopes.

### Oxc rule registry -> generated registries

Replays the raw historical-companion result from `research/history-companion-replay.md`:

- `crates/oxc_linter/src/rules.rs`
- generated `rule_runner_impls.rs`: at least 99/100
- generated `rules_enum.rs`: at least 99/100

### Oxc generated registry -> source registry

Runs the reverse query and preserves the weaker relationship:

- generated `rules_enum.rs` -> source `rules.rs`: exactly 94/100 in the pinned window.

Keeping the forward 99/100 and reverse 94/100 cases side by side protects the design result that historical relationships are directional.

## Running

Fieldwork requires network access, Git, Python 3, and a Rust toolchain.

Run every case:

```bash
python scripts/run-fieldwork.py
```

Run selected cases:

```bash
python scripts/run-fieldwork.py \
  --case cloud-hypervisor-8734-precedent-tension \
  --case oxc-rules-generated-companions
```

List cases without building or cloning:

```bash
python scripts/run-fieldwork.py --list
```

Use an existing binary:

```bash
python scripts/run-fieldwork.py --binary /path/to/cargo-cultist
```

Use a persistent clone directory for investigation:

```bash
python scripts/run-fieldwork.py --workdir /tmp/cultist-fieldwork
```

## CI policy

Normal CI should validate that the harness and case file are syntactically healthy, but it should not require network-heavy external replays on every PR.

A separate manually triggered workflow can run the pinned corpus when we change analyzer semantics or want an explicit release/research check.

## Adding a case

Prefer a real case with a known review/research outcome. Assertions should describe durable semantic evidence:

- analysis kind;
- finding kind;
- required evidence substring;
- companion path with support/opportunity bounds.

Avoid freezing presentation details or asserting a large opaque score.

A good case should answer: **what regression would this catch that a unit fixture would miss?**

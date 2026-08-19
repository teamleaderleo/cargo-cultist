# Counterexample-driven cohort refinement

Tracking: #142.

## Why this starts above edit classification

Cultist already earned the first concrete cohort-refinement discriminator before #142 existed.

The retained Oxc replay in `research/rust-syntax-cohort-replay.md` shows:

```text
source rules.rs -> generated registries
raw focused cohort:       99/100
Rust syntax cohort:        99/99
excluded docs-only edit:     1
```

The sole raw counterexample was the known license-notice/docs change. The reverse direction stayed:

```text
generated rules_enum.rs -> source rules.rs
Rust syntax cohort: 94/100
```

All 100 reverse anchor edits changed Rust syntax, so the same discriminator correctly produced no improvement there.

`src/generated_diff.rs` now carries equivalent source-change classification for product generated-companion evidence, and `examples/rust_syntax_cohort.rs` retains the generic historical probe. Reimplementing Rust edit classification under #142 would duplicate an earned fact producer.

This carrier therefore starts one layer later:

> Given observations already labeled with deterministic facts, which admitted discriminator creates a useful narrower cohort for the current change?

## V0 input

```text
current_facts
observations[]
  id
  support | counterexample
  facts {key -> value}

admitted discriminators[]
min_support
```

The evaluator does not infer facts from prose, paths, commit subjects, or names. An analyzer/research adapter owns those facts.

## Output

Every discriminator preserves:

- the original baseline support/counterexample counts;
- every partition, including observations missing that fact;
- current-change discriminator value when supplied;
- exact counts in the current value cohort;
- exact support/counterexamples excluded from the current cohort.

No purity/confidence score is produced.

V0 statuses:

```text
candidate
  current cohort excludes at least one counterexample and retains min_support

overfit
  counterexamples are excluded but the current cohort has too little positive support

no_improvement
  the current cohort excludes no counterexample

unknown_current
  the current change lacks the discriminator or its value has no observed cohort

incomplete_evidence
  one or more counterexamples lack the discriminator fact, so exclusion cannot be treated as an explained exception
```

These statuses describe the supplied cohort experiment. `candidate` remains OBSERVED evidence; it grants no project policy.

## First controls

### Oxc forward result

Synthetic observations reproduce the retained executed counts:

```text
99 support       edit_class=syntax_changed
1 counterexample edit_class=comments_or_docs_only
current           edit_class=syntax_changed
```

Expected:

```text
baseline       99 support / 1 counterexample
current cohort 99 support / 0 counterexamples
status         candidate
```

### Oxc reverse control

```text
94 support + 6 counterexamples
all edit_class=syntax_changed
```

Expected: `no_improvement` with the current cohort identical to baseline.

### Identity-like overfit control

Four observations carry unique `commit` values. Selecting the current commit isolates one positive observation and one counterexample elsewhere.

With `min_support=3`, expected: `overfit`.

This rejects the easiest fixture-memorization path without hard-coding words such as `commit`, `path`, or `sha` as forbidden facts.

### Missing current membership

If the current change lacks the admitted fact, expected: `unknown_current`.

### Missing counterexample classification

If a counterexample lacks the admitted fact, expected: `incomplete_evidence` even when the current bucket otherwise appears pure.

This prevents missing classification from masquerading as explained counterevidence.

## Boundary

- fact production remains analyzer/language-specific;
- chronology never becomes a fact unless an independent producer establishes a temporal relation;
- candidate refinement remains empirical cohort evidence;
- the baseline is always retained;
- excluded support remains visible alongside excluded counterexamples;
- min-support is an explicit research parameter, not a universal product threshold;
- no model is required;
- no product CLI/report-schema change.

## Next discriminator

Run this evaluator over observations emitted from a real history adapter instead of synthetic count-preserving controls. The first candidate is the already-pinned Oxc source registry replay.

Then test a second, independent discriminator family where useful counterexamples are split by package/scope or generated/authored status. Promotion is earned only if the generic evaluator improves more than one evidence family without turning fact production into a universal classifier.

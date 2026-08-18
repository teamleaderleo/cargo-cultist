# Roadmap

`cargo-cultist` explores repository reasoning between two familiar extremes:

- deterministic tools that enforce rules we already know; and
- unconstrained AI review that is asked to infer everything from a codebase.

The project goal is to make the middle useful.

## Core loop

```text
deterministic facts
  -> scoped observations
  -> counterexample search
  -> questions worth asking
  -> optional explanation
  -> human decision
  -> preserved rationale
  -> promote stable consensus into deterministic policy
```

The primitive is a **finding**, not an error. A good finding says what the repository evidence shows, what it does not establish, and why a human may want to look.

## Principles

### Ask questions before inventing rules

Repository statistics are evidence, not policy. If a repository uses `unit_tests` 89 times and `tests` 33 times, that does not automatically make `unit_tests` correct everywhere.

### Keep scope visible

Precedent can differ at the same time across a file, crate, repository, and recent history. When those scopes disagree, Cultist should show the tension instead of hiding it in one score.

Tracking issue: #3.

### Search counterexamples first

Before emitting a precedent-based finding, look for exceptions and ask whether those exceptions share a reason that applies to the changed code.

Tracking issue: #6.

### Preserve provenance

Cultist should distinguish:

- **PROVEN** — exact machine facts or guarantees;
- **DERIVED** — deterministic conclusions from explicit facts;
- **OBSERVED** — empirical repository patterns;
- **INFERRED** — plausible interpretations;
- **UNKNOWN** — evidence is insufficient to recover the reason.

Tracking issue: #15.

### `UNKNOWN` is useful

If nobody can recover why an important-looking workaround, constant, suppression, or special case exists, that is useful knowledge debt. Cultist should say so instead of fabricating intent.

### Keep the core local and deterministic

Model-assisted explanations may eventually help interpret evidence, but the tool must remain useful without an LLM. A model should receive a bounded evidence packet and should not be responsible for discovering the underlying facts.

Tracking issue: #17.

### Teach the project why

When humans decide an exception is intentional, the reason should be preservable in version control so future analysis can distinguish a known exception from forgotten folklore.

Tracking issue: #10.

### Promote mature questions into rules

If the same fuzzy review question repeatedly receives the same human answer, it may be ready to become a deterministic lint or project rule. Cultist should help incubate that transition while preserving the rationale.

Tracking issue: #11.

## Workstreams

### 1. Precedent engine

The first implementation already compares changed test-module declarations with repository-wide and same-file precedent. The next step is to make precedent richer without pretending popularity is correctness.

- #3 — scope-aware precedent and precedent tension
- #4 — temporal precedent and convention drift
- #5 — convention entropy and first-exception risk
- #6 — counterexample-first findings
- #7 — negative-space associations
- #20 — locally expanded idioms that duplicate helpers
- #21 — package and dependency intent

### 2. Archaeology

Repositories accumulate historical reasons that disappear from the final source code. Cultist should recover those reasons when possible and expose when they have been lost.

- #8 — exception archaeology
- #9 — historical fossils and expired workarounds
- #12 — `why` mode and evidence packets
- #18 — Git, PRs, issues, and reviews as project memory

### 3. Institutional memory and policy

Questions become more valuable when their human answers can persist and eventually become enforceable policy.

- #10 — explicit decision records / `teach`
- #11 — lint incubation and promotion
- #15 — claim provenance model

### 4. Engine, interfaces, and evaluation

The underlying engine should support more analyzers without turning into a pile of ad hoc scans or opaque scores.

- #13 — local evidence index
- #14 — progressive semantic adapters
- #16 — dogfood corpus and precision-focused evaluation
- #17 — optional bounded LLM explanations
- #22 — stable machine-readable findings / JSON

## Near-term sequence

A useful order for the next few experiments is:

1. **Claim provenance + machine-readable findings** (#15, #22)
2. **Scoped precedent** (#3)
3. **Counterexample-first output** (#6)
4. **Dogfood corpus** (#16)
5. **Temporal precedent** (#4)
6. **Helper/dependency precedent** (#20, #21)
7. **History and exception archaeology** (#8, #9, #12)

This is not a commitment to build everything in order. Small experiments that falsify an idea are valuable. The project should prefer evidence that a feature is useful over completing a grand architecture.

## Canonical dogfood case

Cloud Hypervisor PR #8734 is currently the clearest example of the intended behavior.

The repository-wide evidence favored `unit_tests`, while the changed `vfio.rs` file already contained `tests`. Cultist surfaced both facts and asked which scope should govern the change. During that dogfood run, the tool also exposed a bug in its own diff semantics, leading to a merge-base-aware fix.

That is the standard to aim for: the tool should be willing to find ambiguity in the repository, ambiguity in its own conclusions, and bugs in itself.

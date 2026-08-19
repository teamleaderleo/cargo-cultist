# Roadmap

Cultist explores repository reasoning between two familiar extremes:

- deterministic tools that enforce rules we already know; and
- unconstrained AI review that is asked to infer everything from a codebase.

The project goal is to make the middle useful.

Rust currently provides the deepest semantic adapter and the `cargo-cultist` distribution, but Cultist's evidence model is repository-oriented. Git history, provenance, decision memory, and concurrent-change coordination can apply independently of source language; language-specific adapters should add claims only where their evidence justifies them.

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

There is a second loop for Cultist's own development:

```text
agent/human works on Cultist
  -> task exposes friction, a false assumption, duplication, missing context, or a counterexample
  -> preserve exact evidence
  -> ask whether Cultist could have surfaced or retained it earlier
  -> test the generalization against negative controls
  -> improve analyzer / evidence contract / regression / decision memory when earned
  -> future worker starts with a better repository
```

The primitive is a **finding**, not an error. A good finding says what the repository evidence shows, what it does not establish, and why a human may want to look.

## Principles

### Ask questions before inventing rules

Repository statistics are evidence, not policy. If a repository uses `unit_tests` 89 times and `tests` 33 times, that does not automatically make `unit_tests` correct everywhere.

### Keep scope visible

Precedent can differ at the same time across a file, package/crate, repository, and recent history. When those scopes disagree, Cultist should show the tension instead of hiding it in one score.

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

### Learn from the work itself

Cultist development is part of the evaluation corpus. If implementing or reviewing a change forces a worker to manually discover an important repository fact, coordinate duplicate work late, recover lost rationale, correct a stale assumption, or weaken a heuristic after finding a counterexample, treat that episode as candidate product evidence.

Do not automatically widen the current task. Record the exact evidence and use the smallest appropriate durable surface: a regression, research receipt, decision record, roadmap note, or focused follow-up. Generalize only after a discriminator and negative control justify it.

Repository agent guidance: `AGENTS.md`.

### Promote mature questions into rules

If the same fuzzy review question repeatedly receives the same human answer, it may be ready to become a deterministic lint or project rule. Cultist should help incubate that transition while preserving the rationale.

Tracking issue: #11.

## Workstreams

### 1. Precedent engine

The first implementation already compares changed Rust test-module declarations with repository-wide and same-file precedent. The next step is to make precedent richer without pretending popularity is correctness.

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
- #62 / #74 / #75 — bounded agent context and longitudinal decision memory

### 4. Coordination and concurrent work

Cultist should help changes understand other active work without pretending that path overlap is the whole problem.

- #96 — direct concurrent-change preflight baseline
- #99 — real agent-heavy coordination corpus
- #101 — offline active-change inventories and explicit coordination edges

Direct shared paths are `PROVEN` from Git. Deeper generated, historical, policy, behavioral, or intent relationships remain separate evidence layers with their own provenance and counterexamples.

### 5. Engine, interfaces, adapters, and evaluation

The underlying engine should support more analyzers without turning into a pile of ad hoc scans or opaque scores.

- #13 — local evidence index
- #14 — progressive semantic adapters
- #16 — dogfood corpus and precision-focused evaluation
- #17 — optional bounded LLM explanations
- #22 — stable machine-readable findings / JSON

The Rust adapter is the first deep source adapter. Add TypeScript, Python, or other language support when a concrete analyzer family and real corpus justify the parser/semantic investment; keep Git/repository evidence reusable across languages.

## Near-term sequence

A useful order for the next few experiments is:

1. keep claim provenance + machine-readable findings stable (#15, #22)
2. dogfood direct preflight and explicit coordination evidence (#96, #99, #101)
3. continue bounded agent context + decision memory (#62, #74, #75)
4. deepen scoped/counterexample/temporal precedent (#3, #6, #4)
5. continue generated/helper/dependency precedent (#20, #21 and related research)
6. expand history and exception archaeology (#8, #9, #12)
7. add language adapters only when a held-out discriminator earns them

This is not a commitment to build everything in order. Small experiments that falsify an idea are valuable. The project should prefer evidence that a feature is useful over completing a grand architecture.

## Canonical dogfood cases

Cloud Hypervisor PR #8734 remains a clear precedent example: repository-wide evidence favored `unit_tests`, while the changed `vfio.rs` file already contained `tests`. Cultist surfaced both facts and asked which scope should govern the change. During that dogfood run, the tool also exposed a bug in its own diff semantics, leading to a merge-base-aware fix.

Stensibly supplies a complementary coordination corpus: parallel agents, stale evidence, explicit policy changes, duplicate lanes, handoffs, and reviewed lessons that later workers need to recover without old chat history. The #1624/#1625/#1629 overlap is an immediate example of why preflight coordination evidence matters.

That is the standard to aim for: Cultist should be willing to find ambiguity in a repository, ambiguity in its own conclusions, and weaknesses in its own development workflow — then preserve the lesson with enough provenance that a future worker does not have to rediscover it manually.

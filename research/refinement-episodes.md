# Counterexample-guided analyzer refinement episodes

Tracking: #148. Built after #141/#144, #142/#168, project-memory contract refinement #174, and the behavioral episode work in #137/#175/#178.

## Question

Can Cultist preserve analyzer-research transitions as bounded, replayable records while leaving domain facts and predicates with the analyzers that produced them?

V0 answers only the bookkeeping layer.

## Episode contract

```text
RefinementEpisode
  id
  family
  hypothesis_before
  counterexample_refs[]
  admitted_discriminators[]
  candidate_refinements[]
  selected_transition?
  source_receipts[]
  behavioral_episode_ids[]
```

Each candidate carries:

```text
hypothesis_after
admitted discriminator refs
source replay receipts
replay_result
  expected_cases_retained
  counterexamples_resolved
  expected_cases_lost
  counterexamples_remaining
  held_out_status
status
  retained
  weakened
  split
  rejected_no_improvement
  rejected_overfit
  rejected_lost_expected_case
```

The replay counts are supplied by exact source receipts. This layer does zero Rust edit classification, obligation applicability, historical co-change analysis, project-memory relation parsing, or domain predicate evaluation.

## Meta-level invariants

The validator checks only properties the refinement ledger itself owns:

- episode, candidate, discriminator, and reference identities are bounded and unique where required;
- every candidate discriminator reference names one discriminator admitted by the episode;
- a selected transition names an existing `retained`, `weakened`, or `split` candidate;
- kept candidates retain every expected replay case, resolve at least one counterexample, and cannot carry a failed held-out result;
- `rejected_no_improvement` resolves zero counterexamples and loses zero expected cases;
- `rejected_lost_expected_case` records at least one lost expected case;
- rejected-overfit status stays a supplied source conclusion because overfit criteria belong to the source experiment;
- behavioral episode IDs remain optional explicit links to already-observed #165 episodes.

The last point composes with #175: a research transition can link to observed receiver episodes and descriptive summaries while deterministic replay remains independently inspectable.

## Retained episode A: justification / open obligation

The first episode records the refinement discovered by stacking #144 on #141.

```text
H0
  every obligation requires >= 1 observed clearing edge

counterexample
  a durable material UNKNOWN can exist before clearing evidence arrives

selected transition
  allow zero-edge OPEN obligations
  keep typed clearing receipts when evidence later arrives
```

Source receipts point at the #156/#159 research heads and CI. The meta replay records expected cases retained, the counterexample resolved, and zero expected cases lost.

No behavioral episode is attached because this was a deterministic semantic refinement rather than an observed receiver episode.

## Retained episode B: Oxc edit-class cohort

The second episode records the already-executed Oxc result:

```text
raw forward cohort
  99 support / 1 counterexample

edit_class = syntax_changed
  99 support / 0 counterexamples
```

The selected `edit_class` refinement is `weakened` because it narrows the original file-change hypothesis.

Two rejected candidates remain beside it:

```text
reverse edit-class control
  94 support / 6 counterexamples
  -> rejected_no_improvement

exact commit identity partition
  one-current-observation cohort
  -> rejected_overfit
```

The source receipts preserve the full Rust syntax cohort replay, #168's generic refinement evaluator, its green CI run, and the earlier held-out generated-companion product proof.

## Retained episode C: project-memory Primary case contract

The third episode came from a real producer/consumer collision between merged #166 and #167.

```text
H0
  current project-memory relation admission composes with collector evidence

#166 consumer
  strict single-line relation admission + exact target mention

#167 producer
  Primary case:\nURL issue evidence block

counterexample
  both landed with zero path overlap
  current-main packet no longer passed the consumer contract
```

#174 split the admission rule instead of weakening every relation edge:

```text
ordinary relationship prose
  -> existing strict single-line classifier

exact Primary case block
  -> separate validated path
     exact label
     admitted GitHub origin
     packet repository match
     canonical positive issue number
     relation = related
     declared target match
```

The source replay admits the two intended Primary case forms, resolves the integration counterexample, loses zero expected cases, and passed #174 CI/provenance on head `e1196c553ab496df13a230535de997f30c2d63ca`.

This episode also carries the real #178 behavioral episode:

```text
project-memory:primary-case-contract-collision:9792bfe->df5ae59
```

The standard refinement test resolves that ID against the retained #165 behavioral batch and requires its observed outcome to be `changed_next_action`.

## Why rejected candidates stay in the object

Keeping only the winner would erase the research path. The durable episode instead records:

```text
what broad claim existed
which counterexample challenged it
which discriminators were admitted
which candidate survived
which tempting candidates failed
which exact receipts justify those statuses
```

That gives a future worker enough information to resume the research question without reconstructing the discarded alternatives from prose or chronology.

## Behavioral boundary

`behavioral_episode_ids[]` is an explicit optional join to merged #165 observation identity.

Episodes A and B stay empty because no observed worker episode has been collected for those exact research transitions. Episode C links one already-retained #178 observation and verifies that the ID exists in the behavioral corpus.

A deterministic replay result never manufactures a behavioral receipt. Behavioral identity and deterministic refinement identity remain separate records joined by explicit ID only when an observation exists.

## Research reader

```text
cargo run --example refinement_episodes \
  < research/refinement-episodes/cultist-v1.json
```

The reader validates the bounded batch and reprints the typed records.

## Executed GitHub receipts

The first two-family version of draft PR #179 passed on exact head:

```text
f4d92b05e061a99e262a1ef1eb2f2be424686359
```

CI run `32247329835` / run number `1222` and generated provenance review run `32247329697` / run number `219` completed successfully. The first CI attempt had failed only at rustfmt before Clippy or tests; the formatter delta was applied verbatim and the standalone reader received fixture-local dead-code containment.

The three-family carrier was then rebuilt as one commit on merged #178 main so its behavioral join resolves against the current retained observation corpus.

Exact three-family semantic head:

```text
dbb39fb729df762ffefcf97d024e52fd647832cf
```

GitHub Actions CI run `32247721374` / run number `1235` completed successfully. The job passed:

- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- active-work preflight;
- full `cargo test`, including the three retained refinement episodes, rejected-candidate controls, and the cross-corpus #178 behavioral ID/outcome assertion;
- repository text/JSON dogfood;
- history text/JSON dogfood;
- CI test-filter inventory text/JSON plus positive/control fixtures;
- pull-request diff text/JSON dogfood.

Generated provenance review dogfood run `32247721348` / run number `223` also completed successfully on the same three-family head.

## Boundary

- research-only ledger;
- no analyzer predicate language;
- no automatic discriminator enumeration;
- no scalar research score;
- no automatic promotion/demotion;
- no chronology-derived causality;
- no fabricated behavioral evidence;
- exact source receipts remain the authority for domain replay claims.

## Next discriminator

Three independent families now fit the ledger, but they expose different fact-production mechanisms:

```text
justification
  typed clearing-evidence presence

historical companion
  supplied Rust edit class

project memory
  exact evidence-form + target/repository contract
```

That diversity argues for one more restraint: episode count alone does not earn automatic candidate enumeration. The next useful question is whether these source analyzers can expose a common **discriminator reference interface** without moving their domain logic into #148.

A small future experiment can ask each source family for only:

```text
discriminator id
source receipt
current supplied value / applicability
```

and then test whether the refinement ledger can enumerate candidates over those references while still delegating semantics to the source evaluator. If the adapters need family-specific execution logic, keep enumeration outside the meta-layer.

North star:

> Preserve every analyzer refinement as an inspectable transition from broad hypothesis through counterexample and replay, including the rejected alternatives that explain why the surviving rule earned its narrower claim.

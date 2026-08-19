# Agent work and coordination retrospective — 2026-08-19

## Status and boundary

**Status:** research / coordination evidence  
**Opened:** 2026-08-19  
**Repository baseline reviewed:** `main@36f76188a1ef9a7c923caa30c6e49673cf52f040`, plus live issue/PR state sampled afterward  
**Scope:** how Cultist research and multi-agent repository work have been organized; lessons from Stensibly and Preflight are used only as external/public dogfood evidence where explicitly referenced  
**Authority:** none. This document does not approve a feature, merge a hypothesis into the canonical evidence model, or change agent/project policy.

This note asks a different question from the ordinary Cultist roadmap:

> **What has the way we are doing the work taught us about how repository/agent research should be coordinated?**

Cultist's product thesis already says that repository work should expose evidence, counterexamples, uncertainty, applicability, provenance, and durable lessons. Development of Cultist itself is now rich enough to evaluate the *research process* with the same standards.

The note deliberately separates:

- repository-visible facts;
- interpretation;
- open questions;
- proposed experiments.

It does not infer private worker intent from chronology.

---

# Executive conclusion

The current Cultist approach has worked well for **earning semantics before productizing them**.

The strongest pattern has been:

```text
large idea
-> one bounded research question
-> explicit negative control
-> research-only implementation
-> exact observed discriminator
-> narrow product promotion or explicit non-promotion
```

Recent examples are especially clean:

- direct concurrent-change collision -> provider-supplied active-work inventory -> explicit coordination edges -> bounded metadata extractor -> dogfood integration;
- lossless C1 representation -> fail-closed schema/version/bounds -> terse projection -> evidence-role counterexample research;
- report-local terse refs -> proof that stale positional deltas are unsafe -> exact report fingerprint research rather than prematurely inventing semantic lineage;
- exact-coordinate applicability -> immediate follow-up when empty requirements accidentally meant “globally applies”.

This is good research behavior. The project repeatedly lets one experiment *disprove an overly simple next step* before that step becomes a public contract.

The main coordination risk is now the opposite of Stensibly's. Stensibly has too many live-looking implementation artifacts. Cultist has **too many plausible research directions** relative to the number of synthesis/evaluation checkpoints that decide what deserves the next layer of product investment.

The next improvement should not be more process ceremony. It should be a clearer research frontier:

```text
NOW
  exact questions with live discriminators

NEXT
  questions unlocked by current results

PARKED
  attractive hypotheses lacking a current discriminator/corpus need

RETIRED / WEAKENED
  ideas falsified, absorbed, or deliberately not promoted
```

The repository already has most of the conceptual discipline required. The missing piece is making *research convergence* as visible as research generation.

---

# What went well

## 1. Cultist treats a research issue as a question, not a disguised implementation order

Many current issues have unusually good epistemic form. They state:

- thesis/question;
- candidate semantics;
- exact counterexample or positive control;
- boundaries/non-goals;
- success criteria;
- follow-ups that are explicitly conditional on the first experiment earning them.

That changes worker behavior in a productive way. A fresh worker is invited to falsify the idea rather than “finish the ticket” by implementing the prose literally.

Issue #125 is representative. It does not say “add four evidence-role enum values.” It asks whether the current untyped model can collapse two states that require different next actions, then proposes a tiny provisional role vocabulary purely as a discriminator.

Issue #127 likewise does not say “build delta transport.” It first asks what identity contract prevents a delta from applying to the wrong state and uses the simplest reorder counterexample to prove positional refs unsafe.

This is exactly the kind of issue-writing that reduces cargo-cult implementation.

## 2. Negative controls are a first-class deliverable

Cultist's strongest research habit is requiring evidence *against* the tempting conclusion.

Examples include:

- explicit coordination phrase extraction has ordinary `Refs`, `Related`, `Parent`, and ambiguous prose as quiet negatives;
- historical companionship preserves counterexamples instead of turning co-change frequency into a required companion rule;
- terse projection research asks for pairs with the same visible claim but different decision-relevant evidence;
- delta identity research proves the same positional delta bytes can mean two different things after reorder;
- applicability semantics distinguish known mismatch from missing context and now reject empty/underspecified requirements rather than treating absence as global validity.

This has two coordination benefits:

1. different workers can challenge a hypothesis using the same executable discriminator rather than debating prose;
2. a failed experiment can still finish cleanly with a useful boundary.

That is much healthier than research work whose only accepted outcome is “feature shipped.”

## 3. The project has resisted turning every observed pattern into policy

This is one of Cultist's most important identity-preserving decisions.

The roadmap repeatedly keeps distinct:

```text
observed frequency
explicit repository guidance
authoritative reviewed decision
current applicability
finding disposition
```

Historical companion frequency remains `OBSERVED` association evidence. Provider-supplied coordination edges say what the admitted artifact records rather than claiming universal scheduling truth. Model explanations are optional and do not become project facts. A terse `HOLD`-like projection does not grant merge authority.

This discipline prevents the project's own vocabulary from becoming a new source of cargo culting.

## 4. Distinct research views have been kept conceptually separate

The current docs explicitly distinguish:

- lifecycle: **when** evidence is recovered/preserved;
- JEI: **what** evidence is selected;
- review intelligence: **where** scarce attention goes;
- representation/IR: **how** selected evidence is transmitted;
- decision memory: **what reviewed rationale survives**.

This is a valuable coordination rule. Without it, different workers could independently create:

```text
brief status
review status
IR status
memory status
```

for the same underlying provenance/applicability/unknown concepts.

The docs' instruction to compose projections over shared evidence instead of inventing competing truths is worth retaining even if the exact current schemas change.

## 5. The active-work/preflight line advanced through earned layers

The progression around #96 is a strong model for research/product sequencing:

1. #97 proved deterministic direct-path overlap.
2. #98 dogfooded a bounded provider-supplied active-work inventory.
3. #100 made the advisory cheap and visible rather than expanding its semantics first.
4. #103 promoted explicit coordination edges into product preflight.
5. #105/#120 researched one deliberately narrow source-side metadata phrase.
6. #122 composed that extraction into the live heads-up only after the narrower semantics existed.

This avoids an attractive but dangerous shortcut:

```text
fetch every PR body
-> ask model/regex what depends on what
-> treat output as scheduler truth
```

Instead, every new edge class has to earn a high-precision source contract.

## 6. Research-only surfaces have reduced accidental contract commitment

Cultist often uses examples, integration fixtures, research modules, or test-local types before changing the main CLI/schema.

That is useful because many current questions are genuinely unsettled:

- whether evidence roles belong in canonical `Evidence` or a higher-level envelope;
- whether semantic lineage needs analyzer-owned stable IDs;
- what a compact IR vocabulary should contain;
- whether unpublished branches are useful active-work evidence;
- how much context-relative compression remains interoperable across models.

Research-only code gives workers something executable to evaluate without silently turning a hypothesis into compatibility debt.

## 7. The project is increasingly good at distinguishing exact identity jobs

Recent work is converging on an important decomposition:

- exact report snapshot fingerprint;
- report-local object reference;
- exact object content identity;
- semantic lineage identity;
- evidence applicability to repository/revision/work/scope;
- source authority;
- disposition.

#127/#128 explicitly warn that content identity is not semantic lineage. #123/#124 keep applicability separate from epistemic strength and authority. #131 proposes a C1-derived exact report fingerprint while explicitly refusing to claim that similar findings are the same lineage.

This is excellent foundational discipline for both agent communication and coordination.

## 8. The project responds quickly when a newly landed abstraction exposes a vacuous success case

The current #130 follow-up is a good example. Once shared applicability landed, a producer could accidentally provide no requirements and receive `applies` because zero dimensions disagreed.

Rather than rationalizing that behavior as “global evidence,” the follow-up recognizes producer omission as an ambiguity and makes an intentionally global scope something that would need an explicit marker later.

This is a strong general habit:

> When absence accidentally becomes permission/truth/applicability, prefer an explicit positive marker over treating missing fields as a broad grant.

## 9. Performance is being treated as an evidence property, not deferred indefinitely

The performance work is not complete, but the project has already introduced opt-in work counters and history batching while richer analyzers are still being designed.

That is timely. Repository reasoning tools are especially vulnerable to an architecture where every new analyzer adds:

```text
one filesystem walk
+ one full parse per file
+ one Git history traversal
```

The current performance roadmap correctly focuses on work units—files parsed, Git subprocesses, cache hits—not only noisy wall-clock time.

The new external-dogfood harness in #129 is also a useful coordination step: instead of every research worker inventing a bespoke workflow to replay another repo, there is a path toward one bounded reusable carrier.

## 10. The repository is willing to preserve research evidence while retiring the mechanism that produced it

Several decision-memory experiments explicitly remove temporary workflows after the executed receipt is captured. #117 superseded stale docs work rather than preserving obsolete product descriptions as active truth. Branch heads-up work was retained as research while default product behavior stayed conservative.

This is a good instinct: **the evidence should survive; experimental scaffolding does not need permanent authority or runtime presence.**

## 11. Current open implementation/research WIP is comparatively controlled

At the 2026-08-19 sample after #128 merged, the live open PR set was small and coherent:

- #126 — evidence-role discriminator against the actual terse renderer;
- #129 — reusable external dogfood/performance harness;
- #130 — bounded repair to applicability semantics;
- #131 — exact report snapshot fingerprint research.

These are separate enough to coexist, and each names its non-goals. That is a good level of parallelism for a research-heavy repository.

## 12. Cultist is dogfooding coordination on real agent-heavy repositories rather than only synthetic examples

The project has public corpora in Preflight, SmolRunner, Stensibly, Linux Fieldwork, and other external repositories. This matters because coordination semantics are easy to make plausible with invented examples.

The strongest current examples are exactly the cases where simple heuristics fail:

- Preflight #748/#703: zero path overlap but explicit merge/evidence sequencing;
- Stensibly history: an optimization later contributes to a control regression;
- SmolRunner: a sequence of follow-up fixes whose final code alone does not explain why apparently redundant checks were removed or retained;
- branch lifecycle research: divergence alone is not enough evidence that a branch represents active intent.

This is the right kind of corpus pressure.

---

# What could have gone better

## 1. The issue space is much larger than the currently actionable research frontier

Cultist's umbrella/roadmap is intellectually coherent, but a fresh worker sees dozens of attractive open issues:

- precedent;
- counterexample mining;
- archaeology;
- decision memory;
- project-memory adapters;
- JEI;
- review intelligence;
- compact IR;
- deltas;
- applicability;
- evidence roles;
- active-work coordination;
- semantic adapters;
- evidence indexing;
- multiple performance workstreams;
- many bug-species analyzer hypotheses.

Many are correctly exploratory, but GitHub's ordinary open-issue state does not say:

```text
current discriminator worth funding now
vs
next after current result
vs
parked idea
vs
broad roadmap placeholder
```

The roadmap has a near-term sequence, yet a new agent still has to read a large conceptual graph before knowing the *current small research frontier*.

**Improvement:** maintain a tiny generated/curated research-frontier section with perhaps 3–5 live questions, each including:

- current evidence;
- exact missing discriminator;
- active experiment/PR, if any;
- promote / weaken / retire condition;
- what question becomes eligible next.

Open issues can remain the full idea library without all presenting as equally actionable.

## 2. Research artifacts are accumulating faster than their synthesis/index

The `research/` directory contains many valuable receipts: agent-context packets, active-work dogfood, decision-memory replays, branch lifecycle work, explicit coordination metadata, CI test-filter replays, and more.

The retention is a strength. Discoverability is becoming the weakness.

A fresh worker can easily know that *somewhere* in `research/` there is probably an executed receipt relevant to the current question, but finding the strongest/current one may require directory archaeology.

**Improvement:** add a lightweight research index grouped by semantic question, with fields like:

```text
question
current conclusion
strongest positive control
strongest negative control
status: active | weakened | promoted | retired
canonical receipt
superseded receipts
```

Do not duplicate the content of every note. The purpose is to make prior research selection cheap.

## 3. Temporary workflow scaffolding has sometimes become a large fraction of the history

Cultist has repeatedly used temporary replay workflows to get executable evidence, followed by commits to record receipts and remove/retire the workflow. This is legitimate, especially when GitHub Actions is the only available environment for an experiment.

The downside is a history full of:

```text
add experiment
add temporary workflow
run it
record replay
retire workflow
merge
```

For a small research repo, this can make Git history describe CI plumbing almost as much as the research conclusion.

The new #129 external dogfood harness is a good response. Continue consolidating experiment execution into a small number of reusable manual/research harnesses rather than building a bespoke workflow per hypothesis.

## 4. A few direct-main hygiene mistakes show the cost of very high iteration speed

Recent history around the first preflight implementation includes stray placeholder cleanup and an `oops` commit before the clean feature merge path. These are minor and transparently repaired.

They are still useful process evidence: when research velocity is high, direct iterative publication can make the durable history noisier than necessary.

This does not justify heavyweight branch ceremony for every experiment. It suggests a smaller rule:

> For source changes intended to become durable product/research evidence, stage the complete bounded packet before publishing the final mainline commit when practical; reserve direct-main scratch churn for clearly disposable contexts.

## 5. The project sometimes creates a new research issue immediately after every discriminator

Cultist's strength is decomposition. Its possible failure mode is *recursive decomposition without synthesis*.

A result can lead to:

```text
new distinction
-> new issue
-> new fixture
-> new missing distinction
-> new issue
```

This is healthy while each step materially reduces ambiguity. It becomes research debt when several sibling issues all describe parts of one model but no synthesis checkpoint asks what the smallest shared model now is.

A useful trigger:

> After 2–3 successful experiments introduce adjacent semantic primitives, pause new decomposition long enough to write the composition test and try to delete/reject redundant concepts.

The current identity/applicability/evidence-role line is approaching such a checkpoint.

## 6. The project needs stronger promotion and retirement states for hypotheses

An open issue can mean:

- active product work;
- active research;
- partially proven thesis;
- intentionally deferred feature family;
- useful idea with no current discriminator;
- parent roadmap that should remain open for years.

Cultist's prose often explains this well, but machine-visible lifecycle is weak.

A research hypothesis should ideally have an explicit outcome such as:

```text
PROMOTED
  discriminator earned a product primitive

WEAKENED
  useful only under narrower conditions

RETAINED_RESEARCH
  effect observed but product value unproved

REJECTED
  counterexample defeats the proposed generalization

SUPERSEDED
  another primitive explains it better

PARKED
  interesting but no current evaluation priority
```

This does not require a new product schema. A roadmap/research index convention would already help.

## 7. The dogfood/evaluation corpus has lagged the rate of semantic invention

Issue #16 has been open for a while, and the project has meanwhile generated many sophisticated research directions.

The new #129 external dogfood harness is therefore important: it can turn held-out evaluation into ordinary practice rather than a bespoke event.

The core concern is overfitting. A semantic distinction may look compelling because it perfectly explains the episode that motivated it.

Before product promotion, require at least one of:

- a second independent repository/case showing the same discriminator;
- a held-out negative where the tempting heuristic stays quiet;
- a direct before/after worker experiment showing the evidence changes useful behavior.

The exact requirement should depend on the claim, but “one motivating case + one synthetic test” is often not enough for repository-wide reasoning semantics.

## 8. Stensibly is an exceptionally rich corpus and therefore a dangerous corpus to overfit

Cultist issue #41 correctly treats Stensibly as a seed, not a specification.

Stensibly has unusually explicit language around:

- exact heads;
- source authority;
- review gates;
- red controls;
- current-main replay;
- durable provider receipts;
- worker callsigns;
- merge holds;
- recovery generations.

An extractor trained only on this world could become excellent at understanding *teamleaderleo agent-repo dialect* while providing little value elsewhere.

For every coordination semantic promoted from Stensibly/Preflight evidence, ask:

- does another repository express the same concept differently?
- is the semantic relation project-authored and explicit, or are we exploiting local vocabulary?
- does a negative corpus show that similar words can mean something weaker elsewhere?

## 9. “Same next action” is a useful compression discriminator but not a complete product oracle

#125/#126 sensibly use modeled next action to prove that omitted evidence can matter. That is a strong negative-control method.

The danger would be making “same next action” the universal definition of semantic equivalence.

A caller's action can depend on facts outside `AnalysisReport`:

- authority;
- operator preference;
- consequence/risk;
- external dependency state;
- task goal;
- available tools.

Therefore the safer conclusion is:

> If two canonical reports can require different next actions under a fixed explicit evaluation oracle, a projection that collapses them is insufficient for that use. The converse—same modeled action means semantically equivalent—is not established.

Keep the oracle deliberately local to each compression experiment until a real disposition model is earned.

## 10. Compact representation research can tempt premature semantic unification

The IR work is exciting because many current objects repeat concepts such as identity, scope, evidence, unknown, applicability, transition, omission, and invalidation.

But a compact language can accidentally force unrelated semantics into one token because compactness rewards reuse.

The sequencing should remain:

```text
semantic distinction proven independently
-> shared model composition tested
-> compact encoding
```

not:

```text
short token vocabulary invented
-> repository concepts bent to fit it
```

C1's current role as lossless encoding of the existing `AnalysisReport` is a good conservative base.

## 11. Provider-prose extraction should remain unusually hard to promote

Explicit coordination metadata has high value, but natural-language project metadata is a dangerous authority surface.

The current extractor discipline—one reviewed phrase form, exact source identity, negative phrases, no arbitrary intent inference—is correct.

Keep requiring:

- concrete source object;
- exact work/head/freshness coordinate;
- reviewed phrase grammar;
- endpoint resolution;
- explicit ambiguity/unknown outside the clause;
- no scheduling/mutation authority granted by extraction alone.

A future model-assisted extractor may be useful as an `INFERRED` research aid, but it should not silently produce the same typed edge class as deterministic reviewed source syntax.

## 12. The project could use more explicit “do less” decisions

Cultist has many great ideas. A mature research tool should also accumulate a list of things it deliberately decided *not* to do:

- branches are not scanned as active intent by default because divergence is insufficient;
- history correlation does not become policy;
- model explanation is not required;
- report-local refs are not durable lineage;
- unknown future machine-report fields fail closed rather than down-convert silently;
- absence of applicability requirements should not mean global applicability.

These negative decisions are part of the product identity. Make them easy to discover so future agents do not reopen them merely because the rejected implementation is tempting.

---

# Coordination lessons from the comparison with Stensibly

Cultist and Stensibly are now useful mirrors because they have opposite dominant failure modes.

## Stensibly's lesson for Cultist: every research lane needs a durable next action and convergence condition

Stensibly's strongest product property is continuation. A work item should survive the chat with:

- outcome;
- current responsibility;
- exact evidence;
- blockers;
- next executable action;
- wake/completion condition.

Cultist research issues often have excellent success criteria but can be weaker on **current continuation state** once several experiments have run.

A research frontier should therefore answer:

```text
What did the latest experiment establish?
What exact question remains?
What observation would close/weaken/promote it?
Who/what lane is currently active, if any?
```

## Cultist's lesson for Stensibly: coordination policy should be falsifiable

Stensibly can create rich process machinery quickly. Cultist's discipline asks:

- what exact failure motivated this rule?
- what counterexample would show the rule is too broad?
- what evidence says it should be product-enforced rather than guidance?
- when should the rule be weakened or retired?

That is a useful test for every future Stensibly coordination primitive.

## Do not merge the product identities

A promising composition is:

```text
Cultist
  repository evidence / precedent / counterexamples / applicability / UNKNOWN
       |
       v
Stensibly
  responsibility / authority / leases / execution / settlement / recovery
       |
       v
Cultist
  evaluate outcome / preserve earned repository lesson
```

Cultist should not decide that a worker may merge/deploy/contact someone merely because repository evidence looks favorable.

Stensibly should not encode Cultist's research hypotheses as workflow authority merely because they are typed.

---

# Questions worth answering next

## Research frontier and work selection

1. **What are Cultist's 3–5 current highest-value research questions right now?**
2. Can every open research issue be classified as `now`, `next`, `parked`, `promoted`, `weakened`, or `retired` without reading its whole history?
3. What is the minimum evidence required before a worker should open a new sibling research issue instead of extending/synthesizing the current one?
4. When do three adjacent research primitives require a synthesis checkpoint before further decomposition?
5. Should research WIP remain capped around 2–4 live PRs, with extra workers doing held-out evaluation/review rather than spawning more implementations?

## Promotion

6. What exact conditions turn a research-only module/example into a product CLI/schema primitive?
7. Should product promotion normally require a held-out repository or only when the claim generalizes beyond exact machine semantics?
8. Which current research results are mature enough that keeping them “research” now creates more confusion than safety?
9. Which popular-looking roadmap ideas have actually failed to earn promotion and should be marked parked/rejected?

## Evidence model composition

10. How should exact snapshot identity (#127/#131), applicability (#123/#124/#130), evidence role (#125/#126), provenance, and semantic lineage compose without becoming one overloaded identifier/status record?
11. Is semantic lineage needed globally, or should only analyzers that can prove a stable key emit lineage?
12. Should intentionally global applicability be an explicit scope marker, and what would its authority semantics be?
13. Does evidence role belong in canonical `Evidence`, in a selected envelope, or in producer-specific typed relations?
14. Which semantic primitives must be representable in C1 before a broader compact IR can claim useful round-trip coverage?

## Lossy projections and JEI

15. What properties must a lossy projection preserve besides a modeled next action?
16. How should a projection expose “expand before consequential action” without itself becoming an authority engine?
17. Can omission receipts be compact enough to be useful while still preventing absence from being interpreted as negative evidence?
18. When is support evidence safely omittable, and when does provenance/debuggability justify retaining one representative support receipt?
19. How do we measure the cost of a projection that is technically correct but forces too many expansions?
20. What is the right evaluation metric: action correctness, time to first useful inspection, total bytes after expansions, interruption count, or a vector rather than one score?

## Review intelligence

21. Can review-attention selection prove useful on held-out PRs without simply mirroring diff size or known historical failures?
22. What mechanical concerns can be removed from human/model review because deterministic Cultist evidence already answers them?
23. How should a review envelope respond to head movement—full invalidation, per-evidence applicability reevaluation, or something in between?
24. Can resolved review comments become future evidence without treating every reviewer statement as authoritative precedent?

## Coordination evidence

25. Which explicit coordination edge types have enough cross-repository evidence to promote beyond the current narrow `hold_merge_while` style?
26. How should `depends_on`, `blocks`, `supersedes`, and “current head invalidates evidence” differ semantically?
27. Can a provider adapter safely distinguish operative instructions from examples/quotes/old prose without a model?
28. Should a model-assisted edge extractor produce only `INFERRED` candidate edges for human/deterministic confirmation?
29. Can Stensibly supply a stronger active-work inventory—work identity, outcome, responsibility, wake condition—while Cultist independently evaluates repository overlap/applicability?
30. Can Cultist's active-work preflight measurably reduce duplicate Stensibly/Preflight lanes before implementation starts?

## Research evidence storage

31. Should `research/` gain a canonical index with current conclusion/status, or would that simply create another stale tracker?
32. Can the index be partly generated from front matter in research receipts to reduce maintenance?
33. When should an older research receipt be marked superseded versus remain an independent positive/negative control?
34. What is the retention policy for temporary workflow execution receipts after the relevant semantic result is encoded in tests/product?
35. Which negative results deserve top-level discoverability because they prevent tempting regressions?

## Evaluation

36. How many independent repositories/cases should a repository-semantic feature see before promotion?
37. Which hypotheses can be proven entirely from exact machine semantics and therefore need less corpus breadth?
38. Can #129 become the one ordinary external dogfood carrier for most future research, reducing one-off workflow churn?
39. What are the first five held-out cases that should become canonical evaluation corpus entries?
40. Can we replay fresh-agent tasks with and without JEI/preflight evidence and measure actual investigation behavior rather than only renderer output?
41. How do we avoid evaluating Cultist primarily on repositories whose issue/PR language was already shaped by Cultist/Stensibly concepts?

## Performance

42. At what point does richer evidence selection require the local evidence index (#13/#44), based on actual repeated work rather than architectural elegance?
43. Which current commands still perform repository-scale work for no-op/irrelevant queries?
44. Can work counters become an acceptance gate before new analyzers add another scan/history pass?
45. How much cache complexity is justified before measurements show warm analysis cost is a real user problem?
46. Can shared fact extraction stay storage-agnostic long enough to avoid prematurely committing to SQLite/schema maintenance?

## Human and agent consumers

47. Which Cultist views are actually different for humans versus agents, and which can share one projection with different rendering density?
48. Do agents benefit more from terse symbolic form than from small structured JSON once tool-call/schema overhead is included?
49. What debugging cost appears when a different model consumes compact/context-relative forms?
50. How should the system expose unsupported/unknown semantics so a weaker consumer fails safely rather than guessing?

## Authority boundary

51. Are any current output words (`HOLD`, `NEXT`, `PASS`, etc.) likely to be mistaken for effect authorization by an orchestrator?
52. Should machine output explicitly encode `authorizes_effect: false` for some envelopes, or is that adding redundant process semantics to a repository evidence tool?
53. Where should operator/project authority live when Cultist is composed with Stensibly?
54. How do we ensure a reviewed decision record is evidence of project intent without automatically suppressing future findings outside its exact applicability?

---

# Experiments worth running

## Experiment A: research-frontier index

Add a small current frontier—not the whole roadmap—with no more than a handful of rows:

```text
question
current strongest evidence
active PR/experiment
missing discriminator
promotion/retirement condition
next unlocked question
```

Try it for one week of research churn.

Measure:

- how often workers open a redundant/sibling issue;
- time from fresh chat to useful work selection;
- stale rows;
- whether the index causes premature narrowing.

Retire it if it becomes another manually maintained stale scoreboard.

## Experiment B: research receipt index generated from front matter

Add minimal machine-readable metadata to new research receipts:

```text
question-id
status
canonical-positive
canonical-negative
supersedes
promoted-to
```

Generate a human index. Do not backfill every historical note initially.

The experiment succeeds only if discovery gets cheaper without making receipt authoring burdensome.

## Experiment C: synthesis gate after adjacent primitives

Use the current cluster:

- exact report identity;
- applicability;
- evidence role;
- terse projection.

Before adding another adjacent semantic field, construct one composite real case and ask:

- which distinctions are actually orthogonal?
- which can be derived?
- which are view-specific?
- which produce contradictory states?
- what does C1 need to preserve?

The desired output may be a smaller model, not a new abstraction.

## Experiment D: held-out coordination extraction

Take the existing explicit-coordination extractor and evaluate it on:

- Preflight/Stensibly positives;
- ordinary open-source PR bodies using `depends`, `blocked`, `parent`, `related`, and examples in weaker senses;
- quoted instructions;
- old/superseded PR prose.

Measure false typed edges, not only recall.

Promotion of additional edge classes should require a meaningful quiet-negative corpus.

## Experiment E: Stensibly duplicate-lane preflight

Before a new Stensibly implementation dispatch:

1. obtain bounded active-work inventory;
2. use Cultist direct overlap + explicit edges;
3. compare cited issue/outcome identities where available;
4. return evidence only.

Stensibly decides whether to join, review, compete, or proceed.

Measure duplicate PR families and operator reconciliation work before/after.

## Experiment F: A/B fresh-worker JEI test

Select a real task with:

- one important earned invariant;
- one tempting wrong precedent;
- one material unknown.

Give fresh workers:

A. ordinary repository access;
B. a Cultist JEI packet.

Record:

- first files/evidence inspected;
- wrong turns;
- expansions;
- time/steps to bounded solution;
- whether the unknown is respected;
- whether the packet causes anchoring on an incorrect prior conclusion.

This tests product value, not just encoding compactness.

## Experiment G: one external harness, many corpora

Complete #129 and then resist creating repository-specific workflows for ordinary research replay unless a capability is genuinely missing.

Use it to build a small stable corpus matrix with exact refs and bounded receipts.

The harness should make external research cheaper without making network-heavy corpus execution part of ordinary contributor CI.

## Experiment H: negative-result catalogue

Create a short list of decisions such as:

- branch divergence != active intent;
- history correlation != policy;
- positional ref != durable identity;
- missing applicability requirements != global applicability;
- model prose != project authority;
- context compression must not erase decision-changing evidence.

Each entry links the exact experiment/counterexample.

Use the catalogue to stop future agents from reopening already-falsified easy designs unless they bring a new discriminator.

---

# Suggested lightweight coordination practice for Cultist

This is a research recommendation, not an instruction change.

## Keep

- no more than a few live implementation/research PRs at once;
- one bounded question per experimental PR where possible;
- negative controls and explicit non-goals;
- research-only surfaces before compatibility/schema commitments;
- current-main rebuilds rather than dragging stale carrier history forever;
- exact source/corpus coordinates;
- explicit `UNKNOWN` and applicability boundaries;
- performance work counters during feature growth;
- one research result can legitimately be “do not build this.”

## Add

For each live research lane, make these easy to recover:

```text
QUESTION
CURRENT EVIDENCE
WHAT WOULD DISPROVE / WEAKEN IT
WHAT WOULD PROMOTE IT
NEXT EXECUTABLE EXPERIMENT
```

For each completed lane, select one outcome:

```text
PROMOTED
WEAKENED
RETAINED RESEARCH
REJECTED
SUPERSEDED
PARKED
```

Do this in existing issue/receipt metadata before inventing a product-level research workflow.

## Avoid

- opening a sibling issue merely because a new noun appeared;
- keeping temporary workflows/carriers as fake active work after evidence capture;
- maintaining multiple prose trackers with exact live state;
- promoting a semantic primitive from one motivating repository dialect;
- treating a terse renderer's action oracle as general authority;
- forcing unrelated evidence concepts into a compact IR for token economy;
- building a large evidence database before current work counters justify it;
- making every dogfood observation into a feature request.

---

# What Stensibly and Cultist can test together

The two repositories now form a particularly useful closed-loop experiment if the boundary remains clean.

## Proposed loop

```text
1. Stensibly has a durable work item / desired outcome.
2. Cultist compiles bounded repository evidence:
     guidance
     active work
     explicit coordination
     precedent/counterexamples
     applicability/unknowns
3. Stensibly uses that evidence to inform dispatch/review,
   while retaining all authority/lease/effect decisions itself.
4. Worker executes and Stensibly records exact settlement/recovery evidence.
5. Cultist evaluates whether the episode exposed a reusable repository lesson.
6. Only reviewed/earned lessons become future evidence or deterministic policy.
```

## Questions for the joint experiment

- Does Cultist preflight reduce accidental duplicate Stensibly PRs?
- Does a Stensibly work item provide better task context than Cultist currently has to infer from PR/issue metadata?
- Can Cultist's applicability evaluator help Stensibly know when a repository evidence packet went stale without making Cultist the authority for the work?
- Can Stensibly's durable outcome/settlement data supply stronger evaluation labels for Cultist than PR chronology alone?
- Can the operator see fewer coordination interruptions without hiding important `UNKNOWN`s?
- Does the combined system remain useful when either product is unavailable?

The last question is important. Git/GitHub and ordinary repository state should remain independently understandable. Neither project should create an ecosystem where the other is required merely to decode what happened.

---

# Evidence reviewed

Repository-visible evidence for this note includes:

- `AGENTS.md`;
- `ROADMAP.md` and umbrella issue #19;
- current research/product issues including #62, #67, #74, #96, #101, #105, #106, #109, #115, #119, #123, #125, and #127;
- current/recent PRs around active-work preflight, C1, terse projection, applicability, evidence-role research, exact report fingerprinting, and external dogfood;
- recent main history through #128 at `36f76188a1ef9a7c923caa30c6e49673cf52f040`;
- current `research/` receipt inventory;
- public Stensibly coordination evidence summarized in Cultist issue #41;
- public Preflight coordination cases already used by #99/#101/#105.

Not available / deliberately excluded:

- private chat transcripts;
- private evaluation corpus contents;
- unrecorded operator rationale;
- causal claims that cannot be established from explicit repository evidence.

---

# Current conclusion

Cultist does **not** currently need a heavier agent operating protocol.

Its research work benefits from the opposite: keep individual experiments small and falsifiable, but add one better convergence surface so fresh workers know which questions are alive and which prior experiments have already ruled out the easy answer.

The most valuable coordination improvement is therefore:

> **Make the research frontier and research conclusions easier to recover; do not make research execution more bureaucratic.**

That keeps the thing Cultist is already doing well: using repository evidence to make the next worker need less inherited conversation, fewer assumptions, and fewer repeated mistakes.

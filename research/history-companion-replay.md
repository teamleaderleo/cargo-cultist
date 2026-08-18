# Historical companion replay: Oxc and Codex

Date: 2026-08-19

Status: research result for the experimental `cargo cultist history` command. This note records evidence and design consequences; it does not promote a co-change percentage into a correctness rule.

## Question

Can raw file-level Git co-change recover repository relationships that Fieldwork previously had to learn during source investigation?

The first replay deliberately used repositories where prior Fieldwork work already identified meaningful companion surfaces:

- Oxc linter rule registration and generated outputs;
- Codex app-server protocol source, generated schemas, runtime handlers, tests, and documentation.

## Exact inputs

Cargo Cultist experiment head used for the extended replay:

- `teamleaderleo/cargo-cultist@387d81d9f641b645247e49cfaae5ef327baccae8`

Target pins:

- `oxc-project/oxc@8783524015b1e6ff1c39ccf426df0bb07cbbc588`
- `openai/codex@785ecd7452f87c7eb731fbb73892185cbdd9d5f9`

Workflow:

- run `32174897832`
- job `95834419834`
- result: success
- artifact `9338659185`
- artifact digest `sha256:69c2afeccb0de9a084204810c1ccf36847a982e47c99d4fa41b1a0a46f6c6115`

The workflow cloned target history read-only with blob filtering and ran the local Cargo Cultist binary. No external repository was mutated.

## Current experiment semantics

For one current file path, the explorer:

1. asks Git for the most recent non-merge commits touching that path;
2. excludes revert subjects;
3. excludes commits changing more than 100 paths;
4. counts every other path appearing in the remaining commits;
5. reports support, opportunities, examples, and absence counterexamples;
6. keeps co-change explicitly classified as association evidence.

Rename following, semantic change classification, subsystem-aware cohorts, and finding thresholds remain future work.

---

## Oxc: source registry to generated registries

Anchor:

`crates/oxc_linter/src/rules.rs`

Cohort:

- 100 discovered non-merge commits;
- 100 considered commits;
- 0 excluded by the first-pass filters.

Top companions:

| Companion | Support |
|---|---:|
| `crates/oxc_linter/src/generated/rule_runner_impls.rs` | 99/100 (99.0%) |
| `crates/oxc_linter/src/generated/rules_enum.rs` | 99/100 (99.0%) |
| `apps/oxlint/src-js/package/config.generated.ts` | 59/100 (59.0%) |
| `npm/oxlint/configuration_schema.json` | 59/100 (59.0%) |
| `tasks/website_linter/src/snapshots/schema_json.snap` | 37/100 (37.0%) |
| `tasks/track_linter_timings/linter_timings.snap` | 10/100 (10.0%) |

The sole absence counterexample for both 99% generated companions was:

`docs(linter): add license notices for ported ESLint plugins (#22768)`

This is an unusually clean result. Raw history independently recovered the source/generated relationship that Fieldwork had already identified from Oxc's repository conventions.

### Immediate lesson

A naive future finding could already ask a useful question when a semantic change to `rules.rs` leaves both generated registries untouched.

The remaining 1% shows why the comparison cohort belongs in the evidence packet. A documentation/comment-only change can touch the source registry without requiring regeneration.

A stronger detector therefore wants to distinguish source-semantic changes from documentation-only edits before it promotes absence into a finding.

---

## Oxc: generated registry back to source registry

Anchor:

`crates/oxc_linter/src/generated/rules_enum.rs`

Cohort:

- 100 discovered commits;
- 100 considered commits.

Top companions:

| Companion | Support |
|---|---:|
| `crates/oxc_linter/src/generated/rule_runner_impls.rs` | 94/100 (94.0%) |
| `crates/oxc_linter/src/rules.rs` | 94/100 (94.0%) |
| `apps/oxlint/src-js/package/config.generated.ts` | 59/100 (59.0%) |
| `npm/oxlint/configuration_schema.json` | 59/100 (59.0%) |

Representative absence counterexamples for `rules.rs` include:

- `perf(linter): shrink rule serialization dispatch (#25818)`;
- `perf(linter): use table for rule names (#25458)`;
- `perf(linter): reduce rule config dispatch size (#25461)`.

### Major design result: relationships are directional

These two queries are empirically different:

```text
P(generated registry changes | source registry changes) = 99%
P(source registry changes | generated registry changes) = 94%
```

That difference is meaningful. Generated machinery can receive implementation/performance work without a source-rule registration change. A source-rule registration change almost always produces generated registry changes.

Cargo Cultist therefore needs to distinguish at least:

- **symmetric association** — A and B tend to travel together;
- **directed expectation** — when A changes in cohort C, B usually changes;
- **generation/derivation** — B is explicitly produced from A or from facts reachable through A.

A missing-companion detector should reason in the changed direction instead of treating co-change as an undirected edge.

This is one of the clearest results from the first corpus replay.

---

## Oxc: one repository operation can have several companion tiers

The Oxc result also shows several layers of coupling:

- 99%: core generated Rust registries;
- 59%: exposed configuration/schema outputs;
- 37%: website schema snapshot;
- 10%: linter timing snapshot.

These percentages should not become one threshold.

They likely reflect different change subclasses. For example, some rule-registry changes alter public configuration output while others do not. Timing work may be required to execute as a gate without producing a committed timing diff every time.

This gives Cargo Cultist another important distinction:

> "Repository procedure usually runs command X" and "file Y usually changes" are different facts.

Historical file co-change can discover candidate relationships. Explicit workflow/scripts/docs can establish execution requirements that Git alone cannot infer from changed files.

---

## Codex: broad shared protocol source

Anchor:

`codex-rs/app-server-protocol/src/protocol/common.rs`

Cohort:

- 100 discovered non-merge commits;
- 98 considered;
- 2 excluded: one broad 107-path commit and one revert.

Top companions:

| Companion | Support |
|---|---:|
| `codex-rs/app-server/README.md` | 78/98 (79.6%) |
| aggregate JSON schema | 68/98 (69.4%) |
| v2 aggregate JSON schema | 67/98 (68.4%) |
| `ClientRequest.json` | 45/98 (45.9%) |
| `app-server/src/message_processor.rs` | 37/98 (37.8%) |
| TypeScript v2 index | 34/98 (34.7%) |
| v2 protocol tests | 27/98 (27.6%) |
| core protocol source | 25/98 (25.5%) |
| v2 thread source | 24/98 (24.5%) |

Raw history clearly surfaces the generated schema family, but it also surfaces documentation and runtime consumers.

### Interpretation

`common.rs` is a broad anchor. Some edits affect exported protocol types, some affect internal helpers or behavior, and some require runtime/documentation movement without regenerating every schema artifact.

This is useful noise: it demonstrates why a repository-level detector needs **semantic cohorts** after file-level discovery.

A promising refinement is to identify which changed Rust items participate in schema export. The cohort can then ask:

```text
When exported protocol item X changes, which generated artifacts move?
```

instead of:

```text
When common.rs changes at all, which files move?
```

The first query should have much higher specificity.

---

## Codex: narrower v2 thread protocol

Anchor:

`codex-rs/app-server-protocol/src/protocol/v2/thread.rs`

Cohort:

- 53 discovered commits;
- 51 considered;
- 2 excluded.

Top companions:

| Companion | Support |
|---|---:|
| `app-server/src/request_processors/thread_processor.rs` | 41/51 (80.4%) |
| `app-server/README.md` | 39/51 (76.5%) |
| aggregate JSON schema | 34/51 (66.7%) |
| v2 aggregate JSON schema | 34/51 (66.7%) |
| `common.rs` | 29/51 (56.9%) |
| v2 protocol tests | 25/51 (49.0%) |
| `ClientRequest.json` | 23/51 (45.1%) |
| request processor registry | 23/51 (45.1%) |
| TUI app-server session | 20/51 (39.2%) |
| `ServerNotification.json` | 19/51 (37.3%) |
| TypeScript v2 index | 17/51 (33.3%) |
| v2 thread-resume integration test | 17/51 (33.3%) |

This is especially interesting because the strongest companion is the runtime owner for the same domain: thread protocol changes travel with `thread_processor.rs` more often than with any single generated artifact.

### Another design result: companion families are typed

A single source surface may have several meaningful relationships:

```text
thread protocol -> handled by -> thread processor
thread protocol -> documented by -> app-server README
thread protocol -> exported into -> aggregate JSON schemas
thread protocol -> exercised by -> protocol and integration tests
thread protocol -> consumed by -> TUI app-server session
```

The history explorer discovers candidates. Semantic adapters can later classify the relationship type.

The tool should resist collapsing these into one generic "files that often change together" list once a relation becomes understood.

---

## Codex exporter anchor

Anchor:

`codex-rs/app-server-protocol/src/export.rs`

Cohort:

- 52 discovered commits;
- 45 considered;
- 7 broad commits excluded.

Top companions:

| Companion | Support |
|---|---:|
| `protocol/common.rs` | 23/45 (51.1%) |
| `app-server/README.md` | 18/45 (40.0%) |
| aggregate JSON schema | 17/45 (37.8%) |
| message processor | 13/45 (28.9%) |
| `ClientRequest.json` | 10/45 (22.2%) |
| v2 aggregate JSON schema | 10/45 (22.2%) |
| legacy v2 protocol source | 10/45 (22.2%) |

This is a useful reverse-direction control. The exporter implementation itself can change for reasons that do not regenerate every schema file. Once again, generation relationships are directional and conditional.

---

## What the first replay establishes

### 1. File-level co-change has real signal

Oxc produced a 99/100 source-to-generated relationship using only Git path history. That is enough to justify continuing the experiment.

### 2. Direction belongs in the relation model

`P(B|A)` and `P(A|B)` differ in meaningful ways. A future relation index should preserve direction.

### 3. Counterexamples are part of the product

The Oxc 1% counterexample immediately explained why the raw relationship should avoid universal-rule language. In Codex, counterexamples expose distinct subclasses of protocol changes.

### 4. Cohort selection is the next central research problem

The useful question is increasingly:

```text
Given changed fact A of semantic class C in subsystem S and era E,
what companion relationship usually holds?
```

Raw file paths provide the first candidate graph. AST facts, generator metadata, workspace ownership, test declarations, and history eras can refine the cohort.

### 5. Explicit and empirical evidence can reinforce each other

For Oxc, Fieldwork had already recorded that generated registration/timing outputs belong to repository generator commands. Git independently recovered the dominant generated registry pairing.

That suggests a strong evidence composition:

```text
explicit generator ownership
+ historical directional support
+ changed source fact
+ missing generated companion
= high-quality finding packet
```

### 6. Procedure and changed-file precedent must stay separate

The Oxc timing snapshot moved in only 10/100 source-registry commits even though repository work may still require running timing tooling. Cargo Cultist should model "command/gate usually required" separately from "file usually changes."

### 7. Relation types can emerge from candidate edges

The first corpus already suggests:

- `generates` / `derived-output-of`;
- `handled-by`;
- `documented-by`;
- `tested-by`;
- `consumed-by`;
- generic `co-changes-with` for relationships that remain unclassified.

The raw co-change explorer can be the discovery layer beneath richer deterministic adapters.

---

## Candidate next experiments

1. **Semantic edit classification for Oxc `rules.rs`.** Distinguish comment/docs-only edits from item/registration changes and test whether source-semantic changes reach 100% generated-registry support in the sampled window.
2. **Generated marker/generator discovery.** Detect Oxc's generated files and generator commands explicitly, then combine explicit derivation with historical support.
3. **Item-level Codex export cohort.** Identify protocol items participating in schema export and compare their history with the aggregate JSON/TypeScript outputs.
4. **Changed-diff negative-space finding.** On a controlled branch, modify an Oxc-style source registration without generated companions and verify Cargo Cultist can produce a receipt-backed question.
5. **Era sensitivity.** Re-run across older windows and detect migrations where a current relationship replaced an older mechanism.
6. **Lock-order relation species.** Move beyond file co-change by replaying the WGPU case with a narrow acquisition-order extractor.
7. **Test-selection relation species.** Connect workflow commands to exact test declarations and replay Fieldwork's zero-test-green cases.

## Current disposition

**Continue.**

The experiment has crossed the first useful bar: a repository relationship previously learned through manual investigation was recovered deterministically from raw Git history, with counterexamples visible and no universal rule supplied in advance.

The next implementation work should improve cohort specificity and combine historical association with explicit repository facts. The current explorer should remain an evidence/reporting tool until those refinements establish which negative-space associations deserve diff findings.

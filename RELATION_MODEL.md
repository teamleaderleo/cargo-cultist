# Conditional relation model

This note develops the relational idea behind Cargo Cultist into a more explicit model.

It is intentionally a research model rather than a committed public schema. The purpose is to make the questions precise enough that experiments can prove which pieces are useful.

## Core claim

A repository is more than a collection of files and syntax trees.

It contains repeated relationships such as:

- when this source registry changes, these generated registries usually change;
- when this protocol surface changes, this runtime handler commonly changes;
- when this feature exists, these tests or CI lanes usually exist;
- when this lock is held, another lock is usually acquired later in one order;
- when this resource is registered, cleanup usually appears somewhere nearby;
- when this dependency capability is enabled, configuration or packaging changes often accompany it;
- when this error crosses the CLI boundary, repository-native context or hints usually accompany it.

Traditional lint rules usually begin after one of these relationships has already been understood and encoded.

Cargo Cultist can operate earlier:

> recover candidate relationships from repository evidence, preserve their limits and counterexamples, and ask when a new change departs from them.

The useful unit therefore is not simply a fact about node `A` or node `B`.

It is closer to:

```text
R(A -> B | cohort C, scope S, era E)
```

with evidence explaining why the repository suggests that relationship.

---

## 1. A relation is conditional

The first Oxc replay gives the clearest example.

Raw history found:

```text
P(generated registry changes | rules.rs changes) = 99/100
```

The sole absence was a documentation-only edit that added upstream license/source links.

So the actual useful question is closer to:

```text
P(generated registry changes
  | rules.rs has a source-semantic registration change)
```

The condition is part of the relationship.

A relationship without its cohort can be badly misleading.

Candidate cohort dimensions include:

- changed path;
- changed symbol or item kind;
- AST operation;
- subsystem/crate/package;
- feature/configuration context;
- platform;
- caller/callee role;
- error-path versus success-path change;
- generated versus authored file;
- public API versus internal implementation;
- test versus production code;
- release era or time window;
- commit class such as docs-only, generated-only, dependency update, broad mechanical migration;
- repository-owned labels or metadata;
- explicit generator/command ownership.

This suggests that cohort selection is one of Cargo Cultist's central technical problems.

---

## 2. Direction is first-class

The same Oxc replay showed:

```text
P(generated registry changes | source registry changes) = 99%
P(source registry changes | generated registry changes) = 94%
```

The difference is meaningful.

Generated implementation machinery can change for performance or internal reasons without a source registration change. A source registration change almost always produces generated registry movement.

Therefore these are different relationships:

```text
source registry -> expected generated companions

generated registry -> frequently co-changing source registry
```

A repository relation model should never assume that an empirical edge is symmetric.

### Candidate direction categories

#### Symmetric association

```text
A <-> B
```

Meaning: A and B frequently appear in the same relevant cohort.

This is discovery evidence. It carries little causal interpretation.

#### Directed empirical expectation

```text
A -> B
```

Meaning: when A changes under cohort C, B usually accompanies it.

This can support a negative-space question when B is absent.

#### Explicit derivation

```text
A => B
```

Meaning: repository evidence explicitly establishes that B is generated or derived from A, or from an operation whose inputs include A.

Historical co-change may reinforce the relation, but it does not create the derivation fact.

#### Ordering relation

```text
A before B
```

Meaning: the repository repeatedly orders operations, locks, lifecycle phases, or publications this way under comparable circumstances.

#### Ownership / consumption relation

```text
A handled-by B
A tested-by B
A documented-by B
A consumed-by B
A configured-by B
```

These relations can be directional while expressing different meanings from generation.

---

## 3. Relation type belongs beside relation strength

One source surface can have several valid companion families.

The Codex replay found that changes to a v2 thread protocol surface commonly travel with:

- the thread request processor;
- app-server documentation;
- aggregate schemas;
- protocol tests;
- integration tests;
- TUI consumers.

Collapsing this to `co_changes_with` loses the most interesting information.

A mature relation index may contain edges such as:

```text
thread protocol --handled-by--> thread processor
thread protocol --documented-by--> app-server README
thread protocol --exported-into--> JSON schema family
thread protocol --tested-by--> protocol tests
thread protocol --tested-by--> integration tests
thread protocol --consumed-by--> TUI session code
```

The raw historical explorer can discover candidate edges.

Other deterministic adapters can type them later.

---

## 4. Empirical evidence and explicit evidence are independent dimensions

Cargo Cultist should keep these evidence sources separate.

### Empirical history

Examples:

- 99 of 100 comparable commits contain companion B;
- every recent semantic change to A also changed B;
- the reverse direction occurs only 94% of the time;
- the relation has weakened over the last two years.

This is `OBSERVED` evidence.

### Static repository facts

Examples:

- B begins with `@generated`;
- a generator script writes B;
- Cargo metadata declares a workspace member/dependency;
- a workflow runs a generator;
- a test module imports a production module;
- an explicit lock-rank table orders two locks.

These can be `PROVEN` or `DERIVED` depending on the exact fact.

### Historical rationale

Examples:

- a commit message says the pair must remain synchronized;
- a PR review explains why one exception exists;
- an issue records a migration from old mechanism X to new mechanism Y.

This is project memory evidence. It may explain counterexamples or temporal drift.

### Executed evidence

Examples:

- running the generator after changing A changes B;
- running a CI selector lists zero tests;
- reversing a lock order deadlocks in a controlled model;
- an MSRV build cannot resolve the declared dependency graph.

Execution can raise confidence while preserving the relation's exact boundary.

### Human-taught policy

A maintainer may explicitly decide:

```text
When A changes in cohort C, B must change or an exception record must explain why.
```

At this point the relation is approaching deterministic project policy.

---

## 5. Evidence composition is more interesting than a universal threshold

Consider a generated-file finding.

Weak packet:

```text
B changed with A in 73% of historical commits.
```

Interesting, but noisy.

Stronger packet:

```text
PROVEN
  B contains an explicit generated-file marker.

PROVEN
  repository generator G writes B.

OBSERVED
  B changed in 99/99 comparable source-semantic changes to A.

OBSERVED
  reverse-direction generated-only changes exist for performance work.

PROVEN
  current diff changes A in the semantic cohort.

PROVEN
  current diff does not change B.

UNKNOWN
  repository evidence has not yet recovered an exception reason for this change.
```

Question:

```text
Was regeneration intentionally deferred, or is the generated companion stale?
```

No universal probability threshold is needed to understand why this packet deserves attention.

---

## 6. Counterexamples belong inside the relation

A relation should retain examples where it did not hold.

Conceptually:

```text
relation:
  trigger: source registry semantic change
  expected_companion: generated registry
  support: 99
  opportunities: 99
  counterexamples: 0

excluded_from_cohort:
  - docs-only attribution change
```

Or, for a weaker relation:

```text
relation:
  trigger: thread protocol change
  expected_companion: JSON schema
  support: 34
  opportunities: 51

counterexamples:
  - internal helper-only change
  - documentation correction
  - runtime-only behavior change
```

Counterexamples can teach the cohort classifier.

They can also reveal that a candidate edge is really several relationships mixed together.

---

## 7. Relation discovery and relation interpretation should be separate phases

### Phase A: candidate edge discovery

Cheap signals:

- file co-change;
- same-file co-occurrence;
- import/reference adjacency;
- naming similarity;
- test/prod pairing;
- generator markers;
- workflow commands;
- Cargo workspace/dependency connections;
- lock acquisition ordering;
- repeated helper invocation patterns.

Output:

```text
candidate_relation(A, B)
```

### Phase B: cohort refinement

Ask which occurrences of A are genuinely comparable.

Examples:

- semantic changes only;
- exported items only;
- error-producing branches only;
- same crate only;
- recent era only.

### Phase C: relation typing

Use stronger repository facts to ask whether the candidate is:

- generation;
- test ownership;
- runtime handling;
- documentation;
- configuration;
- lifecycle cleanup;
- ordering;
- API/client mirroring;
- generic association.

### Phase D: negative-space application

When a diff contains A but no expected B, ask whether the changed A belongs to the relation cohort and whether an exception applies.

### Phase E: explanation / archaeology

Search Git history, PRs, issues, comments, or taught records for why the current absence may be intentional.

---

## 8. Negative space is a relation query

A conventional analyzer asks:

```text
What suspicious thing exists in the diff?
```

Cargo Cultist can also ask:

```text
What usually exists with this changed thing, but is absent here?
```

That becomes:

```text
changed(A)
AND relation(A -> B | C)
AND current_change ∈ C
AND absent(B)
```

Then search:

```text
known_exception(current_change, relation)?
```

Only after that should a finding be emitted.

A finding can say:

```text
FACTS
  A changed.
  B did not change.
  37 of 38 comparable A changes included B.
  The one historical absence was documentation-only.
  This change modifies executable syntax.

OBSERVATION
  The current change differs from every comparable syntax-changing example.

UNKNOWN
  Repository evidence has not established whether this is a deliberate exception.

QUESTION
  Is B intentionally unchanged?
```

This is the relational heart of Cargo Cultist.

---

## 9. Relations may connect different entity kinds

A repository graph whose nodes are only files will hit a ceiling quickly.

Candidate entity kinds include:

### Files and directories

```text
File
Directory
GeneratedFile
WorkflowFile
Manifest
Lockfile
```

### Source entities

```text
Crate
Package
Module
Type
Function
Method
Trait
Impl
Field
Static
Constant
Macro
Attribute
FeatureFlag
ErrorVariant
```

### Runtime / lifecycle entities

```text
Lock
Resource
Callback
Handler
Queue
Cache
StateOwner
LifecyclePhase
PublicationPoint
CleanupPath
```

### Test entities

```text
TestFunction
TestModule
TestBinary
TestFilter
Fixture
Snapshot
Benchmark
```

### Build/release entities

```text
Generator
Command
CIJob
Artifact
Schema
SDK
Archive
ReleaseTag
VersionDeclaration
DependencyConstraint
```

### Historical entities

```text
Commit
PullRequest
Issue
ReviewComment
DecisionRecord
ExceptionRecord
```

Relations can cross entity kinds:

```text
SourceItem --exported-by--> Generator
Generator --writes--> GeneratedFile
SourceItem --tested-by--> TestFunction
TestFilter --selects--> TestBinary/TestFunction
ProtocolType --handled-by--> RuntimeHandler
Lock --acquired-before--> Lock
FeatureFlag --covered-by--> CIJob
DependencyConstraint --must-fit--> DeclaredMSRV
```

This cross-kind graph is where the project becomes much more than a style analyzer.

---

## 10. Binary edges are sometimes insufficient

Some repository customs are group relations.

Example:

```text
semantic rule registration change
  -> generated rules enum
  -> generated runner table
```

The meaningful expectation may be that **both** outputs belong to one generated companion set.

Representing them as independent edges is useful but incomplete.

Possible higher-order relation:

```text
trigger:
  rules.rs semantic registration change

expected_set:
  - generated/rules_enum.rs
  - generated/rule_runner_impls.rs

policy:
  all
```

Other relations may be alternatives:

```text
expected_any:
  - regression test
  - explicit exception record
```

Lifecycle relations can be sequences:

```text
prepare -> own -> publish -> retire predecessor
```

Lock relations can form a partial order:

```text
snatchable < command_indices < pending_writes
```

The long-term model therefore probably needs relation groups or small predicates, not only binary graph edges.

---

## 11. Procedure relations differ from changed-file relations

The Oxc replay showed a timing snapshot changing in only a minority of rule-registration commits even though repository instructions may require running timing tooling.

These are separate facts:

```text
A change -> command G should run
```

versus:

```text
A change -> file B usually changes
```

A successful command may produce no diff.

So the relation model should include procedural edges:

```text
ChangeClass --validated-by--> Command
ChangeClass --validated-by--> CIJob
ChangeClass --may-produce--> File
```

This prevents Cargo Cultist from treating absence of an output diff as proof that a required gate was skipped.

---

## 12. Scope tension applies to relations too

Relationships can disagree by scope.

Example:

```text
repository-wide:
  new parser modules usually have integration tests

crate-local:
  this crate uses inline unit tests

same-directory:
  neighboring parser modules have snapshot fixtures
```

Or:

```text
historical:
  generated schema lived under old path X

recent:
  generator now writes path Y
```

Cargo Cultist should show relation tension instead of flattening it into one score.

Potential scopes:

- same function;
- same module;
- same file;
- same directory;
- same crate/package;
- same subsystem;
- workspace;
- repository;
- recent history;
- selected era;
- platform-specific surface.

---

## 13. Relations age

Historical relationships can become fossils.

A relation index needs temporal validity:

```text
first_seen
last_seen
recent_support
historical_support
possible_migration_boundary
```

Example:

```text
old mechanism:
  A -> generated_old.rs

new mechanism:
  A -> generated_new.rs
```

If Cargo Cultist only aggregates all history, it can recommend the obsolete companion.

Possible relation states:

```text
emerging
stable
mixed
migrating
legacy
stale
unknown
```

These are observations about evidence, not declarations of policy.

---

## 14. A possible relation evidence record

Conceptual only:

```yaml
relation_id: oxc-linter-rule-registry-generated-outputs

trigger:
  entity: crates/oxc_linter/src/rules.rs
  change_class: rust-syntax-change

relation:
  type: expected-companion
  direction: trigger-to-companion

companions:
  all:
    - crates/oxc_linter/src/generated/rules_enum.rs
    - crates/oxc_linter/src/generated/rule_runner_impls.rs

scope:
  repository: oxc-project/oxc
  subsystem: oxc_linter
  era:
    through: <exact source pin>

empirical:
  opportunities: 99
  support:
    rules_enum.rs: 99
    rule_runner_impls.rs: 99
  excluded_cohort_examples:
    - commit: 5e113baf...
      reason: documentation-only anchor edit

explicit_evidence:
  generated_markers: []
  generator_commands: []

interpretation:
  kind: observed-directed-expectation
  causal_status: unknown

questions:
  - Does repository generator metadata establish the derivation owner?
```

The real schema should emerge from actual analyzers rather than this sketch.

---

## 15. Relation confidence should stay multidimensional

A single numeric confidence score would throw away useful distinctions.

Useful dimensions include:

### Support

How often does the relation appear in the selected cohort?

### Cohort size

`3/3` and `99/99` deserve different treatment.

### Counterexample quality

Are absences random, or do they form a coherent exception class?

### Recency

Does the relation still hold in current development?

### Locality

Does the relation hold specifically in the changed subsystem?

### Explicitness

Does the repository explicitly declare generation/ownership/testing?

### Directionality

Is the forward relation much stronger than the reverse relation?

### Executability

Can the relation be checked by running a deterministic command?

### Explanation coverage

Can exceptions be explained from repository history?

A finding packet can expose these dimensions directly.

---

## 16. Relation promotion ladder

Candidate progression:

```text
R0  candidate association
    raw co-change or co-occurrence signal

R1  observed directional precedent
    support + cohort + counterexamples

R2  typed repository relation
    generated-by / tested-by / handled-by / ordered-before / etc.

R3  explained relation
    exceptions and historical rationale are understood

R4  taught project expectation
    humans explicitly record intended behavior

R5  deterministic policy
    stable relation becomes a lint, schema check, generated-file check,
    type-level rule, test, or CI assertion
```

Cargo Cultist is especially valuable in R0-R3, where conventional linters usually have little to say.

It can also help detect when an R4 relationship is mature enough for R5.

---

## 17. Failure modes to avoid

### Global percentage thresholds

```text
support >= 90% => error
```

This would be crude and easy to game accidentally.

### Treating all co-change as causal

Two files can move together because a broad refactor repeatedly touched both.

### Undirected relation storage

The Oxc replay already falsifies this simplification.

### Erasing counterexamples

The 1% exception may contain the exact reason the current change is legitimate.

### Using current file state to rewrite historical truth

A file can become generated today even if old commits predate the generator.

### Treating a generated marker as generator ownership

`@generated` proves self-identification, not what source owns the output or when regeneration is required.

### Treating a green workflow as procedural completion

A command can succeed after selecting zero tests.

### Treating file movement as proof a command ran

Required validation commands often leave no diff.

### Ignoring migrations

Old dominant relationships can be obsolete.

### Building the entire ontology before useful findings exist

Relation types should graduate from concrete corpus cases.

---

## 18. The Fieldwork corpus is unusually suited to relation discovery

Fieldwork repeatedly records exactly what had to be learned before a change could be judged correctly.

That gives Cargo Cultist labeled research questions such as:

### Oxc

What generated and procedural companions follow rule registration?

### Codex

Which protocol, schema, SDK, runtime, documentation, test, and release identities belong to one generation?

### WGPU

Which lock acquisition order is repository precedent, and where does a new inverse edge appear?

### uv

Which error types, context wrappers, hints, exit codes, and snapshot locations form the repository-native diagnostic path?

### Tantivy

Which workspace metadata and dependency constraints collectively define the declared MSRV contract?

### Tauri

Which callback/lock/snapshot/pending-queue mechanisms establish reentrancy and panic recovery precedent?

### Fieldwork itself

Which workflow/test-selection patterns distinguish actual executed evidence from green commands with zero intended tests?

For each case, ask:

> Could Cargo Cultist recover the relevant relation before Fieldwork had to discover it manually?

That is a strong evaluation criterion.

---

## 19. Near-term implementation consequences

The current evidence suggests this sequence.

### A. Preserve direction in historical association

Every companion report should conceptually be read as:

```text
P(companion | anchor-change cohort)
```

The reverse query is separate evidence.

### B. Add cohort classifiers incrementally

Start with cheap, deterministic distinctions where a corpus case gives a clear discriminator:

- Rust syntax-changing versus comments/docs-only;
- generated-only change;
- dependency-only change;
- workflow-only change;
- same-package/subsystem change.

### C. Add explicit relation facts

Examples:

- generated marker;
- generator command output paths;
- Cargo package ownership;
- test declaration and target identity;
- lock acquisitions;
- import/call references.

### D. Compose evidence before adding findings

Do not jump from raw history percentage to a missing-companion warning.

Build packets that combine independent evidence sources.

### E. Let relation types emerge from successful packets

If generated relations, test-selection relations, and lock-order relations each prove useful, extract the common relation engine afterward.

This keeps the project empirical.

---

## 20. Long-term possibility: repository grammar

The ambitious version of Cargo Cultist can recover a repository-specific grammar such as:

```text
When adding this kind of parser:
  implementation lives here
  tests live there
  snapshots use this mechanism
  errors cross this boundary through that type
  registration happens in this generated table
  generator G owns the table
  CI job J validates the change
  this exception class is documented separately
```

Or:

```text
When changing this protocol item:
  runtime handler H usually changes
  generated schemas S1/S2 may change when export-relevant fields move
  TypeScript client output is generated through G
  compatibility tests T cover boundary cases
  docs D track public methods
```

Or:

```text
When acquiring lock L2 while L1 is held:
  repository precedent gives L1 < L2
  this diff introduces the first inverse edge
```

That is the category worth pursuing.

It sits between a universal linter and unconstrained repository-wide AI review:

```text
repository evidence
-> conditional relationships
-> missing or contradictory precedent
-> bounded findings
-> human explanation
-> durable institutional memory
-> eventual deterministic policy where justified
```

The relationship is the bridge.

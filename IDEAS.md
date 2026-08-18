# Cargo Cultist ideas: repository precedent and relational analysis

Status: research notebook. Everything here is a hypothesis, candidate, or research direction until repository evidence and implementation work graduate it.

The current prototype proves one small loop: collect deterministic repository facts, derive repository-specific observations, and raise a question when a changed fact diverges from precedent. The opportunity is much larger.

The central idea in this notebook is **precedent as a first-class primitive**.

A repository carries accumulated customs, contracts, repeated relationships, local mechanisms, historical couplings, ordering rules, placement habits, exception families, and maintenance rituals. Much of that knowledge lives in code and history instead of a written rulebook. Cargo Cultist can mine that evidence and surface the interesting places where a change deserves a second look.

The long-term goal remains the README's core idea: find out why before you copy it.

---

## 1. North star

Traditional static analysis starts from a rule that already exists.

Cargo Cultist can start from repository evidence and ask:

> What does this repository repeatedly do in comparable situations, and how does this change relate to that precedent?

The most useful output is a **finding with receipts**.

A useful mental equation:

```text
changed or interesting fact
+ relevant precedent
+ strength of precedent
+ counterexamples
+ representative exemplars
+ uncertainty
= finding + question
```

Rarity alone is a discovery signal. The finding becomes useful when Cargo Cultist can explain why the rare case belongs to a meaningful comparison set.

For example, `1 occurrence out of 900` says little by itself. `1 occurrence among 47 comparable call sites in the same subsystem, while all 46 siblings use the same helper` is strong evidence.

The tool should continuously answer two questions:

1. **Why is this interesting?**
2. **What evidence produced that conclusion?**

That keeps the tool useful even when the answer to the final human question is, "yes, this exception is intentional."

---

## 2. The current prototype already contains the seed

The current test-module analyzer has several important ingredients:

- deterministic AST extraction;
- repository-wide counts;
- same-file precedent;
- one-off detection;
- local mixtures;
- diff-aware selection;
- a finding phrased as a question;
- zero requirement for an LLM.

The current unit of analysis is simple:

```text
file -> contains -> test-gated module named X
```

The next leap is to generalize from isolated attributes into **relationships between repository entities**.

---

## 3. The big idea: repositories contain relationships

A repository is full of relationships that maintainers know implicitly.

Examples:

```text
source file -> usually changes with -> generated artifact
protocol definition -> generates -> SDK types
production module -> is usually tested by -> test module
lock A -> is usually acquired before -> lock B
error variant -> usually carries -> recovery hint
command family -> usually wraps errors with -> operation context
manifest field -> is certified by -> CI job
crate feature -> is mirrored by -> forwarding feature
public API -> has compatibility fixture in -> test directory
source-authored registry -> produces -> generated registration file
subsystem -> normally reuses -> helper/mechanism
commit touching A -> usually also touches -> B
unusual workaround -> was introduced by -> historical commit
```

These relationships are often closer to the real review question than a local syntax rule.

A proposed change can be individually valid Rust while violating one of these repository-level expectations.

This suggests a conceptual graph. Cargo Cultist does not need a graph database to begin; the graph is a useful model for what facts mean.

### Candidate entity kinds

- repository
- workspace member / crate
- directory / subsystem
- file
- module
- item
- function / method
- type
- trait / impl
- call site
- lock / guard
- static / global
- error type / error variant
- test
- test module
- fixture
- snapshot
- workflow
- workflow step
- command
- manifest
- manifest field
- dependency
- feature
- generated artifact
- generator
- configuration key
- release artifact
- commit
- author / bot identity when useful for filtering history

### Candidate relationship kinds

- `contains`
- `calls`
- `uses`
- `reuses-mechanism`
- `tested-by`
- `co-changes-with`
- `generated-from`
- `generated-by`
- `certified-by`
- `declares`
- `mirrors`
- `forwards-feature-to`
- `acquired-before`
- `dropped-before`
- `wraps-with-context`
- `reports-with-hint`
- `maps-error-to-exit-class`
- `lives-near`
- `shares-naming-family-with`
- `introduced-by`
- `changed-after`
- `paired-with`
- `gated-by`
- `published-with`
- `version-coupled-to`
- `owned-by-subsystem`

The exact vocabulary should grow from real cases. A tiny useful vocabulary beats a giant speculative ontology.

---

## 4. A fact model for relational analysis

One possible internal representation:

```text
Fact {
    subject
    predicate
    object/value
    context
    provenance
}
```

Examples:

```text
(subject=file:src/foo.rs,
 predicate=contains-test-module,
 object=name:tests)

(subject=lock:command_indices,
 predicate=acquired-before,
 object=lock:pending_writes,
 context=function:Queue::compact_blas_inner)

(subject=file:protocol/schema.rs,
 predicate=co-changes-with,
 object=file:sdk/generated/types.ts,
 context=history:last-2-years)
```

Every fact should retain provenance. Provenance is what makes the tool auditable.

Possible provenance:

- exact source path and span;
- Cargo metadata command/result;
- Git commit and changed paths;
- workflow path and line;
- generated-file marker;
- manifest path and key;
- test declaration path;
- command execution receipt when execution exists.

The current README already points toward claim classes such as proven, derived, observed, inferred, and unknown. Relational facts fit that model naturally.

---

## 5. Precedent is an observation over comparable facts

A precedent should mean more than frequency.

The hard problem is choosing the **comparison cohort**.

Examples:

- every Rust file in a repository may be too broad;
- files in the same crate may be useful;
- files in the same subsystem may be stronger;
- sibling rule implementations may be ideal;
- commits touching the same protocol family may be the right historical cohort;
- lock acquisitions inside one owner type may be more meaningful than global lock order;
- command implementations in the same CLI namespace may carry the relevant diagnostic convention.

Cargo Cultist should treat cohort selection as a first-class piece of evidence.

A finding should be able to say:

```text
Comparison set:
  34 historical commits touching protocol source files under X

Support:
  31/34 also changed generated schema Y

Counterexamples:
  3/34
  - one generator-only migration
  - one revert
  - one repository-wide rename
```

That is far more useful than a mysterious score.

---

## 6. Dimensions of precedent

These dimensions can overlap. A single finding may use several.

### 6.1 Spatial precedent

How does nearby code do this?

Examples:

- same file;
- sibling module;
- same impl block;
- adjacent rule implementations;
- same test directory;
- same crate.

The existing test-module check already uses same-file precedent.

### 6.2 Repository-wide precedent

What does the codebase usually do?

Examples:

- dominant test module name;
- dominant error wrapper;
- dominant fixture naming family;
- dominant feature-forwarding pattern.

### 6.3 Temporal precedent

How were comparable changes made historically?

Examples:

- protocol source and generated client move together;
- a dependency bump usually updates a compatibility fixture;
- a manifest field change usually comes with CI movement;
- new rules usually arrive with registration and generated timing output.

### 6.4 Relational precedent

Which entities repeatedly travel together?

Examples:

- source/generated pairs;
- implementation/test pairs;
- feature/forwarding-feature pairs;
- API/snapshot pairs;
- schema/client pairs.

This is the largest unexplored area.

### 6.5 Ordering precedent

Which operations repeatedly occur in the same order?

Examples:

- lock acquisition order;
- snapshot -> drop guard -> callback;
- validate -> normalize;
- settle old generation -> publish new generation;
- catch -> drop guard -> flush -> resume panic;
- write durable state -> publish success.

Ordering analysis can expose implicit safety contracts.

### 6.6 Placement precedent

Where does this repository put this kind of thing?

Examples:

- inline tests versus dedicated integration test files;
- snapshot location;
- rule registration location;
- error type location;
- generated outputs;
- fixtures beside subsystem owners.

### 6.7 Mechanism precedent

Which existing local mechanism does this subsystem normally use?

Examples:

- one shared helper handles normalization;
- all siblings use the repository hint framework;
- a state manager is reused everywhere except one new local cache;
- one established generator owns registrations.

This connects directly to the README idea: find duplicated local mechanisms when a common helper already exists.

### 6.8 Contract precedent

Which promises emerge from several repository surfaces together?

Examples:

- `rust-version` + dependency ranges + CI compiler version;
- generated SDK version + runtime archive + release tag;
- feature declaration + forwarding features + CI matrix;
- public API type + serialization path + receiver expectation.

A contract finding may span manifests, code, workflows, and generated artifacts.

### 6.9 Exception precedent

A rare pattern can itself form a coherent family.

Examples:

- every platform-specific exception lives behind the same cfg;
- every intentionally distinct test module name has a domain-specific purpose;
- unusual constants all trace to one compatibility workaround;
- a special error path exists only for one protocol generation.

Cargo Cultist should learn subcultures instead of flattening every repository into one dominant convention.

### 6.10 Lifecycle precedent

Repositories often repeat a lifecycle grammar:

```text
prepare -> own -> publish -> retire predecessor
```

or:

```text
snapshot -> release shared state -> invoke caller code
```

or:

```text
request cancellation -> bound delivery -> classify outcome -> retire transport
```

Mining these sequences is ambitious, but Fieldwork repeatedly discovers bugs by recovering exactly these local lifecycle expectations.

---

## 7. Evidence packets: every finding should carry receipts

A future finding packet could contain:

```text
Finding kind
Changed fact(s)
Comparison cohort
Observed precedent
Support / opportunities
Local support
Repository-wide support
Historical support
Recency
Representative exemplars
Counterexamples
Known exception families
Unknowns
Question
```

Example:

```text
FINDING: historical co-change precedent

CHANGE
  protocol/src/schema.rs changed.

FACTS
  Comparison cohort: 34 non-merge commits touching this protocol surface.
  31/34 also changed sdk/typescript/src/generated/schema.ts.
  29/34 also changed schema/openapi.json.

COUNTEREXAMPLES
  1 revert
  1 generator migration
  1 repository-wide rename

OBSERVATION
  This change differs from the dominant historical change set for this surface.

QUESTION
  Is generation intentionally deferred, or are related artifacts missing?
```

A human can answer immediately because the evidence is right there.

---

## 8. Confidence should be decomposed instead of hidden

A single confidence score risks becoming meaningless.

Useful dimensions include:

- **support** — how many comparable examples exist?
- **consistency** — how often does the relationship hold?
- **locality** — is the precedent from the same file, crate, or subsystem?
- **recency** — does current code/history still follow it?
- **specificity** — how narrowly does the cohort match the changed fact?
- **exception coherence** — do counterexamples form understandable families?
- **source authority** — explicit repository docs/generators/contracts can outrank inferred history.
- **history quality** — did filtering remove merge commits, sweeps, reverts, generated-only noise, and mechanical migrations?

Output can expose these dimensions directly.

---

## 9. First major candidate: historical co-change precedent

This feels like the cleanest proof that Cargo Cultist has a distinct thesis.

### Question

When file/entity A changes, what else usually changes with it in comparable commits?

### Candidate facts

For each relevant commit:

- changed paths;
- changed Rust items where feasible;
- commit size;
- merge/revert identity;
- timestamp;
- author/bot identity when useful for noise filtering;
- generated markers;
- subsystem/path family.

### Naive model

```text
P(B changes | A changes)
```

### Better model

```text
P(B changes | A changes, comparable cohort C)
```

### Potential findings

- source file changed while strongly coupled generated artifact stayed unchanged;
- API declaration changed while compatibility fixture stayed unchanged;
- dependency pin changed while lock/config partner stayed unchanged;
- feature changed while forwarding feature stayed unchanged;
- protocol source changed while one client generation stayed unchanged.

### Important noise filters

- merge commits;
- reverts;
- repository-wide formatting;
- mass renames;
- generated-only sweeps;
- dependency bot batches;
- vendored code updates;
- monorepo commits touching unrelated packages;
- historical eras before a generator or workflow existed.

### Research targets

**Codex** is especially strong because Fieldwork already investigated runtime, protocol, generated clients, package versions, platform archives, lock graphs, and release identity together: <https://github.com/teamleaderleo/fieldwork/issues/413>.

**Oxc** is a beautifully bounded source/generated/test-ceremony target. Fieldwork recorded source-authored files, generated registration/timing output, inline `Tester` coverage, and repository generator commands: <https://github.com/teamleaderleo/fieldwork/issues/601>.

If Cargo Cultist can independently recover useful co-change relationships from these repositories, the thesis becomes concrete.

---

## 10. Generated artifact coupling

Historical co-change can feed a more semantic detector for generated outputs.

Potential evidence sources:

- `@generated` comments;
- generator scripts;
- build tasks;
- workflow commands;
- source comments naming generators;
- output directories;
- Git history coupling;
- deterministic regeneration comparison.

Potential findings:

### Missing regeneration

```text
FACTS
  src/rules.rs changed.
  Generated registry X changed in 42/44 comparable commits.
  X remains unchanged in this diff.

QUESTION
  Is regeneration intentionally deferred?
```

### Manual edit of generated output

```text
FACTS
  generated/foo.rs changed.
  Its declared source inputs and generator stayed unchanged.
  Comparable repository changes modify this file through generator output.

QUESTION
  Was this file intentionally edited directly?
```

This is stronger when the repository explicitly marks generated files.

---

## 11. Test coupling and test placement

Repositories teach maintainers where tests belong and which behavior deserves a test.

Potential facts:

- implementation item -> historically changed with test file;
- module -> sibling test module path;
- rule -> inline `Tester` block;
- error message -> snapshot file;
- public API -> integration test family;
- feature -> compile-test matrix.

Potential findings:

- implementation changed while the historically paired regression surface stayed untouched;
- a new test appears in a location that differs from sibling precedent;
- a new rule uses a standalone test file while sibling rules use inline tests;
- a workflow invokes a test filter that selects zero declared tests.

Fieldwork's exact-head evidence audit is a rich source of test-selection cases: <https://github.com/teamleaderleo/fieldwork/issues/225>.

One especially appealing deterministic check:

```text
workflow command says:
  cargo test --lib test_rollback

repository test inventory says:
  zero exact tests named test_rollback

observation:
  command can report success while selecting zero intended tests
```

This crosses source facts and CI facts while remaining deterministic.

---

## 12. Lock-order precedent

Fieldwork's WGPU work is a strong research case: <https://github.com/teamleaderleo/fieldwork/issues/658>.

### Candidate extractor

Start narrow:

- method calls named `lock`, `read`, `write`, `try_lock`, etc.;
- receiver identity where syntactically recoverable;
- lexical nesting order;
- explicit `drop(guard)`;
- owner function/type;
- repeated pair order.

### Derived relation

```text
lock A -> acquired-before -> lock B
```

### Finding

```text
FACTS
  command_indices -> pending_writes appears at 14 comparable sites.
  This change adds pending_writes -> command_indices.
  This is the first reverse edge in the cohort.

QUESTION
  Is this reverse acquisition intentional?
```

### Why this is exciting

This mines an implicit concurrency contract from the repository itself.

A repository may already have ranked-lock tests or explicit lock-order documentation. Those explicit signals can reinforce the inferred relationship.

### Boundaries

Alias analysis will complicate broad coverage. The first version can stay syntactic and local. High precision on obvious lock identities is enough to prove value.

---

## 13. Repository-native error and diagnostic conventions

Fieldwork's uv investigation recovered a lot of tacit repository knowledge around error propagation, `anyhow::Context`, the `Hint` trait, cause walking, exit classification, and snapshot placement: <https://github.com/teamleaderleo/fieldwork/issues/627>.

Potential relationship facts:

```text
command family -> wraps fallible inventory call with -> operation context
error variant -> implements -> Hint
hint-capable error -> is collected by -> diagnostic walker
command -> maps error family to -> exit classification
error output -> is covered by -> snapshot
```

Potential findings:

- new sibling command propagates a fallible operation without context while comparable commands add context;
- a new actionable error variant omits the repository's hint mechanism;
- a command manually renders an error while siblings preserve the standard error chain;
- a diagnostic change lacks the snapshot family used by its siblings.

The key is repository evidence. Cargo Cultist should avoid inventing a universal opinion about error libraries.

---

## 14. Mechanism reuse and duplicated local machinery

A recurring Fieldwork question is:

> Does this code create a second mechanism where the repository already has one owner?

Potential examples:

- custom normalization beside a shared normalizer;
- local cache beside repository cache owner;
- hand-written generation beside a generator;
- local retry loop beside lifecycle/retry owner;
- bespoke error formatting beside diagnostic framework;
- manual path scanning beside repository discovery helper.

Possible evidence:

- sibling call sites converge on helper H;
- new code duplicates a recognizable call sequence;
- Git history migrated siblings from local logic to H;
- new code appears after that migration and reintroduces the old sequence.

This may eventually need semantic similarity or sequence matching. The first detectors can target known local call sequences discovered from the corpus.

---

## 15. Test-only global state and nearby alternatives

This is already in the README's near-term ideas.

The repository-aware version is stronger than a generic "global state in tests is bad" rule.

Potential finding:

```text
FACTS
  New test introduces a process-global environment mutation.
  17 sibling tests in this crate use scoped helper TestEnv.
  2 older exceptions exist; both predate TestEnv.

OBSERVATION
  The change uses an older mechanism while current local precedent uses TestEnv.

QUESTION
  Should this test use the scoped helper?
```

Historical migration can make this especially compelling.

---

## 16. Cargo / workspace contract coherence

Fieldwork's Tantivy MSRV investigation is a clean example: <https://github.com/teamleaderleo/fieldwork/issues/200>.

Repository-level facts can include:

- root `rust-version`;
- member `rust-version`;
- editions;
- direct dependency ranges;
- dependency MSRV when available;
- lockfile policy;
- CI compiler matrix;
- docs declaring supported compiler versions.

Potential observations:

- declared MSRV conflicts with a direct dependency floor;
- workspace members diverge from root compiler contract;
- CI no longer certifies the declared minimum;
- dependency changes historically update one compatibility job, while this one does not.

This family looks more like conventional consistency analysis, yet Cargo Cultist can enrich it with historical and repository-specific evidence.

---

## 17. Version and release coherence

Large repositories publish several artifacts that can drift independently.

Codex gives a strong case:

```text
Rust source
protocol source
schema generation
TypeScript SDK
Python SDK
platform archives
release tag
package lock graph
runtime self-reported version
```

Potential relationships:

```text
protocol source -> generates -> schema
schema -> generates -> SDK client
package version -> selects -> runtime archive tag
release tag -> contains -> platform artifacts
SDK -> expects -> runtime generation
```

Cargo Cultist could begin with static identity relationships and historical co-change, then eventually consume deterministic generator/runtime receipts.

---

## 18. Feature and configuration symmetry

Rust workspaces often contain local feature propagation customs.

Potential facts:

- crate A feature X forwards to dependency B/X;
- integration test crate mirrors feature X;
- workspace feature families appear across sibling crates;
- cfg gates and Cargo features pair consistently.

Potential finding:

```text
FACTS
  A new `dynamic-acl` feature was added to crate X.
  Every comparable integration test feature forwards the product feature.
  Test crate Y has no forwarding entry.

QUESTION
  Does the test crate need to expose the same feature for coverage?
```

This is especially useful in large Rust workspaces.

---

## 19. Callback, lock, and reentrancy customs

Tauri has several excellent cases in Fieldwork where the important local rule only becomes visible after studying neighboring code:

- caller predicates executing while registries remain locked;
- snapshotting metadata before invoking caller code;
- pending queues for reentrant operations;
- catch/drop/flush/resume sequencing after panic;
- different callback owners using different mechanisms.

Useful Fieldwork entry points include:

- <https://github.com/teamleaderleo/fieldwork/issues/118>
- <https://github.com/teamleaderleo/fieldwork/issues/744>
- <https://github.com/teamleaderleo/fieldwork/issues/749>
- <https://github.com/teamleaderleo/fieldwork/issues/754>

A future Cargo Cultist could mine local callback customs:

```text
shared registry guard -> released-before -> caller callback
```

or:

```text
reentrant mutation -> routed-through -> pending queue
```

These are ambitious detectors, but they show where relational precedent can lead.

---

## 20. Transformation and fix conventions

Biome is useful because it has rule metadata, safe/unsafe fix classifications, rule families, test conventions, and semantic transforms.

Fieldwork has already found cases where "safe" transformations changed runtime behavior, which suggests several repository-aware questions:

- which neighboring rules classify similar rewrites as safe/unsafe?
- which fix families carry runtime before/after tests?
- where do transformation helpers encode language semantics?
- do safe numeric folds use one shared formatting mechanism?
- did a batch metadata promotion change safety classification without adding the semantic controls siblings usually have?

Entry points:

- <https://github.com/teamleaderleo/fieldwork/issues/89>
- <https://github.com/teamleaderleo/fieldwork/issues/146>
- <https://github.com/teamleaderleo/fieldwork/issues/255>

This suggests Cargo Cultist can eventually analyze **classification precedent**, not only syntax precedent.

---

## 21. Exception archaeology

One of the most compelling long-term ideas is connecting unusual code to history.

The README already mentions unusual constants and workarounds.

Candidate flow:

1. detect a local outlier;
2. blame the relevant line/item;
3. inspect introducing commit;
4. inspect nearby historical changes;
5. identify issue/PR references in commit messages where available;
6. compare whether the reason still applies to current code;
7. emit history as evidence before any explanation.

Example:

```text
FACTS
  Constant RETRY_LIMIT=17 is unique in this subsystem.
  Introduced in commit abc123 with message referencing issue #456.
  The same commit added a workaround for server version <= 2.
  Current minimum supported server version is 4.

QUESTION
  Does this compatibility workaround still serve a current supported configuration?
```

The interpretation can remain optional. The historical receipt is deterministic.

---

## 22. Repository subcultures

A project can have several legitimate conventions.

Examples:

- one crate uses `mod tests`, another uses `mod unit_tests`;
- platform code follows a different error model;
- generated code has its own naming family;
- old subsystem and new subsystem have different lifecycle owners;
- integration tests use different placement rules than unit tests.

Cargo Cultist should discover clusters instead of declaring one repository-wide winner whenever evidence shows stable local families.

Possible hierarchy for cohorts:

```text
same file
same module family
same directory/subsystem
same crate
same workspace
repository-wide
```

The most specific cohort with enough evidence may deserve priority.

---

## 23. Precedent drift over time

Repository customs evolve.

History should probably carry decay and eras.

Questions:

- did a helper migration create a new local standard?
- did a generator replace manual registration?
- did an error framework land halfway through history?
- did a workspace split change test placement?

A strong detector should recognize:

```text
2019-2023: old mechanism
2024 migration commit
2024-present: new mechanism
```

A new change copying the 2021 pattern is interesting even if total historical frequency still favors the old mechanism.

Recent precedent can outrank ancient frequency when a clear migration exists.

---

## 24. Comparative history needs anti-noise rules

Git history is incredibly valuable and incredibly noisy.

Likely filters / classifications:

- merge commits;
- reverts;
- cherry-pick/backport patterns;
- repository-wide formatting;
- mechanical renames;
- vendored updates;
- generated-only sweeps;
- dependency bot commits;
- lockfile-only churn;
- bulk code generation;
- file moves;
- historical era changes;
- commits spanning many unrelated monorepo packages.

A first co-change prototype can expose raw and filtered counts so the filtering remains auditable.

---

## 25. Explicit repository rules should reinforce or override inferred customs

Repositories sometimes write the rule down.

Potential sources:

- `AGENTS.md`;
- `CONTRIBUTING.md`;
- developer docs;
- codegen comments;
- workflow comments;
- lint config;
- test instructions;
- Cargo metadata;
- script names;
- generator headers.

These are facts too.

A finding can become much stronger when explicit guidance and observed precedent agree.

Example:

```text
EXPLICIT
  AGENTS.md says generated directories must be updated via `cargo lintgen`.

OBSERVED
  37/38 comparable rule additions modify generated output through the generator.

CHANGE
  Generated file edited directly.
```

---

## 26. Findings should preserve counterexamples

Counterexamples are essential evidence.

They answer:

- is the precedent genuinely strong?
- do exceptions form a legitimate family?
- is this change actually part of that family?
- did the custom change over time?

Cargo Cultist should resist the temptation to hide exceptions just to make a cleaner story.

A good finding can say:

```text
Support: 23/26
Counterexamples:
  2 platform-specific paths
  1 revert

The changed file is platform-neutral.
```

That is stronger than pretending the rule is universal.

---

## 27. No dominant precedent is itself useful information

Sometimes the repository genuinely has several styles.

Possible output:

```text
OBSERVATION
  No stable precedent exists for this relationship.
  Two mechanisms are used with similar frequency in comparable code.

QUESTION
  Is this an area where the repository intentionally supports both patterns?
```

For diff review, this may mean staying quiet. Cargo Cultist earns trust by declining to manufacture findings from weak evidence.

---

## 28. Potential finding lifecycle

A useful conceptual lifecycle:

```text
fact
-> relationship
-> cohort
-> observed precedent
-> changed/interesting deviation
-> finding packet
-> optional explanation
-> human disposition
```

Human dispositions could eventually feed evaluation:

- intentional exception;
- missing companion change;
- stale custom;
- wrong cohort;
- useful question, no code change;
- false positive;
- promoted into explicit repository rule.

This feedback can improve detectors while keeping current execution deterministic.

---

## 29. Candidate internal modules

This is one possible decomposition, meant as an idea instead of an API commitment.

### Fact providers

- Rust AST provider;
- Cargo metadata provider;
- Git history provider;
- workflow/config provider;
- test inventory provider;
- generated-artifact provider.

### Relationship index

Stores deterministic edges such as:

```text
A co-changes-with B
A acquired-before B
A generated-from B
A tested-by B
```

### Cohort selector

Builds meaningful comparison sets.

### Observation engine

Computes repeated relationships, dominant patterns, exception groups, and drift.

### Diff mapper

Maps changed lines/items/files onto facts and relationships.

### Finding renderer

Produces the evidence packet and question.

### Optional explanation layer

Consumes an already-complete evidence packet.

The current code can evolve toward this incrementally. There is no need for a giant framework before the second useful check exists.

---

## 30. Possible CLI evolution

Ideas:

```text
cargo cultist
cargo cultist diff
cargo cultist facts
cargo cultist precedents
cargo cultist why <finding>
cargo cultist history <path-or-item>
cargo cultist --json
```

`facts` could become an invaluable debugging command: show exactly what deterministic facts the analyzer extracted.

`why` could expand a terse finding into all exemplars, counterexamples, and provenance.

`--json` would let CI, editor integrations, and future explanation tooling consume the same evidence.

---

## 31. Proven / derived / observed / inferred / unknown

The README already suggests this refinement. It feels especially important once relationships arrive.

Possible meanings:

- **proven** — directly extracted from exact source, metadata, or history;
- **derived** — deterministic transformation of proven facts;
- **observed** — repeated repository pattern over a defined cohort;
- **inferred** — interpretation about intent or likely reason;
- **unknown** — missing evidence that blocks a stronger claim.

Example:

```text
PROVEN
  31 comparable commits changed A and B together.

DERIVED
  Co-change rate is 91.2%.

OBSERVED
  A and B have strong historical coupling in this subsystem.

INFERRED
  B may be generated from A.

UNKNOWN
  No explicit generator marker has been found yet.
```

This keeps the tool intellectually honest.

---

## 32. LLM role

The deterministic packet should exist first.

Useful optional LLM jobs later:

- summarize why several exemplars look related;
- suggest candidate cohorts for deterministic validation;
- explain an unusual historical commit in plain language;
- group counterexamples into possible exception families;
- turn evidence into a reviewer-friendly question;
- propose additional facts worth extracting;
- retrieve relevant repository docs or history for an already-detected finding.

The LLM should never be the sole source of a fact that can be extracted deterministically.

A powerful flow may be:

```text
LLM proposes hypothesis
-> deterministic providers test hypothesis
-> evidence packet survives or dies
-> optional LLM explains surviving packet
```

---

## 33. Fieldwork as a uniquely valuable research corpus

Fieldwork is unusually well suited to developing Cargo Cultist because it records more than bugs.

It records:

- exact source revisions;
- code maps;
- local conventions discovered during investigation;
- test locations;
- generated surfaces;
- lifecycle owners;
- negative results;
- counterexamples;
- historical overlap;
- exact execution receipts;
- cases where an initial interpretation lost;
- review findings about misleading evidence.

That gives us something close to a labeled corpus of **tacit repository knowledge that had to be learned before a change could be judged correctly**.

The core research question for each Fieldwork case should be:

> What did Fieldwork have to discover about this repository before it could correctly judge the change?

The answer is often a Cargo Cultist candidate.

---

## 34. Fieldwork replay as an evaluation method

This could become one of the strongest ways to evaluate the tool.

For a known Fieldwork case:

1. pin the repository generation from before or during the investigation;
2. identify the human/agent-learned precedent;
3. run Cargo Cultist without giving it the conclusion;
4. ask whether it surfaces the useful relationship;
5. inspect the evidence packet;
6. record false positives and missing context;
7. compare how much investigation work the finding could have shortened.

This is better than evaluating on invented toy repositories alone.

Possible benchmark questions:

- Did Cultist surface the relevant companion file?
- Did Cultist find the lock-order reversal?
- Did Cultist identify the repository-native test location?
- Did Cultist notice that a CI command selects zero intended tests?
- Did Cultist recover the error/hint convention?
- Did Cultist detect a source/generated mismatch?
- Did Cultist distinguish a stable exception family from an accidental one-off?

---

## 35. Candidate seed research set

### Codex

Focus:

- co-change;
- generated artifacts;
- release/version coherence;
- protocol/client coupling;
- Cargo/pnpm cross-surface relationships.

Fieldwork entry point: <https://github.com/teamleaderleo/fieldwork/issues/413>.

### Oxc

Focus:

- source-authored versus generated files;
- generator commands;
- inline test precedent;
- rule registration;
- sibling-rule cohorts.

Fieldwork entry point: <https://github.com/teamleaderleo/fieldwork/issues/601>.

### WGPU

Focus:

- lock-order precedent;
- existing ranked-lock assertions;
- compare inferred order against explicit validation.

Fieldwork entry point: <https://github.com/teamleaderleo/fieldwork/issues/658>.

### uv

Focus:

- command-local diagnostic precedent;
- error context;
- hint framework;
- snapshot/test location;
- sibling command behavior.

Fieldwork entry point: <https://github.com/teamleaderleo/fieldwork/issues/627>.

### Tantivy

Focus:

- MSRV contract;
- workspace dependency coherence;
- lockfile policy;
- CI certification.

Fieldwork entry point: <https://github.com/teamleaderleo/fieldwork/issues/200>.

### Tauri

Focus:

- lock/callback sequencing;
- snapshot-before-caller-code precedent;
- pending queue mechanisms;
- panic cleanup ordering;
- feature forwarding and integration tests.

Fieldwork entry point: <https://github.com/teamleaderleo/fieldwork/issues/118>.

### Biome

Focus:

- rule family precedent;
- safe/unsafe fix classification;
- test location;
- transformation helper reuse;
- generated registration.

Fieldwork entry point: <https://github.com/teamleaderleo/fieldwork/issues/89>.

### SWC / Bevy / Jujutsu / additional Rust targets

Use these to discover new relationship species after the first set has exposed the limits of our vocabulary.

---

## 36. Research record format

A future research corpus could record each candidate as something like:

```yaml
id: wgpu-lock-order
repository: gfx-rs/wgpu
fieldwork_issue: 658
question: Can local lock order be recovered from source precedent?

human_learned_precedent:
  - command_indices before pending_writes

required_facts:
  - lock acquisition identity
  - acquisition order
  - owner function/type
  - explicit ranked-lock tests

relationship:
  kind: acquired-before

cohort:
  same_owner_type: Queue
  same_subsystem: ray-tracing queue paths

useful_finding:
  first reverse acquisition in comparable cohort

false_positive_threats:
  - aliases
  - intentionally independent locks
  - try_lock paths
  - test-only code

status: research
```

A corpus like this would keep research cumulative.

---

## 37. Evaluation goals

Early evaluation should optimize for **review usefulness and precision**.

Questions to measure:

- Did the finding identify a real repository custom?
- Was the comparison cohort credible?
- Did the evidence packet contain enough context to act?
- Did counterexamples appear honestly?
- Would a maintainer consider the question worth answering?
- Did the finding shorten repository archaeology?
- Did the analyzer stay quiet when precedent was weak?
- Could a reviewer reproduce the observation from the receipts?

Raw finding count is a poor success metric.

A handful of excellent findings per large diff may be the ideal product.

---

## 38. False-positive defenses

Cargo Cultist will live or die on restraint.

Rules worth keeping in mind:

1. **Rarity is a lead.** Comparison quality determines usefulness.
2. **Local precedent can beat repository-wide precedent.**
3. **Recent migrations can beat historical majority.**
4. **Explicit repository guidance can beat inferred precedent.**
5. **Counterexamples belong in the packet.**
6. **Stable exception families deserve recognition.**
7. **Generated/vendor/test-fixture code may need separate cohorts.**
8. **A helper's existence does not prove applicability.** Call-site similarity and local usage are evidence.
9. **Monorepos need subsystem-aware cohorts.**
10. **Large mechanical commits should rarely teach semantic precedent.**
11. **Weak precedent should produce silence or a low-key observation.**
12. **A finding asks for judgment; it does not manufacture intent.**

---

## 39. Potential output severity / disposition vocabulary

Cargo Cultist may eventually want vocabulary separate from lint severity.

Ideas:

- observation;
- weak precedent;
- strong precedent;
- first reversal;
- unique exception;
- missing companion;
- contract tension;
- historical drift;
- explicit-rule mismatch;
- unknown / insufficient cohort.

These describe evidence instead of moralizing about code.

---

## 40. A repository culture map

A compelling future command could summarize learned customs without a diff.

Example:

```text
cargo cultist precedents

TEST MODULES
  crate A: `tests` 92%
  crate B: `unit_tests` 88%

GENERATED PAIRS
  protocol/schema.rs -> generated/schema.json 31/34 comparable commits

LOCK ORDER
  command_indices -> pending_writes 14 observed sites

TEST PLACEMENT
  unicorn rules: inline Tester 52/54

ERROR CONVENTIONS
  tool commands: inventory failures gain operation context 8/9
```

This could become useful for onboarding, code review, and repository archaeology even when no finding fires.

The product fantasy here is: **show me how this repository tends to think.**

---

## 41. Historical drift map

Another future surface:

```text
cargo cultist history <mechanism>
```

It could show:

```text
2021-2023  local ad-hoc implementation common
2024-02    helper H introduced
2024-03    migration converted 18 call sites
2024-present  23/24 new call sites use H
```

A new ad-hoc implementation can then be compared with the current era instead of all-time frequency.

---

## 42. Reviewer mode

`cargo cultist diff` should probably remain the primary reviewer experience.

A good reviewer finding should be compact first, expandable second.

Compact:

```text
FINDING 2: generated-artifact precedent
  protocol/src/schema.rs changed; generated/schema.json did not.

FACTS
  31/34 comparable historical changes moved these together.

QUESTION
  Is generation intentionally deferred?
```

Expanded `--why`:

- cohort definition;
- all 34 commits;
- excluded commits and reasons;
- generator markers;
- counterexamples;
- path history;
- relevant repository docs.

---

## 43. Self-dogfooding

Cargo Cultist should inspect Cargo Cultist.

As checks land, the repository itself can become a tiny known corpus:

- test module conventions;
- source/test coupling;
- command/help test coupling;
- check registration;
- finding renderer conventions;
- feature/dependency conventions;
- eventual generated output.

Dogfooding helps detect whether a new detector requires special cases to survive its own repository.

---

## 44. First implementation sequence

A candidate sequence, subject to research results:

### Slice 0 — current prototype

Keep test-module precedent healthy and use it to establish reusable fact/finding interfaces only when the next check needs them.

### Slice 1 — Git co-change explorer

Build a read-only command that can answer:

```text
Given changed path A, which paths most often changed with it in comparable history?
```

Start as an exploratory report before deciding finding thresholds.

Goals:

- parse Git history deterministically;
- filter obvious noise;
- count co-change relationships;
- show exemplars and counterexamples;
- scope by directory/crate;
- work in diff mode.

### Slice 2 — generated companion experiment

Use Oxc or Codex to test whether co-change plus explicit generator evidence can produce one high-quality missing-companion finding.

### Slice 3 — test coupling / exact test selection

Use Fieldwork #225 cases to connect test declarations with workflow commands and detect zero-selection greens where syntax permits deterministic proof.

### Slice 4 — lock-order experiment

Use WGPU as a research target. Keep the extractor narrow and compare it against the repository's ranked-lock knowledge.

### Slice 5 — mechanism reuse / diagnostic precedent

Use uv/Oxc/Tauri to test whether sibling cohorts can identify repository-native mechanisms.

This sequence deliberately proves increasingly semantic relationship types.

---

## 45. The co-change explorer should begin as research instrumentation

Before turning historical coupling into findings, expose the raw data.

Possible command:

```text
cargo cultist history path/to/file.rs
```

Output:

```text
HISTORICAL COMPANIONS
  generated/foo.rs          31 / 34 comparable commits
  tests/foo.rs              28 / 34
  Cargo.toml                 4 / 34

EXCLUDED
  3 merge commits
  2 repository-wide formatting commits

COUNTEREXAMPLES FOR generated/foo.rs
  abc123  revert
  def456  generator migration
  789abc  ordinary change
```

This lets us study the corpus before encoding policy.

---

## 46. Relationship discovery can be incremental

There is no requirement to build a universal relation engine up front.

A practical path:

1. implement one detector with explicit facts;
2. notice common fact/relationship needs;
3. extract shared types;
4. repeat;
5. let the data model emerge from useful checks.

The relational model is the conceptual compass. The code can remain small.

---

## 47. Potential data persistence

Large repositories and Git history will eventually make recomputation expensive.

Possible later cache keys:

- repository root identity;
- HEAD commit;
- provider version;
- file blob SHA;
- commit SHA;
- Cargo metadata fingerprint.

A local cache could store deterministic facts and relationships. SQLite is one possible future option; the first experiments can stay in memory and keep dependencies small.

Caching should never blur provenance. Every result still needs to identify the source generation that produced it.

---

## 48. Security and trust angle

Repository precedent can surface authority mistakes that local type checking misses.

Examples from Fieldwork include:

- allegedly isolated clients inheriting mutable global credentials/config;
- cache identity omitting a plugin key;
- stale catalog/runtime generations being mixed;
- cleanup acting without clear ownership;
- timeout treated as remote completion.

These may eventually inspire relationship checks around ownership and identity.

This area needs especially strong cohorts and evidence because intent is subtle.

---

## 49. The deepest long-term idea: repository grammar

A mature repository develops a grammar of how changes are made.

Examples:

```text
new rule
-> source implementation
-> registration
-> inline test
-> generated metadata
-> timing update

protocol change
-> source schema
-> generated schema
-> client generation
-> compatibility test

callback dispatch
-> snapshot shared state
-> drop guard
-> invoke caller code

worker replacement
-> settle old generation
-> publish replacement
```

Cargo Cultist could learn fragments of this grammar from source, history, tests, and workflows.

A diff then becomes a partially observed sentence. The tool asks which expected relationships are present, absent, reversed, or exceptional.

That is a much richer target than style enforcement.

---

## 50. A compact definition of the project

Possible internal definition:

> Cargo Cultist is a repository-aware analyzer that extracts deterministic facts, learns evidence-backed local precedent, and surfaces changed relationships that deserve explanation.

Possible stronger formulation:

> Cargo Cultist mines the customs a repository has accumulated in code and history, then shows reviewers where a change follows, bends, or breaks those customs—with receipts.

The second captures the ambition well.

---

## 51. Questions to keep asking as this grows

- What exact repository fact are we extracting?
- What relationship does it participate in?
- What makes two examples comparable?
- What are the strongest counterexamples?
- Is the precedent current or historical residue?
- Does an explicit repository rule exist?
- Does the exception form a coherent local family?
- Can the tool show representative exemplars?
- Can a reviewer reproduce the observation?
- Would this finding have shortened a real investigation?
- Is the tool discovering repository knowledge, or smuggling in a universal style preference?
- Can we keep the evidence deterministic before interpretation?

---

## 52. Immediate research backlog

1. Build a small Fieldwork-derived corpus of tacit repository conventions.
2. Start with Codex, Oxc, WGPU, uv, Tantivy, Tauri, and Biome.
3. For each case, record the human-learned precedent, required facts, relationship kind, cohort, counterexamples, and candidate detector.
4. Implement a raw Git co-change explorer before defining finding thresholds.
5. Replay the explorer against Codex and Oxc history.
6. Test whether explicit generated markers plus co-change produce a strong missing-companion finding.
7. Inventory deterministic test names versus focused CI filters for one Fieldwork #225-style zero-test case.
8. Prototype a narrow lock-order fact extractor and compare it with WGPU's explicit ranked-lock evidence.
9. Keep every experiment read-only against target repositories.
10. Preserve negative results; failed relationship hypotheses are valuable for refining cohort selection.
11. Dogfood every landed detector on Cargo Cultist itself.
12. Add machine-readable finding output once more than one detector needs the common evidence packet.

---

## 53. Why this feels unusually promising

Fieldwork has already spent substantial effort learning the hidden operating customs of many serious repositories. Those investigations repeatedly show that consequential review questions live between local syntax and full open-ended AI interpretation.

Cargo Cultist has a plausible way to occupy that space:

- deterministic extraction;
- repository-specific precedent;
- relational analysis;
- historical evidence;
- explicit counterexamples;
- restrained questions;
- optional interpretation after the receipts exist.

The current test-module check is the smallest possible proof of the loop.

Historical co-change can prove the relational thesis.

Generated companion analysis can prove practical reviewer value.

Lock-order precedent can prove that mined repository customs can reach into correctness and concurrency.

If those experiments work, Cargo Cultist becomes something distinctly different from a fuzzy linter: a tool for recovering and interrogating the unwritten grammar of a codebase.

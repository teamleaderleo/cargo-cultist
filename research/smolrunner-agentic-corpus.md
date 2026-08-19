# SmolRunner agentic-history corpus

Status: research seed for cargo-cultist#41. This is a corpus, not product policy.

`teamleaderleo/smolrunner` is a useful complement to Stensibly because many of its recent changes preserve an unusually explicit causal packet inside the repository itself:

```text
prior assumption / active contract
-> physical or adversarial observation
-> concrete failure
-> repair
-> promoted test / invariant / budget / identity rule
```

The aim of this note is to test how much of that lineage Cargo Cultist can recover from Git, PR, issue, test, and policy history without private chat transcripts.

## Evidence-edge vocabulary

Use these edge classes while replaying the corpus:

- **EXPLICIT** — an artifact directly names the triggering run, failure, predecessor, or causal reason.
- **DERIVED** — the relation follows deterministically from source/diff/test evidence, while the artifact does not state the causal sentence itself.
- **ADJACENT** — chronology, touched files, or vocabulary make a relationship worth inspecting, but repository evidence does not establish causality.
- **UNKNOWN** — the repository does not preserve enough evidence to recover the relation.

Cargo Cultist should preserve these classes instead of flattening every nearby change into one causal chain.

## Initial SmolRunner episodes

| Episode | Repository evidence | Recoverable lineage | Candidate species | Edge quality |
| --- | --- | --- | --- | --- |
| Clone checkpoint adjacency | `teamleaderleo/smolrunner#530` | A live Quarry pilot published `CloneStarted`; a transient Scale Set poll then failed before `limactl` ran, leaving unbound durable recovery debt. The repair moves every fallible pre-command gate before publication and carries successful preflight through a single-use token so the clone command is the first external operation after publication. | **authority/action adjacency**; durable publication should sit immediately beside the operation that consumes the authority when intermediate failure creates unrecoverable ambiguity | **EXPLICIT** |
| Redundant clone admission polls | `teamleaderleo/smolrunner#533` | Physical run `31957197739` reached `CloneAuthorized` without starting a VM; serial official long polls consumed the assignment window and GitHub requeued the job. The repair removes superseded polls while retaining one exact final under-lock authority poll. | **verification accretion / control-budget regression** | **EXPLICIT** |
| Registration admission latency | `teamleaderleo/smolrunner#538` | Physical run `31959672775` proved clone/boot/cleanup while serial 75-second polls plus readiness exhausted the assignment lease before JIT. The repair lets clone execution and registration own their final live poll and moves expensive readiness ahead of the final admission sample. | **freshness placement**; expensive proof can make later authority evidence stale before it is consumed | **EXPLICIT** |
| Duplicate readiness composites | `teamleaderleo/smolrunner#540` | Complete observations were repeated with no intervening mutation. The repair removes adjacent duplicates and promotes command counts into exact regressions: authorization 3 subprocesses, registration 34, runner transaction 171; combined measured path 243 -> 208. | **verification accretion -> executable budget**; a learned performance constraint becomes a regression-tested contract | **EXPLICIT** |
| Runner workspace canonical identity | `teamleaderleo/smolrunner#539` | Physical run `31962242919` reached the JIT runner, then private `actions/checkout` failed because `_work` was a symlink: Git resolved the repository under one canonical path while checkout scoped credentials to another. The repair restores the official runner's real `_work` / `_diag` directories and narrows the writable integrity exception. | **local equivalence vs consumer identity semantics**; two paths that look interchangeable locally can differ to a downstream authorization mechanism | **EXPLICIT** |
| JIT secret survives `exec` | `teamleaderleo/smolrunner#549` | One-time JIT config remained in the long-lived `Runner.Listener` environment across `exec`, so same-UID workflow code could read `/proc/<pid>/environ`. The repair makes the listener non-dumpable through a guarded setgid transition and verifies the OS prerequisites. | **secret lifetime drift**; an ephemeral authority survives a process transition longer than its intended lifetime | **EXPLICIT** |
| Guest-mutable evidence used for teardown authority | `teamleaderleo/smolrunner#550` | Cleanup identity included root-disk GPT evidence that privileged guest code could mutate, letting the object being destroyed veto its own teardown. The repair derives cleanup ownership from host-controlled VZ identity while keeping descriptor and size checks and a bounded legacy match path. | **proxy/authority drift**; evidence mutable by the subject gains authority over destructive ownership decisions | **EXPLICIT** |
| Read-only Git observation can execute code | `teamleaderleo/smolrunner#551` | `git status --porcelain=v2` can run repository-configured clean/process filters. The checkout observer therefore had a command-execution sink inside a read-only, credentialless observation path. The repair probes included filter config first and refuses status when risky filters exist. | **effectful observation**; an operation classified as observational carries hidden extension points with execution authority | **EXPLICIT** |
| Runner readiness bound to path instead of bytes | `teamleaderleo/smolrunner#552` | A hostile workflow could replace runner executables or credential files while retaining the expected root/path identity and still satisfy readiness checks. The repair binds reviewed installation files and observed `/proc/<pid>/exe` bytes to SHA-256 identities. | **identity-evidence completeness**; pathname/process placement is correlated evidence until immutable content identity is required | **EXPLICIT** |
| Reviewed wrapper sourced from hostile workspace | `teamleaderleo/smolrunner#553` | The Renderprove wrapper executable was derived from the disposable repository workspace even though the trust model required it to come from a separate operator-reviewed checkout. The repair changes the source path and adds a regression proving the wrapper path is disjoint from the disposable workspace. | **trust-domain aliasing / provenance loss**; a value with the right role is sourced from the wrong authority domain | **EXPLICIT** |
| Documentation moves build code across privilege boundary | `teamleaderleo/smolrunner#554` | Repository instructions used `sudo cargo run`, allowing build scripts and procedural macros to execute as root before SmolRunner's confirmation/journal controls. The repair changes the documented journey to unprivileged build followed by elevation of the reviewed binary only. | **instruction-level authority widening**; repository guidance can create a security-relevant execution path even when product code is correct | **EXPLICIT** |
| Public proposal failed to bind hidden command semantics | `teamleaderleo/smolrunner#555` | Host-preparation confirmation serialized the redacted public proposal while exact root-command arguments lived in a retained hidden `DurableLanePlan`. Two publicly identical proposals could therefore authorize different privileged commands. The repair hashes the exact durable plan into a versioned confirmation binding while keeping public output redacted. | **approval/projection mismatch**; the artifact a human approves omits execution-significant semantics | **EXPLICIT** |

## A useful multi-change lineage

The `#533 -> #538 -> #540` sequence is especially valuable because it shows a lesson becoming progressively more general:

```text
physical assignment timeout
-> remove superseded long polls
-> discover another stale outer poll + expensive readiness placement
-> move final authority observation closer to use
-> find adjacent duplicate observations
-> promote subprocess counts into explicit budgets
```

Every local change is defensible as a narrow repair. The historical sequence exposes a broader lesson: **verification itself consumes a bounded external control window**.

Cargo Cultist should test whether that broader lesson can be recovered without embedding SmolRunner-specific vocabulary. Candidate deterministic signals include repeated removal of observation calls, PR bodies naming timeout/latency, later introduction of exact command-count assertions, and shared touched transaction surfaces.

## Cross-corpus hypotheses with Stensibly

### 1. Proxy fact becomes authority

Stensibly `teamleaderleo/stensibly#1605` corrects a positive `claimGeneration` counter being treated as proof of a live responsibility even though the counter also advances on unclaimed transitions.

SmolRunner has several independent forms:

- `#550`: guest-mutable GPT evidence participated in cleanup ownership authority;
- `#552`: stable path/root identity participated in readiness authority without immutable file identity;
- `#555`: a public projection participated in privileged confirmation authority while hidden command semantics remained outside the binding.

Candidate generalized question:

> Which facts are correlated with the authoritative state, and which exact facts are actually sufficient to authorize the later operation?

This looks promising for evidence packets even before a broad analyzer exists.

### 2. Control optimization changes correctness semantics

SmolRunner `#533/#538/#540` gives an explicit causal lineage: defensive observations accumulated until they violated GitHub's bounded assignment window, then the project promoted observation placement/count into a performance contract.

Stensibly `#1583` and `#1598` optimize CI admission and evidence reuse; `#1617` later reports that a workflow-admission change could let metadata-triggered skipped jobs satisfy required contexts and admit an unvalidated merge.

The family is valuable, but the exact Stensibly edge needs care. `#1617` says a workflow-admission change caused the weakness, yet its PR body does not explicitly identify `#1583` or `#1598`. Cargo Cultist may surface those earlier PRs as candidate ancestry from file history and semantics; it should classify the exact causal link as **ADJACENT/INFERRED** until stronger repository evidence connects them.

That is a useful negative result: chronology plus semantic similarity is insufficient for a `caused-by` edge.

### 3. Proof surface mismatch

Stensibly `#1515` was excluded from acceptance because the verifier produced a PR review comment while the evidence contract required an ordinary conversation comment. The stale-head behavior itself worked; the artifact type invalidated the proof.

SmolRunner supplies related variants:

- `#539`: filesystem paths were locally equivalent but differed under checkout's canonical path-scoped credential semantics;
- `#555`: public proposal bytes looked equivalent while exact privileged command semantics differed.

Candidate generalized relation:

```text
locally equivalent representation
-> external consumer distinguishes semantic identity
-> proof / authorization / credential contract fails
```

This appears highly recoverable when the artifact type/path/binding is explicit.

### 4. Unknown authority should fail closed

Stensibly `#1624` and draft `#1629` both address project-scoped authorization when item-project resolution throws or remains unresolved. The project allowlist requires authoritative project identity before a write can proceed.

SmolRunner repeatedly encodes the same principle around durable identity, VM ownership, runner readiness, and exact plan confirmation. The common species is less interesting as a universal "fail closed" slogan; the useful research question is narrower:

> Which lookup or observation supplies the authority-bearing identity, and what does this repository do when that evidence becomes unavailable or ambiguous?

This belongs beside project-memory and sibling-path analysis because the expected behavior is repository-specific.

## Deliberately rejected / deferred causal claims

### Do not claim that `#1583 -> #1598 -> #1617` is proven causality yet

The chronology and subsystem overlap are compelling, and `#1617` explicitly attributes the weakness to a workflow-admission change. The artifacts inspected here do not directly name `#1583` or `#1598` as the cause.

A deterministic lineage explorer should therefore emit something like:

```text
EXPLICIT
  #1617 attributes the weakness to an earlier workflow-admission change.

CANDIDATE ANCESTRY
  #1583 and #1598 are earlier workflow-admission/evidence-reuse changes on the same control surface.

UNKNOWN
  Repository evidence inspected so far does not prove which earlier change introduced the vulnerable semantics.
```

This is exactly the kind of restraint Cargo Cultist needs if it is going to do software archaeology without inventing stories.

## Deterministic experiments suggested by this corpus

### A. Change-lineage edge extraction

For a PR/commit/issue, retain distinct edge types:

- explicit `Refs` / `Closes` / named run / named predecessor;
- exact branch ancestry;
- same-file history;
- shared test or invariant introduction;
- temporal adjacency.

Never silently promote the last category into causality.

### B. Lesson-promotion detector

Look for a sequence where a failure vocabulary appears in an issue/PR and a later change adds a broader executable guard:

```text
failure receipt
-> one-off repair
-> shared check / invariant / budget / CI gate
```

SmolRunner `#533/#538/#540` is a strong fixture because the promotion ends in exact subprocess-budget assertions.

### C. Authority-input packet

Given a destructive, privileged, readiness, or authorization decision, collect the exact facts participating in the decision and annotate their provenance/mutability:

```text
fact
source domain
mutable by subject?
public projection?
exact execution input?
used by equality/hash/binding?
used to authorize action?
```

`#550`, `#552`, `#553`, and `#555` provide four independent positive fixtures.

### D. Effectful-observer inventory

Find commands or APIs used inside code described as read-only/observation/probe/status, then inspect repository history for added preflights, disabled extension points, empty environments, hook suppression, helper suppression, or sandboxing.

`#551` is a crisp fixture because the regression explicitly asserts that `git status` is never invoked once an executable filter is detected.

## What this corpus can test

SmolRunner should be used as a **cross-corpus falsification target** after extracting patterns from Stensibly.

A useful species should survive changes in repository vocabulary and engineering style. A Stensibly-only detector that depends on ledger terminology, activity records, or that project's operating protocol should weaken or fail here. Conversely, patterns such as proxy-to-authority drift, proof-surface mismatch, effectful observation, trust-domain provenance, and lesson promotion now have materially different examples in both corpora.

The goal is to keep the evidence packets even when an analyzer idea fails. Negative results define Cargo Cultist's boundary as clearly as successful detectors.

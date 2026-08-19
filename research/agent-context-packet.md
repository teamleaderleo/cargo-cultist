# Agent context packet: bounded repository evidence before an edit

Date: 2026-08-19

Status: research design for #62, composed from #12, #18, #38, #41, and the existing precedent/history work. This note defines a consumer contract; it does not promote new repository rules.

## Question

Can Cargo Cultist give a coding agent a small, deterministic packet of repository evidence that materially improves the agent's next action without sending the repository to a model or inventing missing rationale?

The target interaction is deliberately simple:

```text
agent intends to edit target T
-> cargo cultist brief T --format json
-> bounded evidence packet
-> agent decides what to inspect/change next
```

The packet answers:

> What repository evidence would an agent regret missing before it edits this target?

That is different from asking Cultist to write the patch or review the whole repository.

## Why agents are a strong first consumer

Agents are unusually sensitive to repository precedent because they are good at reproducing local patterns once those patterns are visible. That is useful when the pattern was earned and dangerous when the local pattern is accidental, stale, or only valid in a narrower context.

The packet should therefore optimize for four things:

1. expose the strongest relevant evidence;
2. expose the strongest counterevidence;
3. preserve where every claim came from;
4. say `UNKNOWN` when repository artifacts cannot recover the answer.

The product value is not "more context." The product value is **better selected context with receipts**.

## Candidate command

```text
cargo cultist brief path/to/file.rs
cargo cultist brief --max-history 20 --max-bytes 32768 path/to/file.rs
cargo cultist brief --format json path/to/file.rs
```

`brief` is intentionally a view over existing and planned evidence primitives. It should not grow its own analyzer taxonomy.

## v0 scope

Keep the first slice local, deterministic, and read-only.

For one existing file target:

1. resolve repository-relative target identity;
2. discover explicit guidance files applicable by path scope;
3. collect a bounded recent Git history for the target;
4. attach the existing historical-companion packet;
5. attach current deterministic findings whose location/scope intersects the target;
6. emit explicit unknowns for evidence classes unavailable in local-only mode;
7. record every truncation caused by the packet budget.

Remote PR, issue, and review enrichment belongs to #18 and remains optional.

## Packet contract

A possible machine-readable envelope:

```json
{
  "schema_version": 1,
  "analysis": "agent_context",
  "repository": "/repo",
  "target": {
    "path": "src/foo.rs"
  },
  "budget": {
    "max_bytes": 32768,
    "max_history_commits": 20,
    "max_examples_per_relation": 3
  },
  "guidance": [],
  "direct_evidence": [],
  "recent_history": [],
  "historical_companions": [],
  "findings": [],
  "counterevidence": [],
  "unknowns": [],
  "truncation": []
}
```

The exact schema can change during research. The important property is that every claim remains typed and attributable.

## Evidence item

A common evidence item should carry enough information for an agent to re-open the original artifact instead of trusting a summary blindly.

Candidate fields:

```json
{
  "claim_kind": "observed",
  "message": "99/99 relevant edits also changed generated/rules_enum.rs",
  "scope": "history:target-file",
  "source": {
    "kind": "git_history",
    "repository": "owner/repo",
    "commit": "abc123...",
    "path": "src/rules.rs"
  }
}
```

For explicit guidance:

```json
{
  "claim_kind": "proven",
  "message": "The applicable AGENTS.md requires generator-owned files to be updated through cargo lintgen.",
  "scope": "path:crates/linter",
  "source": {
    "kind": "repository_guidance",
    "path": "AGENTS.md",
    "line": 42
  }
}
```

If natural-language guidance extraction is not deterministic enough, v0 can surface the applicable guidance file and relevant exact machine-readable markers while leaving interpretation `UNKNOWN`.

## Guidance scope

Agents often operate under path-scoped instructions. Cultist should preserve that scope explicitly instead of flattening every guidance file into one repository-wide rule set.

A conservative first resolver can:

1. identify known guidance filenames such as `AGENTS.md` and `CONTRIBUTING.md`;
2. walk from repository root toward the target directory;
3. record each discovered guidance file and its directory scope;
4. let narrower guidance and broader guidance coexist as separate evidence;
5. report conflicts instead of silently selecting a winner.

The first version does not need general prose understanding to make this useful. Merely telling an agent which guidance artifacts apply to its target is valuable and deterministic.

## Ordering and budget

A bounded packet needs deterministic eviction rules.

Candidate priority order:

1. explicit path-scoped guidance;
2. direct proven/derived evidence about the target;
3. current findings and counterexamples;
4. recent target history with explicit commit messages;
5. strong historical companions with counterexamples;
6. broader repository precedent;
7. optional remote project memory;
8. inferred/model-assisted explanation.

Within a class, use stable path/date/identity ordering.

When the budget is exhausted, emit a receipt:

```text
TRUNCATED
  17 additional historical companion examples omitted by max-bytes=32768.
  Re-run with --max-bytes or query `history` directly for the full packet.
```

Silent truncation is unacceptable for an agent-facing interface because absence could otherwise be mistaken for negative evidence.

## Counterevidence is first-class

A packet should never say only:

```text
99/100 commits changed A with B
```

It should preserve the one absence case when available:

```text
COUNTEREXAMPLE
  5e113baf touched A without B; the change was documentation-only.
```

This is one of the central protections against agent cargo culting. An agent sees both the custom and the known exception family before copying it.

## `UNKNOWN` is an action signal

`UNKNOWN` should tell the consuming agent what repository evidence failed to answer.

Examples:

```text
UNKNOWN
  No local artifact explains why this exception exists.

UNKNOWN
  Current local Git history does not establish whether PR #123 caused the later repair.

UNKNOWN
  A remote review discussion may contain rationale, but remote project-memory enrichment is disabled.
```

For an agent, that is useful because it can turn the next action from "edit" into "investigate this missing link."

## Provenance / trust-domain evidence

The SmolRunner corpus suggests a reusable family where values that look locally equivalent differ because their origin carries authority semantics.

Examples:

- `smolrunner#553`: wrapper path from the disposable workspace versus the disjoint operator-reviewed Renderprove checkout;
- `#539`: symlinked runner work path versus the canonical path expected by `actions/checkout` credential scoping;
- `#555`: public proposal bytes versus the hidden exact root command plan being authorized;
- `#550`: guest-mutable disk evidence versus host-controlled cleanup ownership identity;
- `#552`: path/inode process identity versus immutable executable/credential bytes.

This argues for preserving source/origin in evidence items whenever Cultist knows it. A packet that reduces both values to the same string or type would erase the distinction the repository learned was important.

## First replay: SmolRunner clone runtime

Target:

```text
teamleaderleo/smolrunner/src/disposable_clone_runtime.rs
```

A useful local-first packet should recover at least:

### Explicit guidance

- root repository operating guidance applies to the target;
- unknown-state and destructive-operation policies remain visible as source artifacts even before natural-language interpretation is automated.

### Recent history

The target's recent history should expose the sequence around:

```text
#530  keep clone preflight before durable checkpoint
#533  collapse redundant clone admission polls
#538  reduce live admission latency before JIT
#540  remove duplicate worker readiness probes
```

A fresh agent should be able to see from commit messages alone that this area contains earned ordering and latency constraints before modifying observation placement.

### Historical companions

The packet should show which files routinely move with the clone runtime and retain absence counterexamples. This can tell the agent where tests/docs/coordinator logic are commonly coupled without declaring them mandatory.

### Unknown

Local Git alone may not contain the physical-run details preserved in PR bodies. The packet should say so explicitly instead of paraphrasing those details from nowhere.

Later #18 enrichment can attach the PR bodies and upgrade those edges from local-history adjacency to explicit project-memory evidence.

## Second replay: SmolRunner host preparation confirmation

Target:

```text
teamleaderleo/smolrunner/src/host_preparation_command.rs
```

The desired agent-visible lesson is not simply "hash this object." The earned rule is closer to:

```text
approval identity must cover the exact hidden execution semantics it authorizes
```

A useful packet should expose:

- the commit introducing the confirmation binding repair;
- companion movement with the durable plan definition/tests;
- any explicit repository guidance governing privileged mutation;
- `UNKNOWN` for causal claims not available locally.

This is a good discriminator for whether the packet selects relevant history or merely dumps the last N commits.

## Stensibly cross-corpus controls

Issue #41 provides independent cases for the same packet design:

- proxy fact vs authoritative fact (`claimGeneration` vs live responsibility evidence);
- proof-surface mismatch (PR review comment vs ordinary conversation comment);
- duplicate lanes and canonical-carrier selection;
- policy reversal after agent/operator friction;
- bounded reconciliation of two observations with different freshness/authority.

The goal is to keep packet semantics reusable across repositories with very different implementation domains.

## Negative control: chronology is not causality

The packet must preserve a tempting but unsupported lineage as uncertainty.

Stensibly gives a good case:

```text
#1583 metadata-CI optimization
#1598 byte-identical CI reuse
#1617 metadata-run skipped-check repair
```

The later PR says an earlier workflow-admission change caused the vulnerability, but repository evidence inspected so far does not identify #1583 or #1598 as that causal ancestor.

A correct packet may show those artifacts as nearby/relevant history. It must not emit:

```text
PROVEN: #1583 caused #1617
```

That is exactly the kind of plausible story an agent can overlearn.

## Agent evaluation questions

For each replay, give a fresh agent only:

1. the requested task;
2. the repository checkout;
3. either the baseline repository or the same repository plus the Cultist packet.

Then measure inspectable outcomes:

- did the agent open the earned invariant before editing?
- did it discover relevant companion files earlier?
- did it avoid repeating a known failed approach?
- did it distinguish explicit guidance from observed frequency?
- did it inspect a counterexample before applying a dominant convention?
- did it identify an important `UNKNOWN` and investigate it?
- how many packet items were never useful?

Avoid collapsing this into one opaque score. Retain the actual decisions and evidence accesses.

## Useful efficiency metric

A practical agent-facing objective is:

> useful evidence consulted per packet byte / developer interruption

Large context windows make it easy to solve the wrong problem by shipping everything. Cargo Cultist should compete on selection quality and provenance, not context volume.

## Relationship to existing work

- #12 owns target-scoped `why` packets;
- #18 owns local/remote project memory;
- #38 owns explicit repository guidance;
- #6 owns counterexample-first evidence;
- #34 owns raw historical companions;
- #15/#22 own claim provenance and machine-readable output;
- #41 owns agentic-history research corpora;
- #62 owns the agent-facing bounded composition view.

The agent packet should consume those capabilities instead of duplicating them.

## Promotion criteria

Do not promote `brief` from research until:

1. the packet can be generated locally without an LLM;
2. output size has an explicit deterministic budget;
3. guidance and history provenance are preserved;
4. counterevidence and unknowns survive truncation policy;
5. one SmolRunner replay causes a fresh agent to inspect an earned invariant it otherwise misses;
6. one negative replay proves the packet refuses to manufacture causality;
7. the JSON packet remains useful to a non-agent human or tool consumer.

If those tests fail, keep the result as research rather than forcing an agent feature into the product.
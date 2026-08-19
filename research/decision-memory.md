# Decision memory: closing the after -> future-before loop

Date: 2026-08-19

Status: research slice for #75, composed from #10, #18, #62, and #74. The record format and location in this branch are explicitly provisional.

## Question

Can one reviewed repository decision become deterministic project memory that a later agent can recover for the exact target it applies to?

The lifecycle is:

```text
repository question / finding
-> human-reviewed decision
-> rationale stored in version control
-> later agent targets related code
-> decision resolver returns the applicable rationale + authority references
```

The key product property is continuity. Cargo Cultist should not need a second private memory database for the after-code lesson to become before-code context.

## Why this is separate from suppression

A decision record is evidence first.

It may explain why a finding is intentional, but merely resolving a record must not automatically:

- suppress a future finding;
- prove that the decision still applies;
- widen the decision to neighboring code;
- turn one maintainer choice into repository-wide policy.

Those are later explicit operations with their own evidence and authority.

## Research storage

This branch uses:

```text
research/decision-memory/*.json
```

That location is intentionally research-only. It proves record semantics without claiming a final `.cargo-cultist` layout or committing #10 to JSON rather than TOML/another reviewed format.

One record per file has useful properties worth testing:

- independent Git history for each decision;
- reduced merge contention for parallel agents;
- simple stable identity;
- easy code review and deletion;
- no need to rewrite a monolithic registry for every new rationale.

## v0 record

The first record captures an existing Cargo Cultist decision already present in #34, PR #39, and #19:

```json
{
  "schema_version": 1,
  "id": "history-cochange-remains-association-v1",
  "kind": "historical-companion-policy",
  "scope": {
    "path_prefix": "src/history.rs"
  },
  "reason": "Raw historical co-change support is association evidence. Frequency alone does not establish that a companion is required for a future change, so examples, counterexamples, cohort exclusions, and stronger evidence layers remain visible before promotion.",
  "authority": [
    {"kind": "issue", "reference": "#34"},
    {"kind": "pull_request", "reference": "#39"},
    {"kind": "roadmap", "reference": "#19"}
  ]
}
```

This is useful as a self-dogfood decision because it directly protects Cargo Cultist from cargo-culting its own historical percentages.

## Resolver

`examples/decision_memory.rs` is a standalone read-only probe:

```text
cargo run --example decision_memory -- RECORDS_DIR TARGET
```

It:

1. resolves the target inside the current Git repository;
2. reads sorted JSON decision files from the explicitly supplied research directory;
3. validates schema and required fields;
4. treats absolute scopes as invalid;
5. matches `path_prefix` using path-component semantics rather than string prefixing;
6. returns source file, full decision record, rationale, and authority references;
7. fails closed on malformed or unsupported records.

It does not modify the repository or suppress analyzer output.

## First discriminator

For:

```text
TARGET=src/history.rs
```

expected:

```text
1 applicable decision
id = history-cochange-remains-association-v1
```

For:

```text
TARGET=src/main.rs
```

expected:

```text
0 applicable decisions
```

For a record with:

```json
{"schema_version": 99, ...}
```

expected:

```text
hard refusal: unsupported decision schema
```

CI executes all three cases.

## Authority model

This prototype deliberately separates **record content** from **record authority**.

A file on an unmerged agent branch is only a proposal. It becomes accepted repository memory through the ordinary repository authority path: review/merge or another explicit maintainer action.

That means an agent can draft:

```text
reason + scope + evidence references
```

without silently teaching the project anything.

The Git event that accepts the record is part of its provenance.

A later product implementation should consider exposing that acceptance identity directly rather than relying only on the record's self-declared authority list.

## Why authority references remain plural

The first fixture cites #34, #39, and #19 separately because they provide different evidence:

- #34 owns the raw history experiment contract;
- #39 is the implementation/replay carrier;
- #19 records the broader project rule that raw evidence remains exploratory until promoted.

Collapsing those into one prose summary would lose useful ancestry.

## Scope semantics

The current `path_prefix` model is intentionally narrow.

Path-component matching means:

```text
scope: src/history.rs
```

matches:

```text
src/history.rs
```

and does not match:

```text
src/history.rs.bak
```

A directory scope such as:

```text
src/history
```

can match descendants.

Future questions include:

- item/function scopes;
- finding-family scopes;
- package/workspace scopes;
- glob semantics;
- rename/refactor migration;
- expiry/conditions.

Do not add them until real decisions require them.

## Feeding #62 `brief`

The important composition is straightforward:

```text
brief TARGET
-> resolve applicable decision records
-> emit them as explicit project-memory evidence
-> keep observed history/precedent separate
```

Possible future packet section:

```json
{
  "decisions": [
    {
      "claim_kind": "proven",
      "record_id": "history-cochange-remains-association-v1",
      "scope": "src/history.rs",
      "reason": "...",
      "authority": ["#34", "#39", "#19"],
      "source_file": "..."
    }
  ]
}
```

`PROVEN` here would mean the reviewed record exists and applies by deterministic scope. It would not mean every assertion inside the rationale is formally proved.

That distinction should remain visible.

## Feeding the during-code phase

`diff` can use the same resolver to say:

```text
KNOWN DECISION
  This changed target has an applicable repository decision.
  Reason: raw co-change remains association evidence.
```

If the live change contradicts the decision, Cultist should surface tension rather than silently suppressing either side.

## Feeding #11 promotion

Repeated decisions may later justify a deterministic rule, but the record itself should preserve the intermediate history:

```text
finding
-> reviewed decision record
-> repeated reviewed decisions
-> explicit promotion event
-> lint / CI rule
```

The promoted rule should link back to the decision records/counterexamples that earned it.

## Agent lifecycle consequence

This is the missing closure in #74:

```text
BEFORE
  brief retrieves earned memory

DURING
  diff checks the live change against that memory

AFTER
  reviewed decision stores a newly earned lesson

NEXT TIME
  brief retrieves it automatically
```

An agent can therefore enter with no original chat transcript, work from repository evidence, and leave behind reviewable memory for its successor.

## Promotion gates

Keep this as research until:

1. at least two independent decision types need the same core fields;
2. scope semantics survive a real refactor/rename case;
3. brief/diff consume decisions without turning them into implicit suppressions;
4. accepted-vs-proposed authority can be represented cleanly;
5. invalid records fail closed with useful source identity;
6. one longitudinal agent replay shows agent B recovering a decision left by agent A.

If the record format fails these tests, replace it rather than preserving compatibility with a research fixture.

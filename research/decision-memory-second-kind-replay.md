# Decision-memory second-kind replay

Date: 2026-08-19

Status: successful second independent reviewed-decision kind for PR #94 / issue #75.

## Question

Are the research decision-memory core fields reusable across more than one reviewed repository decision family?

The first retained decision is a historical-evidence policy. This replay adds a materially different provenance implementation boundary without changing the record schema.

## Decision kind 1: historical companion policy

Existing reviewed record:

```text
id:    history-cochange-remains-association-v1
kind:  historical-companion-policy
scope: src/history.rs
refs:  #34, #39, #19
```

Its rationale keeps raw historical co-change as association evidence until stronger evidence earns promotion.

## Decision kind 2: provenance implementation boundary

New reviewed record:

```text
id:    generated-companion-canonical-provenance-v1
kind:  provenance-implementation-boundary
scope: src/generated_diff.rs
refs:  #80, #90, #82, #70
```

Its rationale preserves the generated-provenance lesson earned during #80/#90:

```text
src/generated_diff.rs
  -> consume canonical generator ownership from src/generator_ownership.rs
  -> do not maintain a second repository-root / path-I/O parser
```

The earlier product copy drifted from stricter provenance semantics and had to be contained until the canonical implementation was physically shared.

## Core schema remained unchanged

Both independent decisions use the same research fields:

```text
schema_version
id
kind
scope.path_prefix
reason
authority[]
```

No new field was added to accommodate the provenance-boundary decision.

## Exact semantic matrix

Executed head:

```text
98fa4930ea46757355d73a1612b7fd0b0219760e
```

Dedicated research run:

```text
run:    32224093880
job:    95980168089
result: success
```

Artifact:

```text
id:     9354926351
name:   decision-memory-second-kind
sha256: d8e8b4100c1f5ac0f0e69961d1f40c29eed79f70a630430e3a133db7cd46d9d2
```

Focused quality gates passed before the decision checks.

### Generated diff target

Target:

```text
src/generated_diff.rs
```

Resolved exactly one decision:

```text
generated-companion-canonical-provenance-v1
kind: provenance-implementation-boundary
matched_via: direct
matched_path: src/generated_diff.rs
```

Authority order was preserved:

```text
#80
#90
#82
#70
```

### Historical analyzer target

Target:

```text
src/history.rs
```

Resolved exactly the independent existing record:

```text
history-cochange-remains-association-v1
kind: historical-companion-policy
```

The two decision kinds remained distinct.

### Agent packet

Schema-4 `agent_context_packet` for:

```text
src/generated_diff.rs
```

surfaced exactly the new provenance-boundary decision with a direct match.

### Scope non-spill control

Agent packet for:

```text
src/generator_ownership.rs
```

resolved:

```text
reviewed_decisions: []
```

The decision applies to the consumer that must avoid duplicating provenance analysis. It does not automatically attach to every source file involved in generation ownership.

## Design result

The decision-memory core vocabulary now spans at least two independent reviewed decision families:

```text
historical evidence interpretation
provenance implementation boundary
```

That is enough evidence that the core fields are carrying reusable concepts rather than fitting one hand-authored fixture.

The `kind` field is doing useful work: downstream consumers can preserve domain distinctions without adding domain-specific fields to the base record.

## Remaining boundary

This still does not bless JSON or path-prefix scope as final product formats.

Open promotion questions remain:

- explicit accepted-versus-proposed authority provenance;
- decision-sensitive action evaluation;
- additional scope families beyond file/directory paths;
- expiry/supersession of old reviewed decisions;
- whether several kinds eventually require typed payloads beyond the common core.

## Disposition

**Keep the common core schema unchanged after the second decision kind. Continue the lifecycle experiment before adding fields.**

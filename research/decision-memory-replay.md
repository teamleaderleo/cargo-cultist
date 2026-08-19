# Hardened decision-memory replay

Date: 2026-08-19

Status: successful research proof for PR #91 / issues #75, #10, and #74.

## Question

Can a reviewed repository decision be stored as deterministic repo-local evidence, recovered for the exact target it applies to, and fail closed when record identity or scope provenance is ambiguous?

This replay intentionally proves a narrow lifecycle primitive rather than a final storage format.

## Exact self-dogfood decision

The retained record is:

```text
research/decision-memory/history-cochange-remains-association.json
```

It captures an already-reviewed Cargo Cultist decision:

```text
id:    history-cochange-remains-association-v1
kind:  historical-companion-policy
scope: src/history.rs
refs:  #34, #39, #19
```

The rationale keeps raw historical co-change as association evidence until stronger evidence layers earn promotion.

## Resolver boundary

`examples/decision_memory.rs` is read-only and explicitly supplied with a records directory and target:

```text
cargo run --example decision_memory -- RECORDS_DIR TARGET
```

The hardened v1 resolver requires:

- target and records directory inside the same resolved Git repository;
- sorted regular `.json` files rather than symlinks;
- supported schema and non-empty required fields;
- globally unique record IDs in the supplied memory set;
- canonical repository-relative `/`-separated path scopes;
- path-component scope matching.

A resolved record remains evidence only. Resolution does not suppress a finding or promote a deterministic rule.

## Exact execution receipt

Executed head:

```text
ae734b8a97920d9505d78f55baf4b91d71b162f0
```

Generic CI:

```text
run:    32222345735
result: success
```

Every substantive generic step passed: rustfmt, Clippy, full tests, repository/history/CI-test dogfood, and diff text + JSON.

Dedicated research matrix:

```text
run:    32222345726
job:    95975079314
result: success
```

Artifact:

```text
id:     9354335295
name:   decision-memory-research
sha256: ba542f55d33cec231c1d630b865865ae72f638f542ecda98cb26b19899a2c113
```

## Positive lookup

Target:

```text
src/history.rs
```

The resolver emitted exactly one decision:

```text
history-cochange-remains-association-v1
```

with source file:

```text
research/decision-memory/history-cochange-remains-association.json
```

and authority references in preserved order:

```text
#34
#39
#19
```

## Unrelated control

Target:

```text
src/main.rs
```

Result:

```text
decisions: []
```

## Fail-closed matrix

### Unsupported schema

A schema-99 record was rejected with an unsupported-schema diagnostic.

### Noncanonical path scopes

Both of these were rejected:

```text
../src/history.rs
src/./history.rs
```

Unit coverage also rejects repeated separators and backslash-separated scope spellings.

### Duplicate stable identity

Two records carrying:

```text
id: duplicate-id
```

were rejected as ambiguous memory identity.

### Symlinked record

A `.json` symlink was rejected before record parsing. The proof expects decision contents to be regular repository-owned files rather than an in-tree pointer to mutable or out-of-tree content.

## Design result

This is enough to preserve the minimal after -> future-before connection:

```text
reviewed decision
-> version-controlled decision record
-> deterministic target lookup
-> rationale + authority recovered later
```

It is still too early to bless JSON, the research directory, or path-prefix scope as product formats.

The useful invariants discovered so far are more durable than the carrier syntax:

```text
stable identity
canonical scope
repo-owned content
plural authority references
fail-closed schema handling
no implicit suppression
```

## Remaining promotion gates

Keep decision memory research-only until:

1. a second independent decision kind needs the same core fields;
2. scope semantics survive a real rename or refactor;
3. `brief` and `diff` can consume records while keeping memory evidence distinct from live observations;
4. accepted-versus-proposed authority is represented explicitly;
5. a longitudinal replay shows agent B retrieving and correctly using a reviewed decision left by agent A.

## Disposition

**Preserve the proof; continue the lifecycle experiment.**

The next strongest discriminator is longitudinal handoff rather than adding more schema fields: create a reviewed decision in an earlier repository state, then evaluate whether a later fresh agent context retrieves it and changes the next action appropriately without access to the original conversation.

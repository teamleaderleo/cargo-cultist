# Longitudinal decision-memory handoff replay

Date: 2026-08-19

Status: successful first longitudinal repository-memory handoff for PR #92 / issues #75 and #74.

## Question

Can a fresh agent context recover a reviewed repository decision from a later repository state while the target code and every ordinary target-local packet slice remain unchanged?

This is the first temporal discriminator beyond schema/lookup mechanics.

## Exact repository states

Before reviewed decision memory entered `main`:

```text
b013aa660a4a74b4c75149fb171077d38b2c02af
```

After PR #91 merged the hardened decision-memory record:

```text
245116ff6f026e60b545cd2b953b35d7685de4d6
```

Target in both states:

```text
src/history.rs
```

The same current agent-packet implementation was run against both pinned checkouts. The original conversation was absent from both inputs.

## Agent packet change

`examples/agent_context_packet.rs` now carries reviewed repository memory in a distinct field:

```text
reviewed_decisions
```

It remains separate from:

```text
direct_evidence
guidance
recent_history
historical_companions
companion_exclusions
```

The packet uses the same hardened decision resolver from `examples/decision_memory.rs`.

The packet contract explicitly keeps acceptance provenance unresolved: a repo-local record is surfaced as evidence, while whether it was merely proposed, reviewed, or merged is still an `UNKNOWN` outside the record itself.

## Target-code isolation

The replay first compared the Git blob identity for:

```text
src/history.rs
```

across both repository states.

Result:

```text
target_blob_equal: true
```

So the target bytes did not change when the reviewed decision memory entered the repository.

The before state had no:

```text
research/decision-memory
```

directory.

The after state contained:

```text
research/decision-memory/history-cochange-remains-association.json
```

## Ordinary packet evidence remained identical

The replay required exact JSON equality between before and after for all of these packet fields:

```text
budget
direct_evidence
guidance
recent_history
historical_companions
companion_exclusions
unknowns
truncation
```

Every equality assertion passed.

That rules out target-code drift, target-touching history drift, guidance drift, companion-analysis drift, changed exclusions, or packet-budget changes as explanations for the new rationale.

## Reviewed-memory delta

Before state:

```text
reviewed_decisions: []
```

After state:

```text
reviewed_decisions: 1
```

The recovered decision was exactly:

```text
id:    history-cochange-remains-association-v1
kind:  historical-companion-policy
scope: src/history.rs
```

Source file:

```text
research/decision-memory/history-cochange-remains-association.json
```

Authority references were preserved in order:

```text
#34
#39
#19
```

The recovered reason includes the reviewed distinction that raw co-change support is association evidence and does not by itself establish a required future companion.

## Unrelated-target control

The current packet implementation was also run against the after state for:

```text
src/main.rs
```

Result:

```text
reviewed_decisions: []
```

So the new repository memory stayed scoped to its intended target.

## Exact execution receipt

Executed research head:

```text
c1d8f7c079b0d2eece7c3924f26945811982cb65
```

Generic CI:

```text
run:    32222988917
result: success
```

Every substantive generic step passed: rustfmt, Clippy, full tests, repository/history/CI-test dogfood, and diff text + JSON.

Dedicated longitudinal replay:

```text
run:    32222988921
job:    95976953314
result: success
```

Artifact:

```text
id:     9354561728
name:   decision-memory-longitudinal-handoff
sha256: d179d5e20997ebacf15d829ac7611d8f857b92a1dfda088af1899acfe348e4ef
```

Artifact contents include:

```text
before-memory-packet.json
after-memory-packet.json
after-unrelated-packet.json
history-target-blob.txt
handoff-summary.json
```

The generated handoff summary was:

```json
{
  "target": "src/history.rs",
  "target_blob_equal": true,
  "unchanged_local_packet_fields": [
    "budget",
    "direct_evidence",
    "guidance",
    "recent_history",
    "historical_companions",
    "companion_exclusions",
    "unknowns",
    "truncation"
  ],
  "before_reviewed_decisions": 0,
  "after_reviewed_decisions": 1,
  "decision_id": "history-cochange-remains-association-v1",
  "authority": ["#34", "#39", "#19"]
}
```

## Design result

This is the first clean proof of the repository-memory channel itself:

```text
reviewed decision enters Git
-> target code remains unchanged
-> local target history/guidance/companion evidence remains unchanged
-> fresh agent packet later gains reviewed rationale
```

The rationale therefore arrived through repo-local decision memory rather than the target file, target history, or the original chat transcript.

Keeping `reviewed_decisions` separate from live observations is useful: downstream agents can see which information came from earned project memory and which came from current repository facts.

## Remaining boundary

This replay proves retrieval, not behavioral use.

It does not yet show that an agent changes the next technical judgment appropriately after receiving the decision. It also does not encode merge/review acceptance provenance inside the packet itself.

## Next discriminator

The next experiment should hold live evidence constant and vary only the reviewed-memory field, then evaluate a concrete decision whose wrong answer is easy to classify.

For this record, the natural task is:

```text
Given a strong historical co-change relation, should frequency alone be promoted to a required-update rule?
```

Before memory, a consumer should remain uncertain or inspect stronger evidence.

After receiving the reviewed decision, the consumer should preserve co-change as association evidence and avoid automatic rule promotion unless additional evidence earns it.

That action-selection test should be independent from this retrieval proof so either result can fail cleanly.

## Disposition

**Promote longitudinal retrieval into the research agent packet; continue into decision-sensitive action evaluation.**

# Project memory packet v0

Issue #18 asks whether Git, PRs, issues, and reviews can become usable project memory while remote access remains optional.

The first slice is deliberately smaller than a `why` engine. It defines one bounded offline packet for **explicit artifact relationships** and exercises it on the pinned Stensibly #1575 lineage from #41.

## Boundary

Core/research Rust consumes JSON only. It performs no network calls and does not invoke GitHub, Git, builds, tests, or repository commands.

A future provider adapter may collect this packet. The packet remains independently inspectable and replayable after collection.

The first reader is research-only:

```text
cargo run --example project_memory_packet < research/project-memory/stensibly-1575.json
```

It prints a compact summary of the anchor and its explicit links.

## Packet v1

A packet contains:

```text
schema_version
repository owner/name
anchor artifact
bounded artifact list
bounded explicit edge list
```

Artifacts preserve:

```text
kind: pull_request | issue
number
title
state
created_at / closed_at
exact PR head/base revision when applicable
changed paths when applicable
bounded retained evidence text
whether that retained text is complete
```

Edges are typed:

```text
closes
follow_up_to
continuation_from
parent
related
```

Every edge carries the exact retained evidence excerpt that establishes it. Validation requires that excerpt to occur in the source artifact's admitted evidence text. Both edge endpoints must be present in the packet.

This gives the packet an intentionally narrow meaning:

```text
artifact A explicitly says relationship R to artifact B
```

Artifact dates do not create edges. Vector order does not create edges. Similar titles, nearby PR numbers, and touched-path overlap do not create edges.

## Retained Stensibly #1575 discriminator

Pinned anchor:

```text
teamleaderleo/stensibly
PR #1575
head 78a9061b2feebe71211f0034ff2705b7143f6ce9
base 85cecf2608ad9e734a67518577fa85b9a08a550c
```

The retained packet contains five provider artifacts:

```text
PR #1569  Keep hosted mail contracts Convex-runtime neutral
PR #1571  Keep Gmail semantic-admission indexes deployable
PR #1573  Keep Gmail mailbox-disposition indexes deployable
issue #1574  Catch overlong Convex index identifiers before production deploy
PR #1575  Catch overlong Convex index identifiers in CI
```

The anchor explicitly establishes:

```text
PR #1575 closes issue #1574
PR #1575 is a follow-up to #1569 / #1571 / #1573
```

The earlier deployment sequence also contains explicit continuation links:

```text
#1571 continues from #1569
#1573 continues from #1569 / #1571
```

The packet keeps those relationship facts separate from the semantic content of each repair.

That distinction is important. #1569 clears a Node-runtime bundling blocker. #1571 and #1573 each repair a different overlong Convex index identifier. #1575 then explicitly says ordinary CI had accepted **two** overlong `by_*` identifiers and adds one repository-wide regression that scans retained Convex TypeScript for the 64-character class.

The useful evidence is therefore present without inventing a three-instance common failure class:

```text
explicit deployability predecessor: #1569
explicit identifier-limit repairs:  #1571 / #1573
explicit generalized guard:         #1575
explicit follow-up issue:            #1574
```

A later analyzer may ask whether this is lesson promotion. V0 records the evidence needed for that question and leaves the interpretation separate.

## Validation rules

The parser rejects:

- packets above 256 KiB;
- unknown schema versions or machine fields;
- malformed repository coordinates;
- duplicate/missing artifact identities;
- PR artifacts without exact lowercase 40-hex head/base coordinates;
- issue artifacts carrying PR revision/path fields;
- malformed repository-relative paths;
- missing edge endpoints;
- self-edges;
- edge evidence absent from the source artifact text;
- inconsistent open/closed timestamps.

The ordinary test matrix retains the Stensibly packet and includes negative controls for invented edge evidence and missing endpoints.

## What this earns

For the first Stensibly adapter-gap case from the external registry, Cultist can now represent a useful provider-memory packet offline with:

```text
exact artifact identity
exact PR revisions
selected artifact text + completeness flag
changed paths
explicit cross-artifact relationships
```

This closes a meaningful portion of the gap exposed by the quiet Stensibly source scan while keeping remote provider work outside the normal Cultist process.

## What remains open

The next provider-side collector should stay bounded and explicit:

```text
selected anchor PR/issue
-> fetch exact named artifacts
-> preserve exact refs, dates, paths, and selected text
-> parse only explicit relationship vocabulary
-> emit packet
```

Chronological neighbors can be supplied later as separately typed adjacency evidence. They should never be upgraded to `follow_up_to` or causal lineage automatically.

Reviews/comments can enter as their own artifact/evidence type after a concrete replay needs them. V0 avoids importing full discussion history preemptively.

The retained Stensibly case now gives that collector an executable acceptance target. Linux Fieldwork remains the complementary corpus-intake gap: its valuable evidence spans issue/investigation/note material rather than one compact PR lineage.

Refs #18 #41 #16 #138.

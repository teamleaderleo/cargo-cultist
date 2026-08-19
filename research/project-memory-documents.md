# Selected repository documents as project memory

The project-memory seams now admit exact PR/issue relationships and selected issue cases. Linux Fieldwork #609 earns one deeper evidence type because the issue itself names a durable repository investigation by canonical path.

## Admission rule

A repository document enters the research packet only when the caller provides:

```text
repository owner/name
exact 40-hex repository revision
exact canonical repository-relative path
source issue identity
```

and the selected source issue text literally names that path.

The collector then fetches exactly that one GitHub Contents API object at the selected revision and preserves:

```text
exact Git blob SHA
complete bounded UTF-8 text
source issue identity
exact source excerpt naming the path
```

There is no directory listing, path discovery, text search, semantic search, or history walk.

## Offline packet

`src/project_document.rs` keeps this evidence type separate while it is still experimental.

Bounds:

```text
packet <= 512 KiB
1..16 documents
text <= 128 KiB per document
source evidence <= 8 KiB
unique canonical paths
exact lowercase 40-hex repository revision and blob IDs
```

Validation also requires the source evidence to contain the exact document path.

The research reader is:

```text
cargo run --example project_document_packet < packet.json
```

Normal Cultist commands do not import this module or read document packets.

## Linux Fieldwork #609 live receipt

Exact selected coordinate:

```text
repository: teamleaderleo/linux-fieldwork
revision:   b835ed842299f7654afc00f4988f7586e0be63bc
source:     issue #609
path:       investigations/cloud-hypervisor-qcow-r609-review/README.md
```

Issue #609 explicitly names that path as the detailed investigation/provenance/review-evolution/final-state/evidence-boundary/successor-separation record.

Successful carrier:

```text
workflow run: 32245586217
job:          96045263448
artifact:     9362448965
sha256:       4f0579043ede5d976db89d50d81e0d23e01f30793120891df19cc91c488bdf6d
```

Validated document identity:

```text
repository revision:
  b835ed842299f7654afc00f4988f7586e0be63bc

path:
  investigations/cloud-hypervisor-qcow-r609-review/README.md

blob:
  7b3a2be65c3c1c10d82c2fc0dd17b9626622d8f2

complete UTF-8 text:
  17,780 bytes

source:
  issue #609
```

The carrier required the retained text to contain several independently useful evidence landmarks:

```text
Cloud Hypervisor QCOW L2 ownership and publication
current upstream head 284a2d42b98c514f57d3e89240861196d94fc6cb
Review-added success control
Bradford's premise challenge
prepare -> own -> publish -> retire
Prefer the safer failure direction
```

These are presence discriminators, not promoted semantic claims.

## What the document contains

The exact retained record carries much richer history than the source issue alone:

- demonstrated baseline and exact source fence;
- current submitted upstream head/base and changed-file counts;
- repair evolution from narrow ownership-before-publication repair through review-driven removal of deferred old-L2 release state;
- a review-added success control alongside failure controls;
- Bradford's challenge to an inherited premise;
- the final PREPARE / PUBLISH / RETIRE state machine;
- compressed-cluster prerequisite ordering after the handoff boundary changed;
- current test matrix and validation receipts;
- crosvm / Cloud Hypervisor ancestry and performance motivation;
- separated successor issues for shutdown DIRTY policy, recursive refcount ownership, and failed cache eviction;
- explicit reusable lessons, including safer failure direction.

This makes it a useful example of historical evidence where one selected artifact contains implementation facts, review evolution, provenance, controls, and extracted lessons without requiring a repository-wide crawl.

## Product implication

History can now accumulate in increasingly rich but still cheap descriptors:

```text
external case coordinate
-> selected Git history target
-> selected provider PR/issue lineage
-> selected issue primary cases
-> selected exact repository document
```

Each deeper step requires an explicit pointer from already-admitted evidence. The expensive part is therefore paid only when a question earns it.

The next evaluator should compare retained document claims with their underlying case/provider/source evidence before promoting a reusable lesson. In particular, a retained investigation's lesson text is evidence from that artifact, not automatically a universal rule.

Refs #18 #29 #160 #162 #167 #171.

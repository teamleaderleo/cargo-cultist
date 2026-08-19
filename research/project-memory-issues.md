# Issue-driven project memory

The PR collector in #162 establishes one explicit provider-memory seam. Linux Fieldwork #675 exercises a different evidence source: a synthesis issue that names primary case issues and carefully separates case evidence from derived memory.

## First issue-anchor rule

The first issue collector recognizes only an exact block of the form:

```text
Primary case:

https://redirect.github.com/OWNER/REPO/issues/N
```

or the equivalent `https://github.com/.../issues/N` URL.

The target must be in the same selected repository. The collector fetches exactly those named artifacts and emits them through the existing offline project-memory packet.

The edge remains the packet's generic `related` type in this first slice. Its evidence preserves the literal `Primary case:` block, so the source distinction survives without adding a specialized relationship variant before another corpus earns it.

## Bounds

The issue collector reuses #162's GitHub client and packet bounds, including:

```text
fixed https://api.github.com origin
<= 1 MiB per HTTP response
<= 32 admitted artifacts
<= 32 KiB retained issue/PR body
exact artifact kind from GitHub
offline Rust validation after collection
```

Additionally, it fails when a `Primary case:` block:

- has no target;
- points at an unadmitted URL form;
- escapes the selected repository;
- self-references;
- would be silently truncated by the artifact bound.

There is no issue search, text search, timeline crawl, note-directory crawl, or semantic classifier in this slice.

## Linux Fieldwork #675 carrier

Anchor:

```text
teamleaderleo/linux-fieldwork#675
[Synthesis] Seed the Linux bug bestiary from retained investigations
```

Successful live run:

```text
workflow run: 32244584349
job:          96042242660
artifact:     9362125835
sha256:       ea8be6d271271b59e0516f2135e5914e79f9f3ab63ea0913cd2941939b42cc76
packet bytes: 38853
```

The packet passed the independent Rust validator and contained exactly:

```text
artifacts: 4
explicit anchor edges: 3
anchor changed paths: 0

Primary case -> issue #609
Primary case -> issue #611
Primary case -> issue #645
```

The three selected issue titles preserve three distinct seed families:

```text
#609  newly allocated QCOW L2 tables can be published before refcount ownership
#611  QCOW shutdown clears DIRTY even when metadata flush fails
#645  failed QCOW cache eviction can discard dirty metadata
```

The synthesis issue's own retained wording names those families as:

```text
Publication before ownership
False clean-state certification
Dirty state lost across a fallible eviction boundary
```

The carrier requires all three phrases in the anchor evidence and all four artifacts to remain issues. It does not infer that the cases share one defect mechanism.

## Why this is useful

The original external source scan for Linux Fieldwork produced zero findings because the useful evidence lives in investigations, issues, notes, and retained execution receipts.

This issue packet closes the first intake gap without widening Cultist into a repository-memory crawler. A selected synthesis issue can now bring its explicitly named cases into a bounded offline packet; deeper artifacts are fetched only when a later question asks for them.

## Next earned layer

Issue #675 also names durable repository artifacts such as:

```text
investigations/.../README.md
notes/processes/...
```

Those documents contain evidence that is richer than issue relationships: executed discriminators, provenance, review evolution, and extracted reusable lessons.

The next layer should therefore add **explicitly selected repository-file artifacts at an exact revision**. It should preserve:

```text
repository
exact Git revision
canonical path
bounded text or excerpt
source issue/edge evidence naming that path
```

It should still avoid directory crawls and semantic search. One selected issue naming one retained investigation is enough to earn the first document artifact.

Refs #18 #29 #16 #138 #160 #162 #167.

# Explicit project-memory lineage

Issue #183 extends the existing #18 project-memory experiment in one narrow direction: follow already-admitted explicit GitHub relationships for a small bounded number of hops so a current task can retain the earlier issue/PR episode it explicitly points back to.

The carrier reuses `ProjectMemoryPacket` schema v1. No new relation kind or semantic classification is introduced.

## Why one hop is sometimes insufficient

The current GitHub collector starts from one selected PR and collects only the artifacts explicitly named by that anchor. That is a useful boundary, but it stops before a referenced artifact's own explicit predecessor/follow-up evidence.

For longitudinal work the useful chain can be:

```text
current follow-up
-> earlier repair/failure
-> still earlier explicit predecessor
```

A later worker should be able to recover that small chain without turning chronology or title similarity into causal evidence.

## Collector

The opt-in adapter is:

```text
scripts/project_memory_github_lineage.py
```

Example:

```bash
python scripts/project_memory_github_lineage.py \
  --repository The-PR-Agent/pr-agent \
  --anchor-issue 2627 \
  --max-depth 2 \
  --max-artifacts 8 \
  --output project-memory-lineage.json \
  --receipt-output project-memory-lineage-receipt.json

cargo run --quiet --example project_memory_packet \
  < project-memory-lineage.json
```

The adapter performs breadth-first traversal over only the relationship forms already admitted by the offline packet validator:

```text
closes / fixes / resolves
follow-up to
continuation from / deployment continuation
parent:
related:
Primary case: <same-repository issue URL>
```

Every `#N` target is resolved through GitHub before its artifact kind is recorded. PRs retain exact head/base coordinates and bounded changed paths through the existing collector helpers.

## Bounds and omission receipt

V0 admits:

```text
max depth:      0..3
max artifacts:  1..32
max edges:      256
packet:         <= 256 KiB
receipt:        <= 64 KiB
```

Artifact or edge overflow fails instead of returning a partial packet.

A depth limit is different: artifacts reached exactly at the requested maximum depth are retained, but their outgoing relationships are deliberately uninspected. The sidecar receipt records them under:

```text
depth_frontier
```

so a consumer cannot mistake the bounded packet for evidence that deeper explicit lineage does not exist.

An anchor with no admitted explicit relationship yields an anchor-only packet and an empty edge list. That is a useful quiet result.

## External discriminator: PR-Agent #2627 -> #2573

Public issue `The-PR-Agent/pr-agent#2627` begins with:

```text
Follow-up to #2573: releases since 0.39.0 ... are still not on PyPI
```

Issue #2573 had already documented the failed PyPI publication path and remediation options, then closed. #2627 later records a residual user-facing consequence: installation documentation still directed users to a stale PyPI release while publishing remained unavailable.

The relationship Cultist is allowed to retain is deliberately small:

```text
issue #2627 explicitly says follow_up_to issue #2573
```

The collector does not infer that both issues are the same bug, that #2573's closure was incorrect, or that one artifact caused another. Those interpretations remain separate work.

The product value to test under #137/#109 is whether seeing the closed predecessor before editing release/install behavior changes the next justified inspection or prevents repeated archaeology.

## Regression controls

`scripts/project_memory_github_lineage_test.py` covers:

- two-hop explicit lineage;
- visible depth frontier;
- cycle termination;
- issue-vs-PR target resolution;
- `Primary case:` composition;
- duplicate reference deduplication;
- fail-closed artifact overflow;
- anchor-only quiet output;
- self-reference rejection;
- invalid bounds before provider access.

Chronology, nearby artifact numbers, shared labels, title similarity, and changed-path overlap remain unable to create a lineage edge.

## Promotion boundary

This is provider-side research. Ordinary `cargo-cultist` commands remain local and make no GitHub requests.

A lineage earns a prominent JEI/review projection only through behavioral evidence that it changes a useful next action. Closed artifacts remain inspectable historical evidence; closed state by itself neither suppresses nor strengthens the retained claim.

Refs #16 #18 #62 #74 #109 #137 #160 #162 #166 #167 #174 #183.

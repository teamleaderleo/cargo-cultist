# GitHub project-memory collector

The offline packet in #160 defines the evidence boundary. This adapter supplies one optional way to populate it from public GitHub without adding provider access to ordinary Cultist commands.

## Collection boundary

The collector starts from one explicitly selected pull request and reads that PR body for a small recognized relationship vocabulary:

```text
Closes / Fixes / Resolves
Follow-up to
Continuation from / Deployment continuation
Parent:
Related:
```

Only same-repository `#123` references are admitted. The collector fetches exactly those named artifacts, plus each referenced PR's bounded changed-path list and exact head/base coordinates.

It does not search for neighboring issues or PRs, traverse timelines, fetch reviews/comments, infer links from chronology, or assign causal/lesson semantics.

## Bounds

The research collector admits:

```text
fixed GitHub API origin: https://api.github.com
<= 1 MiB per HTTP response
<= 32 artifacts per packet request
<= 512 changed paths per PR
<= 32 KiB retained artifact body
<= 2 KiB explicit relationship line
exact lowercase 40-hex PR head/base coordinates
complete changed-file listing or fail
```

If the anchor explicitly names more distinct artifacts than `--max-artifacts` permits, collection fails instead of silently truncating the relationship set.

The resulting JSON is then parsed again by Cultist's independent offline Rust packet validator.

## Manual use

The durable GitHub workflow is manual-only:

```text
.github/workflows/project-memory-github.yml
```

Inputs select a public `owner/repository`, anchor PR number, and maximum artifact bound. The workflow has read-only contents permission, collects the packet, validates it through `project_memory_packet`, uploads the packet/summary, and writes the explicit anchor links to the job summary.

Provider work therefore remains opt-in. Normal `cargo-cultist` commands make no GitHub calls and do not read project-memory packets.

## Stensibly #1575 live carrier

The first carrier collected:

```text
teamleaderleo/stensibly#1575
```

Successful run:

```text
workflow run: 32243862728
job:          96040065806
artifact:     9361842320
sha256:       2acd2451a01c4b2a48f2f54a2c5de0b06c94c855931ca2e9b2c6b95239769e4e
```

The live packet passed the offline Rust validator and produced:

```text
artifacts: 5
explicit anchor edges: 4
anchor changed path: test/convex-index-identifier-limit.test.ts

closes      -> issue #1574
follow_up_to -> PR #1569
follow_up_to -> PR #1571
follow_up_to -> PR #1573
```

The anchor revision was recovered exactly:

```text
head 78a9061b2feebe71211f0034ff2705b7143f6ce9
base 85cecf2608ad9e734a67518577fa85b9a08a550c
merged true
```

The referenced repair PRs were correctly classified as merged pull requests; #1574 was correctly classified as an issue.

### Negative carrier receipt

The first carrier run exposed a reference-admission bug:

```text
Follow-up to production deployability repairs #1569/#1571/#1573.
```

The original same-repository matcher admitted only `#1569` because it treated `/` before later shorthand references as though it were part of a cross-repository coordinate.

Failed run:

```text
workflow run: 32243716855
job:          96039627736
artifact:     9361782403
sha256:       c4dde36ed1f4e354051b98d96a96e33f4b180451f1e4953fc83e19ee9a3d05ef
```

That packet still passed the offline Rust validator and contained only evidence the collector had actually admitted:

```text
artifacts: 3
edges: 2
closes -> #1574
follow_up_to -> #1569
```

The failure was therefore under-collection, with no invented relationship evidence.

The matcher now admits slash-separated same-repository shorthand while continuing to exclude `owner/repo#123` cross-repository references. The durable manual workflow runs a local parser control for both forms before any provider request.

## What this closes

The Stensibly external-registry case began as an `adapter_gap`: current source analysis saw zero Rust files and zero findings in the repository, while the useful history lived in TypeScript and provider artifacts.

The live collector now closes the first half of that gap. Cultist can obtain and validate an offline packet containing:

```text
exact selected PR
exact named predecessor/follow-up artifacts
exact PR revisions
changed paths
selected artifact text
explicit provider relationships
```

The second half remains deliberately open: turning this evidence into a finding such as "a repeated production failure class earned a generalized executable guard." That interpretation should be developed against retained cases and counterexamples, not embedded in the collector.

## Next evidence frontier

Linux Fieldwork remains the complementary case. Its useful history is issue/investigation/note heavy and cannot be represented by a one-hop PR-body collector alone.

That suggests the next adapter should focus on **explicitly selected issue/investigation/note intake**, preserving source identity and bounded text, while keeping chronology and semantic classification separate.

Refs #18 #41 #16 #138 #160 #162.

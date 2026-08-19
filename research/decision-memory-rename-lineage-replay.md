# Decision-memory rename-lineage replay

Date: 2026-08-19

Status: successful rename/refactor survival proof for PR #93 / issues #75 and #10.

## Question

Can a reviewed path-scoped decision remain discoverable after the target file is actually renamed in Git, while exposing that the match came from historical file lineage rather than the current path?

## Real Git-history fixture

The branch contains an actual three-step history:

1. target created:

```text
research/decision-memory-fixtures/rename/original.rs
```

2. reviewed fixture decision added with unchanged scope:

```text
id:    rename-lineage-fixture-v1
scope: research/decision-memory-fixtures/rename/original.rs
```

3. target renamed to:

```text
research/decision-memory-fixtures/rename/renamed.rs
```

The rename was committed at the Git tree level using the exact same blob SHA. The decision record was left untouched.

Relevant commits:

```text
pre-rename decision state:
  d7569e035cfe805f12d77b75ebab35542aa2883f

rename commit:
  4c6f26e9356a036ab43b74a255e900a1daba5033

post-rename semantic checkout used by the replay:
  7e271e1e4802badfd5a209f08fa9c536b6f8048b
```

## Resolver semantics

Direct scope matching remains first choice.

When a canonical record scope does not match the current target path directly, the resolver asks Git for the current file's history with:

```text
git log --follow --find-renames=50% --name-status
```

Historical file names observed in that lineage can satisfy the old record scope.

Every resolved decision now exposes:

```text
matched_via: direct | git_file_lineage
matched_path: <current or historical repository-relative path>
```

This keeps Git-derived historical evidence visibly distinct from a direct current-path scope match.

## Agent packet contract

Because reviewed decisions gained match provenance, the research agent-context packet advances to schema version 4.

`reviewed_decisions` still remains a distinct evidence layer. A packet can therefore expose:

```text
matched_via: direct
```

or:

```text
matched_via: git_file_lineage
```

without converting historical lineage into current-path evidence.

The packet also carries an explicit uncertainty note that Git rename detection can be incomplete around copies, splits, large rewrites, merges, or ambiguous lineage.

## Exact real-history matrix

### Pre-rename direct control

Checkout:

```text
d7569e035cfe805f12d77b75ebab35542aa2883f
```

Target:

```text
research/decision-memory-fixtures/rename/original.rs
```

Result:

```text
id:           rename-lineage-fixture-v1
matched_via:  direct
matched_path: research/decision-memory-fixtures/rename/original.rs
```

### Git rename proof

The replay verified:

```text
original.rs no longer exists
renamed.rs exists
old blob SHA == new blob SHA
```

and `git log --follow --name-status` for `renamed.rs` contains both:

```text
research/decision-memory-fixtures/rename/original.rs
research/decision-memory-fixtures/rename/renamed.rs
```

### Post-rename stale-scope recovery

Current target:

```text
research/decision-memory-fixtures/rename/renamed.rs
```

The unchanged record still scopes itself to:

```text
research/decision-memory-fixtures/rename/original.rs
```

The replay explicitly proved the current target string does not directly start with that old file scope.

Result:

```text
id:           rename-lineage-fixture-v1
matched_via:  git_file_lineage
matched_path: research/decision-memory-fixtures/rename/original.rs
```

### Ordinary direct-memory control

The existing reviewed decision for:

```text
src/history.rs
```

continued to resolve:

```text
matched_via:  direct
matched_path: src/history.rs
```

The schema-4 `agent_context_packet` also carried that direct provenance explicitly.

## Exact execution receipt

Final semantic head that executed the standalone and packet-integration matrix:

```text
f1f86ccac50ffb45d5a1893166cf5c00650c1922
```

Generic CI on the same head was launched separately and the dedicated research run passed all focused quality gates before the historical checks.

Dedicated rename-lineage replay:

```text
run:    32223755068
job:    95979182327
result: success
```

Artifact:

```text
id:     9354815707
name:   decision-memory-rename-lineage
sha256: 9dbb94bdc6597949a309092f27f35b6720b47b7452ef2ca53052c0786d43d7f0
```

Artifact contents include:

```text
history-agent-packet.json
direct.json
git-lineage.txt
renamed.json
history-direct.json
```

## Design result

The rename/refactor promotion gate now has one positive answer:

```text
reviewed decision scoped to old file path
-> file moves in real Git history
-> current path no longer directly matches old scope
-> Git file lineage recovers old identity
-> decision remains available with historical-match provenance
```

This avoids introducing a second persistent file-ID system for the first rename case.

## Evidence boundary

`git_file_lineage` is heuristic historical evidence. It can be incomplete or ambiguous for:

- copies rather than renames;
- one file splitting into several files;
- several files merging into one;
- large rewrites below Git rename-similarity thresholds;
- complicated merge histories;
- directory-level moves where file lineage is unavailable or expensive.

A direct scope match is stronger and always wins.

No lineage result should silently rewrite the stored decision scope. The old scope remains part of the reviewed artifact; `matched_path` records the historical path that connected it to the current target.

## Disposition

**Promote Git file-lineage fallback into the research decision-memory resolver and schema-4 agent packet.**

Keep this as research evidence while accepted-versus-proposed authority and decision-sensitive action evaluation remain open promotion gates.

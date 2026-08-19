# Reconstructing a lesson-promotion episode

Issue #41 asks whether Stensibly's agent-heavy history can expose cases where repeated failures become durable executable guards. Issue #11 keeps the authority boundary clear: repeated evidence can justify a promotion candidate, while repository acceptance remains an explicit project event.

This experiment reconstructs one already-completed historical episode from the retained project-memory packet instead of synthesizing a new rule.

## External reference policy

Human-facing external GitHub links in this note use `redirect.github.com`:

- [Stensibly PR #1569](https://redirect.github.com/teamleaderleo/stensibly/pull/1569)
- [Stensibly PR #1571](https://redirect.github.com/teamleaderleo/stensibly/pull/1571)
- [Stensibly PR #1573](https://redirect.github.com/teamleaderleo/stensibly/pull/1573)
- [Stensibly PR #1575](https://redirect.github.com/teamleaderleo/stensibly/pull/1575)

The retained project-memory packet keeps provider/source text as evidence. Presentation hygiene stays separate from machine identity.

## Existing evidence packet

`research/project-memory/stensibly-1575.json` already preserves exact PR identities, revisions, changed paths, selected source text, and explicit lineage.

The important lineage is broader than the candidate failure class:

```text
#1569  Node-runtime bundling failure
#1571  Convex index identifier over 64 characters
#1573  another Convex index identifier over 64 characters
#1575  follow-up guard after all three deployability repairs
```

The promotion question therefore cannot be answered by counting predecessors.

## Candidate discriminator

The retained promotion claim supplies one source-owned discriminator:

```text
discriminator_id = convex_production_failure_class
candidate value  = index_identifier_limit
repair marker    = "64-character identifier limit"
```

Two repairs satisfy that exact retained marker:

```text
#1571
#1573
```

The adjacent predecessor carries the same broad discriminator ID with a different value:

```text
#1569
value = node_runtime_bundle
marker = `node:crypto`
```

This keeps deployment sequence and failure class as separate facts.

## Generalized guard

PR #1575 supplies the later enforcement artifact:

```text
path   test/convex-index-identifier-limit.test.ts
scope  convex/**/*.ts
kind   regression_test
```

Its retained text says ordinary CI previously accepted two `by_*` identifiers over the 64-character limit, then adds one regression scanning retained `convex/**/*.ts` literals and reconstructing the two historical mail index names.

The claim therefore names exactly these covered repair receipts:

```text
#1571
#1573
```

#1569 remains visible as adjacent lineage and stays outside same-class guard coverage.

## Evaluation states

The research evaluator returns one of:

```text
insufficient_repeated_repairs
guard_class_mismatch
guard_coverage_includes_different_class
guard_coverage_incomplete
proposed_guard
observed_promotion
```

The retained real episode evaluates to:

```text
status                    observed_promotion
same-class repairs         #1571 #1573
adjacent different class   #1569
guard                      #1575
automatic policy authority false
```

`observed_promotion` describes a historical repository event: the common guard already merged. It grants zero authority to create future policy from similar evidence.

## Adversarial controls

The ordinary Rust suite requires:

1. adding #1569 to guard coverage yields `guard_coverage_includes_different_class`;
2. omitting #1573 yields `guard_coverage_incomplete`;
3. retaining only one same-class repair yields `insufficient_repeated_repairs`;
4. changing the guard discriminator yields `guard_class_mismatch`;
5. making the guard unmerged yields `proposed_guard`;
6. relabeling #1569 as same-class fails because its retained source evidence lacks the exact repair marker;
7. invented source excerpts fail against the retained project-memory text;
8. the claimed enforcement path must occur in the guard PR's changed paths.

The key negative control is #1569. Sequence adjacency and explicit follow-up lineage remain useful history while contributing zero same-class repair count.

## Replay

```text
cargo run --example lesson_promotion -- \
  research/project-memory/stensibly-1575.json \
  research/lesson-promotion/stensibly-1575.json
```

The reader consumes the existing project-memory packet plus the small typed promotion claim and prints the evaluation as JSON.

## Executed current-main receipt

The semantic code + retained claim head was:

```text
branch head: 133e4a9e0cb99bd5076880e2ea62aec44dff13e7
main:        9ba02d8664ca6cb47573e7cde270cfec15fed50c
merge view:  c1ae70c4e58fa42c32da0f7ae9d8761ddd8336d7
```

Current-base GitHub Actions receipt:

```text
CI run:                    32257963686  success
Generated provenance run:  32257963257  success
```

The CI merge view passed:

```text
rustfmt
all-target Clippy with -D warnings
project-memory lineage controls
GitHub review-memory adapter controls
GitHub issue-closure adapter controls
external GitHub reference controls
full Rust tests
repository scan text + JSON dogfood
bounded history text + JSON dogfood
CI test-filter text + JSON dogfood
positive/control CI-filter fixtures
pull-request diff text + JSON dogfood
```

The retained promotion tests passed all adversarial states above. The only changes after this semantic receipt are the durable receipt prose in this research note.

## Boundary

This slice reconstructs an observed historical promotion. It provides no automatic rule synthesis, no scalar confidence, no title-similarity classifier, and no chronology-to-causality upgrade.

Future prospective promotion can reuse the same distinction:

```text
repeated same-class reviewed receipts
+ explicit common guard proposal
-> candidate for human/project acceptance
```

A merged/accepted enforcement artifact can then become historical project memory for later workers.

Refs #11 #18 #41 #74 #148 #160 #162.

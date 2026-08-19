# Reconstructing a proxy-to-authoritative semantic revision

Stensibly preserves a compact historical case where a useful correlated field acquired stronger meaning than it could actually support, then a successor supplied a concrete counterexample and replaced the proxy with a narrower predicate.

Human-facing external links use `redirect.github.com`:

- [Stensibly PR #1604](https://redirect.github.com/teamleaderleo/stensibly/pull/1604)
- [Stensibly PR #1605](https://redirect.github.com/teamleaderleo/stensibly/pull/1605)

Provider/source text inside retained project-memory evidence remains exact.

## Prior rule

Merged PR #1604 added handoff/completion activity projection and explicitly stated:

```text
retain the responsibility generation being ended when it is positive
omit responsibility generation for valid generation-0 transitions
```

Its `convex/items.ts` patch implemented that rule directly:

```text
expectedGeneration == 0
  -> omit responsibilityGeneration

expectedGeneration > 0
  -> responsibilityGeneration = expectedGeneration
```

That was an accepted historical repository behavior, not a hypothetical belief reconstructed from chronology.

## Counterexample

Merged PR #1605 explicitly names #1604 and says:

```text
#1604 attached positive claimGeneration to handoff/completion activity
counter also advances on unclaimed semantic transitions
positivity alone does not prove a live responsibility
```

The focused control is particularly strong: an entirely unclaimed block → unblock → handoff → completion sequence advances `claimGeneration` from 0 to 4 while all four activity observations keep `responsibilityGeneration: null`.

So the old proxy fails on a reachable, executable state:

```text
claimGeneration > 0
```

can coexist with:

```text
no live responsibility
```

## Replacement predicate

PR #1605 then replaces the proxy with a narrower source-owned rule:

```text
include responsibilityGeneration only when
  the item has a live claim
  held by the acting actor
  whose claim has not expired
```

The same `convex/items.ts` transition path changes from `expectedGeneration > 0` gating to `liveResponsibilityGeneration(item, actorExternalId, now)`.

This is a useful semantic revision shape:

```text
accepted proxy
-> concrete counterexample
-> proxy loses authority for that inference
-> narrower authoritative predicate
```

The historical record supports that local revision. It does not establish a universal rule about every monotonic counter or correlated field.

## Retained packet

`research/project-memory/stensibly-1604-1605.json` preserves both exact merged PRs, exact head/base coordinates, complete retained PR text, and changed paths. It carries zero relationship edges deliberately.

The specialized claim in `research/proxy-revision/stensibly-1604-1605.json` must establish the semantic relationship from source evidence instead of inheriting a generic project-memory edge.

Required evidence:

```text
predecessor #1604 is merged
successor #1605 is merged
successor counterexample text explicitly names #1604
both touch convex/items.ts
predecessor excerpt states the proxy rule
successor excerpt states the counterexample
successor excerpt states the replacement predicate
prior/replacement semantic values differ
```

## Evaluation states

The research evaluator can return:

```text
predecessor_unmerged
successor_unmerged
successor_does_not_name_predecessor
no_shared_implementation_path
prior_proxy_rule_missing
counterexample_missing
replacement_rule_missing
observed_proxy_revision
```

The retained case evaluates to:

```text
status                           observed_proxy_revision
semantic axis                    responsibility_generation_evidence
prior value                      positive_expected_generation
replacement value                live_unexpired_claim_for_actor
predecessor                      #1604
successor                        #1605
shared path                      convex/items.ts
automatic generalization authority false
```

## Adversarial controls

The ordinary Rust suite requires:

1. unmerged #1604 stays `predecessor_unmerged`;
2. unmerged #1605 stays `successor_unmerged`;
3. successor evidence that never names #1604 stays `successor_does_not_name_predecessor`;
4. removing `convex/items.ts` from either side stays `no_shared_implementation_path`;
5. a missing predecessor rule marker stays `prior_proxy_rule_missing`;
6. a missing successor counterexample marker stays `counterexample_missing`;
7. a missing replacement marker stays `replacement_rule_missing`;
8. identical prior/replacement semantic values reject;
9. invented source excerpts reject against retained project-memory text;
10. claim input is bounded before JSON parsing.

## Replay

```text
cargo run --example proxy_revision -- \
  research/project-memory/stensibly-1604-1605.json \
  research/proxy-revision/stensibly-1604-1605.json
```

## Executed current-main receipt

The semantic code + retained evidence head was:

```text
head:       8d7c5ed05077e242e8013b78cf7cf91d52b89834
main:       6edf32f07138579d87abf5210e84c71e94c1d431
merge view: c25ab8817bfc0528edecb19a7b41679291abd474
```

GitHub Actions receipt:

```text
CI run:                    32259077612  success
Generated provenance run:  32259077595  success
```

The merge-view CI passed:

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

The retained proxy-revision tests passed all ten adversarial controls above. The only change after this semantic receipt is the durable receipt prose in this research note.

## Product direction

Together with the merged lesson-promotion replay, this gives project memory two different temporal semantics:

```text
repeated same-class repairs -> later common guard

accepted proxy -> counterexample -> narrower predicate
```

Both are historical evidence packets first. A later worker can use them to avoid rediscovering already-rejected assumptions, while any prospective generalization still needs its own evidence and acceptance event.

Refs #6 #18 #41 #74 #222.

# Reconstructing a proof-surface mismatch

Stensibly preserves an adversarial dogfood case where the underlying stale-head recovery behavior succeeded while the acceptance proof failed because the verifier produced the wrong GitHub artifact type.

Human-facing external link:

- [Stensibly PR #1515](https://redirect.github.com/teamleaderleo/stensibly/pull/1515)

The retained machine/provider receipt keeps the exact canonical event URL and review state because those fields establish semantic artifact identity.

## Behavior and proof are separate

The #1515 PR body explicitly records both facts:

```text
The A→B stale-head refusal itself was clean.
```

and:

```text
Aborted as a proof target after the verifier accidentally created a PR review COMMENT instead of the required ordinary conversation comment.
```

So the historical episode already separates:

```text
behavioral discriminator
```

from:

```text
acceptance-proof artifact
```

The first succeeded. The second used the wrong semantic surface.

## Provider artifact

The retained provider event has:

```text
id            4943138474
review state  COMMENTED
URL identity  pullrequestreview-4943138474
body          R5Q7 dogfood: mail-originated conversation comment accepted only after current-head refresh.
```

Its body sounds like the desired artifact, which makes this a useful anti-cheat case. Semantic identity comes from the provider event type, not from prose similarity.

The research evaluator classifies:

```text
#pullrequestreview-<id> + review=COMMENTED
  -> pull_request_review

#issuecomment-<id> + no review state
  -> issue_conversation_comment
```

The selected source requirement is bound to the exact phrase:

```text
required ordinary conversation comment
```

## Retained memory

`research/project-memory/stensibly-1515.json` preserves the exact closed/unmerged PR, exact head/base coordinates, complete PR body, and disposable changed path.

`research/proof-surface/stensibly-1515.json` preserves the source excerpts plus the exact provider event receipt.

The retained evaluation is:

```text
status                     observed_proof_surface_mismatch
behavior_passed            true
required artifact          issue_conversation_comment
produced artifact          pull_request_review
proof_valid                false
automatic behavior failure false
automatic acceptance       false
```

## Evaluation states

The evaluator can return:

```text
behavior_evidence_missing
requirement_evidence_missing
provider_event_body_missing
produced_artifact_unclassifiable
proof_surface_matched
observed_proof_surface_mismatch
```

## Adversarial controls

The ordinary Rust suite requires:

1. the retained real event classifies as `pull_request_review` even though its body says “conversation comment”;
2. an exact `issuecomment-<id>` event with no review state satisfies the required surface;
3. missing behavior-success marker stays `behavior_evidence_missing`;
4. retained source excerpt lacking the required ordinary-comment phrase stays `requirement_evidence_missing`;
5. missing provider-event body marker stays `provider_event_body_missing`;
6. URL/ID mismatch stays `produced_artifact_unclassifiable`;
7. provider event bound to another PR stays unclassifiable;
8. required artifact kind and canonical `required ...` source marker must agree;
9. invented source excerpts reject against retained project-memory text;
10. claim input is bounded before JSON parsing.

## Replay

```text
cargo run --example proof_surface -- \
  research/project-memory/stensibly-1515.json \
  research/proof-surface/stensibly-1515.json
```

## Executed current-main receipt

The formatted semantic code + retained evidence head was:

```text
head:       f4b2f389dcd498d5a188ef0f7c1f37e9d1199912
main:       be09462db0f4df2d002bc90d7c5ccc042f737092
merge view: 8ba904ed75ded2a7a03db003649aa96116bdd6b5
```

GitHub Actions receipt:

```text
CI run:                    32261351691  success
Generated provenance run:  32261351575  success
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

The retained proof-surface tests passed all ten adversarial controls above. The existing live GitHub review-memory carrier skipped on its own path filter, so this replay consumed retained evidence only and performed no provider fetch.

The only change after this semantic receipt is the durable receipt prose in this research note.

## Product direction

This adds another temporal/evidence disposition beside the first three historical replays:

```text
repeated repair class -> common guard
accepted proxy -> counterexample -> narrower predicate
authoritative vs lagging observation -> bounded convergence / hard exhaustion
behavior correct + wrong proof artifact type -> proof surface mismatch
```

The useful lesson is local and precise: acceptance evidence can require an exact provider semantic type. Semantically adjacent content does not satisfy that contract automatically.

Refs #15 #18 #41 #74 #222 #229 #233.

# Reconstructing authoritative and lagging observations

Stensibly preserves a production receipt case where two observations disagreed about the deployed Worker version because they had different authority and freshness semantics.

Human-facing external links use `redirect.github.com`:

- [Stensibly PR #1609](https://redirect.github.com/teamleaderleo/stensibly/pull/1609)
- [Stensibly PR #1610](https://redirect.github.com/teamleaderleo/stensibly/pull/1610)

Provider/source text inside retained project-memory evidence remains exact.

## Triggering deployment

Merged #1609 supplied the release coordinate. The later #1610 receipt analysis explicitly says the #1609 deployment had already succeeded at the provider:

```text
Cloudflare provider-current
  exact candidate promoted to 100%

workers.dev public origin
  briefly serves previous Worker version
```

The exact observed values retained by #1610 are:

```text
authoritative/provider-current
  d5bf6ea5-621f-411a-9078-bd097d2e6f63

lagging workers.dev origin
  4c2ea61c-8f96-46c0-a3a7-7d08bf26206f
```

The release itself succeeded; the receipt failed because one public origin had not propagated yet.

## Authority and freshness

#1610 keeps Cloudflare provider-current as the authoritative exact-version fence. It does not discard public-origin evidence: receipt emission still waits for both public origins to report the exact provider-current version.

The policy therefore distinguishes:

```text
what version is authoritative as provider current?
```

from:

```text
has every public observation surface converged to that version yet?
```

A temporary disagreement is modeled as bounded convergence rather than immediate contradiction.

## Bounded convergence and exhaustion

The merged repair changes:

```text
scripts/worker-production-receipt.ts
test/worker-production-receipt.test.ts
```

and explicitly defines:

```text
up to 8 public-origin observations
5-second bounded spacing
no receipt until origins match provider current
hard failure when an origin never converges
```

The focused tests cover both sides:

```text
temporarily stale origin
-> converges on later observation
-> receipt succeeds

origin remains stale through bound
-> receipt still fails
```

So bounded convergence does not weaken the eventual equality obligation.

## Retained packet

`research/project-memory/stensibly-1609-1610.json` preserves the exact merged predecessor and reconciler with complete retained PR text, exact head/base identities, and changed paths. It carries zero generic relationship edges.

The specialized claim in `research/observation-reconciliation/stensibly-1609-1610.json` must independently establish:

```text
predecessor and reconciler merged
reconciler explicitly names #1609 deployment
provider-current authority declaration
exact provider-current value
exact stale public-origin value
bounded convergence policy
permanent-divergence failure policy
implementation path
focused test path
```

## Evaluation states

The evaluator can return:

```text
predecessor_unmerged
reconciler_unmerged
reconciler_does_not_name_predecessor
authority_rule_missing
divergent_observation_missing
convergence_policy_missing
permanent_divergence_control_missing
implementation_path_missing
test_path_missing
observed_reconciliation
```

The retained case evaluates to:

```text
status                  observed_reconciliation
semantic axis           worker_deployed_version_observation
authoritative source    cloudflare_provider_current
lagging source          workers_dev_public_origin
temporary disagreement  bounded_convergence
persistent disagreement hard_failure
automatic authority change false
```

## Adversarial controls

The ordinary Rust suite requires:

1. merged predecessor;
2. merged reconciler;
3. explicit #1609 naming;
4. explicit provider-current authority marker;
5. both divergent exact values in retained evidence;
6. explicit bounded retry marker;
7. explicit exhaustion/hard-failure marker;
8. reconciler implementation path;
9. reconciler focused test path;
10. different authority/lagging source identities;
11. different authority/lagging observed values;
12. invented source evidence rejects;
13. malformed semantic identities reject through shape validation;
14. bounded input before JSON parsing.

## Replay

```text
cargo run --example observation_reconciliation -- \
  research/project-memory/stensibly-1609-1610.json \
  research/observation-reconciliation/stensibly-1609-1610.json
```

## Executed current-main receipt

The formatted semantic code + retained evidence head was:

```text
head:       7174484426834a932ee54f90c33699e48a1c0b9a
main:       a76fae2e768e6429a55260a5177500a8e0be79ff
merge view: 9d434166dd375af005270922e459df564506cad1
```

GitHub Actions receipt:

```text
CI run:                    32260151598  success
Generated provenance run:  32260151606  success
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

The retained observation-reconciliation tests passed all fourteen adversarial controls above. The only change after this semantic receipt is the durable receipt prose in this research note.

## Product direction

Together with the previous two temporal replays, project memory can now represent:

```text
repeated repair class -> common guard
accepted proxy -> counterexample -> narrower predicate
authoritative observation + lagging observation -> bounded convergence / hard exhaustion
```

This third form is especially useful for distributed/provider systems where evidence surfaces have different authority and freshness. It preserves disagreement long enough to classify it correctly instead of flattening everything into one boolean consistency check.

Refs #18 #41 #74 #222 #229.

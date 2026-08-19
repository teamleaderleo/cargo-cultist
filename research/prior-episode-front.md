# Actionable prior-episode front

Issue #217 tests a narrow delivery question over evidence Cultist has already earned:

> Can the smallest historical fact that changes the first justified inspection appear at the front of task context without replacing the underlying evidence contracts?

The answer remains a research-only composition layer.

## Source semantics remain separate

The front does not decide whether review evidence is current, whether an issue was cleared, whether repeated repairs earned a guard, whether a proxy lost authority, whether a distributed observation is still converging, or whether a provider proof artifact satisfies an exact evidence contract.

It calls the existing source evaluators directly:

```text
ReviewMemoryQuery
  -> evaluate_review_memory()

IssueClosureEpisode
  -> evaluate_closure_episode()

ProjectMemoryPacket + LessonPromotionClaim
  -> evaluate_lesson_promotion()

ProjectMemoryPacket + ProxyRevisionClaim
  -> evaluate_proxy_revision()

ProjectMemoryPacket + ObservationReconciliationClaim
  -> evaluate_observation_reconciliation()

ProjectMemoryPacket + ProofSurfaceClaim
  -> evaluate_proof_surface()
```

The complete source evaluation is retained inside every surfaced front item.

The useful facts therefore stay typed before task-facing projection:

```text
review memory
  concern lineage
  exact-head applicability
  prior outcome
  reuse / refresh / need-context / new-thread disposition

issue closure
  lifecycle state
  closure kind
  explicit re-report relation
  clearance UNKNOWN
  inspect-prior-failure disposition

lesson promotion
  repeated same-class repairs
  adjacent different-class lineage
  merged common guard

proxy revision
  accepted prior inference
  explicit counterexample
  narrower successor predicate

observation reconciliation
  authoritative observation source
  lagging observation source
  bounded convergence
  hard exhaustion

proof surface
  behavioral success
  required provider artifact kind
  produced provider artifact kind
  proof validity
```

The front adds only a task-facing next-action projection.

## Contract

`src/prior_episode_front.rs` accepts a bounded list of tagged source inputs:

```text
review_memory
  packet-local id
  exact ReviewMemoryQuery

issue_closure
  packet-local id
  exact IssueClosureEpisode

lesson_promotion
  packet-local id
  exact ProjectMemoryPacket
  exact LessonPromotionClaim

proxy_revision
  packet-local id
  exact ProjectMemoryPacket
  exact ProxyRevisionClaim

observation_reconciliation
  packet-local id
  exact ProjectMemoryPacket
  exact ObservationReconciliationClaim

proof_surface
  packet-local id
  exact ProjectMemoryPacket
  exact ProofSurfaceClaim
```

The front admits at most 64 inputs inside a 512 KiB transport boundary.

The output has two lists:

```text
items
  historical evidence that earned front-of-context delivery

quiet
  admitted review-memory inputs that correctly produced no prior-episode item
```

There is no scalar score and no generic ranking. Surfaced items preserve admitted input order.

The query is deserializable. The evaluated front is a rendered output and remains serialize-only; source evaluator result types keep their original contracts.

## Review projection

Source disposition maps narrowly:

```text
reuse_current_thread
  -> reuse_existing_review_thread

refresh_existing_thread
  -> recompute_and_refresh_review_thread

need_context + actual prior lineage
  -> acquire_missing_review_coordinate

new_thread
  -> quiet receipt

need_context + zero prior lineage
  -> quiet receipt
```

The last distinction prevents a missing current head from manufacturing a fake historical episode when there is no prior matching concern.

Quiet receipts retain the complete `ReviewMemoryEvaluation`, including unrelated same-key records when present.

## Closure projection

An admitted issue-closure episode currently has one earned source disposition:

```text
inspect_prior_failure
```

The front maps it to:

```text
inspect_prior_failure_and_rereport
```

and retains:

- the complete `IssueClosureEvaluation`;
- exact closure source reference;
- exact re-report source reference;
- duplicate-suggestion/rejection source references when that optional source evidence exists.

Closing the later re-report does not remove this item because the source evaluator still reports clearance as `UNKNOWN`.

## Temporal disposition projection

Caller-selected temporal episodes are admitted only when their source evaluator already reached the fully observed historical disposition.

The front maps them directly:

```text
lesson promotion
  observed_promotion
  -> use_accepted_guard

proxy revision
  observed_proxy_revision
  -> use_corrected_predicate

observation reconciliation
  observed_reconciliation
  -> await_bounded_convergence

proof surface
  observed_proof_surface_mismatch
  -> produce_required_proof_artifact
```

This is a projection, not a new interpretation layer.

If a selected temporal input evaluates to an incomplete state such as:

```text
guard_coverage_incomplete
counterexample_missing
convergence_policy_missing
proof_surface_matched
```

then the front rejects that selected input and names its packet-local ID. The caller must resolve the source evidence/question before treating it as an actionable prior episode.

The front does not discover which temporal episode is relevant to the current task. Selection remains upstream.

## Real carrier A: PR-Agent review memory

Retained input:

```text
research/prior-episode-front/pr-agent-2424.json
```

Human source: [PR-Agent PR #2424](https://redirect.github.com/The-PR-Agent/pr-agent/pull/2424).

The exact coordinates were already executed and retained by merged #206:

```text
root review comment
  3355870564

reviewed head
  8fb9e4e86b4794d39afba2d62413571cbc04a744

resolution reply
  3355925719

current PR head
  f6070fb1a45516565bbb8deeb02a1f66cec13d91

path
  pr_agent/git_providers/github_provider.py
```

The source review outcome is `patch_changed` because the selected direct reply is the retained resolution evidence used by #206.

Expected front:

```text
source disposition
  refresh_existing_thread

prior outcome applicability
  INVALID

front next
  recompute_and_refresh_review_thread
```

The front therefore preserves both halves of the desired behavior:

```text
remember the concern/thread
expire the old resolution on the new head
```

## Real carrier B: Claude Code closure/re-report

Retained input:

```text
research/prior-episode-front/claude-code-57507.json
```

Human sources: [Claude Code #31294](https://redirect.github.com/anthropics/claude-code/issues/31294) and [#57507](https://redirect.github.com/anthropics/claude-code/issues/57507).

Merged #212 already executed the provider carrier and retained:

```text
#31294
  closed by github-actions[bot]
  state_reason not_planned
  exact administrative-inactivity closure comment 4230270046

#57507
  explicitly re-reports #31294
  later also closed
```

The canonical closure evidence intentionally retains the original `github.com/.../issues/new/choose` URL because exact source text is part of the admitted closure classifier. Human-facing links in this note use `redirect.github.com` under the repository link policy.

Expected front:

```text
closure kind
  administrative_inactive

re-report
  observed

later state
  closed

clearance
  UNKNOWN

front next
  inspect_prior_failure_and_rereport
```

This keeps `closed` useful without treating it as a repair receipt.

## Real temporal composition: four Stensibly dispositions

The real composition control uses the four already-landed Stensibly replay packets.

Human sources:

- guard promotion: [#1571](https://redirect.github.com/teamleaderleo/stensibly/pull/1571), [#1573](https://redirect.github.com/teamleaderleo/stensibly/pull/1573), [#1575](https://redirect.github.com/teamleaderleo/stensibly/pull/1575);
- proxy revision: [#1604](https://redirect.github.com/teamleaderleo/stensibly/pull/1604), [#1605](https://redirect.github.com/teamleaderleo/stensibly/pull/1605);
- observation reconciliation: [#1609](https://redirect.github.com/teamleaderleo/stensibly/pull/1609), [#1610](https://redirect.github.com/teamleaderleo/stensibly/pull/1610);
- proof-surface mismatch: [#1515](https://redirect.github.com/teamleaderleo/stensibly/pull/1515).

The combined query is serialized, reparsed, and evaluated in this admitted order:

```text
1. index-limit guard promotion
2. responsibility-generation proxy revision
3. Worker origin convergence
4. R5Q7 proof-surface mismatch
```

Expected front:

```text
1. use_accepted_guard
2. use_corrected_predicate
3. await_bounded_convergence
4. produce_required_proof_artifact
```

Every front item retains the complete source evaluator output, including its zero-automatic-authority fields.

A negative control removes #1573 from the selected guard's coverage. The source evaluator becomes `guard_coverage_incomplete`; the front rejects the selected episode and names `stensibly:index-limit-guard` instead of emitting `use_accepted_guard`.

That is the composition property this experiment needs: the front cannot make an incomplete historical story actionable merely because the caller selected it.

## Executed temporal composition receipt

The formatted semantic composition head was:

```text
head:       102d9ce644928a8d93977e1ac3a3d11eae7888b7
main:       660f7069a264281a7de07dd2b06caf32863c5ad5
merge view: ce9bf0f3bcdfc3ef803c0cb68976c8750e976481
```

GitHub Actions receipt:

```text
CI run:                    32262851994  success
Generated provenance run:  32262851951  success
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

The four temporal inputs serialized and reparsed through the front query grammar and surfaced in exact input order:

```text
use_accepted_guard
use_corrected_predicate
await_bounded_convergence
produce_required_proof_artifact
```

The incomplete-guard mutation failed closed with packet-local ID `stensibly:index-limit-guard` and source status `GuardCoverageIncomplete`.

The existing live review-memory and issue-closure carrier workflows skipped on their path filters for this semantic head; temporal composition itself performed no provider fetch.

The only change after this semantic receipt is the durable receipt prose in this research note.

## Quiet receipts are part of the experiment

The front intentionally records why a review input stayed out of `items`:

```text
no_prior_review_lineage
  no matching historical concern exists

no_current_review_lineage
  historical same-key records exist but belong to another work/scope lineage
```

That gives #137 a negative-control receipt. An A/B packet can prove that the front stayed quiet on an unrelated review instead of merely omitting it invisibly.

Temporal inputs currently fail closed when selected but incomplete. Future temporal quiet receipts should be earned only if a caller needs to submit candidate-but-nonactionable history intentionally.

## Standard controls

`tests/prior_episode_front.rs` requires:

- moved-head review -> recompute + refresh and preserved INVALID applicability;
- exact-head review -> reuse current thread;
- missing coordinate with prior lineage -> acquire coordinate;
- missing coordinate without prior lineage -> quiet;
- unrelated work lineage -> quiet with source evaluation retained;
- explicit closure/re-report -> inspect with clearance UNKNOWN;
- later re-report closure does not remove the front item;
- empty input -> empty front + quiet set;
- surfaced item order follows admitted input order;
- duplicate packet-local IDs reject before projection;
- source evaluator errors name the packet-local input ID;
- unknown machine fields fail closed.

`tests/prior_episode_front_real.rs` parses the two retained external carrier fixtures and asserts the same review/closure source semantics under the composition layer.

`tests/prior_episode_front_temporal.rs` serializes and reparses a four-species Stensibly query, requires exact next-action order, preserves all four source evaluations, and proves an incomplete selected guard fails closed with the packet-local input ID.

## Behavioral gate

This slice does not claim the front changed worker behavior.

The retained carriers become held-out treatment packets for #137.

Candidate A/B outcomes to observe:

```text
PR-Agent review carrier
  prevented duplicate review interruption
  prevented stale resolution reuse
  reused an existing thread while recomputing current evidence

Claude closure carrier
  prevented closed-issue = fixed assumption
  opened the prior reproduction or later discriminator earlier
  reduced repeated issue archaeology

Stensibly temporal carriers
  used an already-accepted guard instead of rediscovering the rule
  avoided a rejected proxy inference
  waited for bounded propagation instead of declaring contradiction or success too early
  produced the exact required proof artifact instead of a semantically adjacent one
```

A useful behavioral receipt must name the concrete next action. Until then the front remains research output.

## Boundary

- research only;
- no network/provider calls in the composition evaluator;
- no relevance discovery or similarity search;
- no issue-title or review-text inference;
- no concern-key generation;
- no confidence/risk score;
- no universal ordering beyond admitted input order;
- no change to ordinary `cargo-cultist` commands or `AnalysisReport`;
- no review, merge, issue-reopen, provider, policy, or external-effect authority;
- temporal `next` values are task-facing context, not executable commands.

The front is allowed to stay small because the full typed source evidence remains attached and independently inspectable.

Refs #18 #41 #62 #67 #109 #123 #137 #192 #198 #202 #206 #207 #212 #217 #222 #229 #233 #236.

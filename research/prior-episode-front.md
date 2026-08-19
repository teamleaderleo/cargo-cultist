# Actionable prior-episode front

Issue #217 tests a narrow delivery question over evidence Cultist has already earned:

> Can the smallest historical fact that changes the first justified inspection appear at the front of task context without replacing the underlying review/closure evidence contracts?

The v0 answer is a research-only composition layer.

## Source semantics remain separate

The front does not decide whether review evidence is current and does not decide whether an issue was cleared.

It calls the existing source evaluators directly:

```text
ReviewMemoryQuery
  -> evaluate_review_memory()

IssueClosureEpisode
  -> evaluate_closure_episode()
```

The complete source evaluation is retained inside every surfaced front item.

This is important because the useful facts are already typed:

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
```

V0 admits at most 64 inputs inside a 512 KiB transport boundary.

The output has two lists:

```text
items
  historical evidence that earned front-of-context delivery

quiet
  admitted review-memory inputs that correctly produced no prior-episode item
```

There is no scalar score and no generic ranking. Surfaced items preserve admitted input order.

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

## Quiet receipts are part of the experiment

The front intentionally records why a review input stayed out of `items`:

```text
no_prior_review_lineage
  no matching historical concern exists

no_current_review_lineage
  historical same-key records exist but belong to another work/scope lineage
```

That gives #137 a negative-control receipt. An A/B packet can prove that the front stayed quiet on an unrelated review instead of merely omitting it invisibly.

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

`tests/prior_episode_front_real.rs` parses the two retained external carrier fixtures and asserts the same source semantics under the composition layer.

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
```

A useful behavioral receipt must name the concrete next action. Until then the front remains research output.

## Boundary

- research only;
- no network/provider calls in the composition evaluator;
- no similarity search;
- no issue-title or review-text inference;
- no concern-key generation;
- no confidence/risk score;
- no universal ordering beyond admitted input order;
- no change to ordinary `cargo-cultist` commands or `AnalysisReport`;
- no review, merge, issue-reopen, or external-effect authority.

The front is allowed to be small precisely because the full typed source evidence remains attached and independently inspectable.

Refs #18 #62 #67 #109 #123 #137 #192 #198 #202 #206 #207 #212 #217.

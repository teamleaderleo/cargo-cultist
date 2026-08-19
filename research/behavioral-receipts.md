# Behavioral evidence receipts

Status: research experiment for #137.

Cultist already records what evidence establishes. This experiment records a different fact about dogfood/evaluation episodes:

> What happened to the worker's next action after a specific evidence candidate surfaced or correctly stayed quiet?

The first schema is deliberately small and inspectable. It does not produce a score, rank an analyzer, or grant authority.

## Receipt v1

A receipt binds one evidence episode to:

- repository identity;
- exact 40-hex Git revision;
- task identity;
- evidence kind and evidence reference;
- delivery state: `surfaced` or `quiet`;
- whether the surfaced evidence was consulted;
- one typed behavioral outcome;
- one concrete changed next action when the outcome requires it.

Current outcome vocabulary:

```text
changed_next_action
prevented_or_reversed_wrong_turn
useful_same_action
ignored
irrelevant
stale_or_wrong_coordinate
needed_stronger_evidence
correct_quiet_negative
```

The vocabulary describes the observed episode. It does not claim why an analyzer behaved that way or whether the same result generalizes to another task.

## Fail-closed combinations

The validator rejects combinations that would blur the behavioral claim.

Examples:

```text
quiet + consulted
  -> invalid

quiet + changed_next_action
  -> invalid

surfaced + correct_quiet_negative
  -> invalid

ignored + consulted
  -> invalid

changed_next_action + no concrete action
  -> invalid

useful_same_action + action
  -> invalid
```

A `stale_or_wrong_coordinate` or `needed_stronger_evidence` receipt requires a concrete action because the useful behavioral fact is the recovery step the evidence forced.

## Research validator

```text
cargo run --example behavioral_receipt < receipt.json
```

Input is bounded to 64 KiB. Unknown JSON fields, unsupported schema versions, non-exact revisions, malformed coordinates, and impossible outcome combinations reject explicitly.

The validator prints the admitted typed receipt as JSON. It does not append to a database or mutate repository state.

## First retained positive

`research/behavioral-receipts/collaboration-140.json` records the collaboration episode from product PR #140.

During that lane, `main` advanced while the branch was in progress. The new exact head was inspected, the landed file changes were checked, and the next action changed: rebuild the lane on current `main` and recheck live PR overlap before opening the PR.

That is a useful first positive because the causal chain is directly observable:

```text
new current-main evidence surfaced
-> evidence consulted
-> branch plan changed
-> stale-base integration risk avoided
```

It does not prove every head movement deserves an interruption. Quiet/disjoint controls remain necessary.

## Relationship to behavioral A/B evaluation

The receipt is a per-episode primitive. A later #137 A/B harness can compare tasks with and without selected Cultist evidence using collections of these receipts plus independent completion/inspection measurements.

Useful aggregate questions can be computed later from raw receipts:

```text
How often was evidence consulted?
How often did it change the next action?
Which evidence families were repeatedly irrelevant?
Which families exposed stale coordinates?
Which candidates correctly stayed quiet?
```

Keep the raw episodes available so an aggregate never becomes the only explanation.

## Boundary

- v1 is Git-revision-bound;
- evidence references are caller-supplied coordinates, not durable Cultist finding IDs;
- action text is a bounded receipt, not an executable command or authority grant;
- no model attribution or worker capability taxonomy;
- no universal actionability score;
- no automatic analyzer promotion/demotion;
- no claim that `consulted` proves the evidence caused a successful final outcome.

North star:

> Preserve enough behavioral evidence to tell whether an interruption changed the work, without turning that observation into a hidden policy.

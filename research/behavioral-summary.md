# Descriptive behavioral summary

Tracking: #137, building on merged #157 / #165 / #169.

Cultist now has a small retained set of uniquely identified behavioral evidence episodes. Before using those episodes to promote, demote, or rank anything, this experiment asks a narrower question:

> Can we summarize what happened while keeping every count inspectable and avoiding a universal actionability score?

## Summary v1

The summary compiles one already-validated `BehavioralEpisodeBatch` into:

```text
total_episodes
surfaced
quiet
consulted
by_outcome[]
by_evidence_kind[]
```

Every grouped count carries:

```text
key
count
episode_ids[]
```

so a consumer can always inspect exactly which retained episodes produced the number.

No weighting is applied. `changed_next_action=2` and `correct_quiet_negative=1` remain different observed outcomes rather than points in one score.

## Current retained Cultist corpus

`research/behavioral-episodes/cultist-collaboration-v1.json` currently contains three unique episodes:

```text
2 surfaced
1 quiet
2 consulted

2 changed_next_action
1 correct_quiet_negative
```

Evidence families remain separate:

```text
active-work-heads-up
concurrent-main-head-movement
project-memory-relation-strengthening
```

This sample is deliberately tiny. The counts describe these three episodes only.

## Research command

```text
cargo run --example behavioral_summary \
  < research/behavioral-episodes/cultist-collaboration-v1.json
```

The input batch is revalidated through the merged episode/receipt contracts before summarization.

## What this does not justify

The summary does not answer:

- whether one evidence family is globally good or bad;
- whether an action change improved the final task result;
- whether a quiet negative is worth the same attention/cost as an action-changing positive;
- whether a three-episode sample supports product promotion;
- whether one worker/model population generalizes to another;
- whether two different tasks are comparable.

Those require additional experimental design and held-out tasks under #137.

## Next discriminator

The useful next step is more raw evidence, especially episodes that fill currently empty outcomes:

```text
useful_same_action
ignored
irrelevant
stale_or_wrong_coordinate
needed_stronger_evidence
prevented_or_reversed_wrong_turn
```

Only after enough varied episodes exist should Cultist test any higher-level promotion/demotion view. Even then, retain raw episode IDs beside every aggregate.

North star:

> Count observed episodes without letting the count become the explanation.

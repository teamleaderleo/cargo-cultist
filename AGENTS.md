# Agent guidance

Cultist is built by dogfooding the same repository-reasoning ideas it is trying to make useful for other projects.

While working here, do the requested task first. At the same time, pay attention to evidence that the work itself exposes about how Cultist could improve.

## Feed useful friction back into the project

If the task reveals something that made the work harder, less reliable, more repetitive, or easier to misunderstand, ask whether Cultist could have surfaced that fact earlier or preserved the lesson for the next worker.

Useful signals include:

- two workers or branches independently doing overlapping work;
- a repository fact, convention, generated relationship, or policy that was important but easy to miss;
- a false assumption that later evidence disproves;
- a stale or wrong evidence coordinate;
- a historical reason that had to be reconstructed manually;
- a useful counterexample that weakens an existing heuristic;
- an analyzer finding that was noisy, overconfident, incomplete, or missing an important `UNKNOWN`;
- repeated investigation that could become a deterministic probe;
- a reviewed lesson that future agents would benefit from seeing before making the same edit.

Do not turn every inconvenience into a feature. Preserve the distinction between:

- an observation from this task;
- a plausible reusable pattern;
- a tested discriminator with counterexamples;
- a product behavior that has earned promotion.

## Prefer evidence over retrospective stories

When proposing a Cultist improvement from dogfood:

1. identify the exact task, change, file, issue, PR, commit, or repository evidence that exposed it;
2. state what is **PROVEN**, **DERIVED**, **OBSERVED**, **INFERRED**, or **UNKNOWN**;
3. search for a counterexample or negative control before generalizing;
4. keep chronology separate from causality unless an explicit relationship establishes it;
5. prefer a small deterministic probe or evidence packet before a broad heuristic;
6. preserve failed experiments and weakened hypotheses when they teach a boundary.

A successful task can still expose a product problem. A failed task can still produce useful evidence. Neither automatically proves a general rule.

## Close the loop when it is useful

When the evidence is strong enough and the task scope permits it, incorporate the lesson into Cultist through the smallest appropriate durable surface:

- improve an existing analyzer or evidence contract;
- add a focused regression or negative control;
- record a research receipt;
- update a relevant roadmap/research issue;
- preserve reviewed rationale for future context/decision memory;
- open a focused follow-up issue when the lesson is real but outside the current task.

Avoid unrelated implementation expansion merely because an idea occurred during the task. If the lesson belongs elsewhere, leave an exact handoff with its evidence instead of silently widening scope.

## Agent-facing product test

A recurring question for work in this repository is:

> What did this task force the worker to discover manually that Cultist could have surfaced, bounded, or preserved for the next worker?

If the answer is meaningful, treat the task as dogfood evidence and carry the lesson forward with provenance.

The goal is a repository that becomes easier to work in because prior workers leave behind earned, inspectable knowledge—not because later workers inherit their conversations or trust their conclusions blindly.

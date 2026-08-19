# Held-out replay state in refinement investigation demand

Tracking: #248. This note resolves the review edge raised on PR #251 before downstream probe planning treats a selected candidate as investigation-eligible.

## Contract

At the investigation-demand layer, **replay survival is status-level non-rejection**:

```text
Retained | Weakened | Split
  -> surviving candidate status

RejectedNoImprovement | RejectedOverfit | RejectedLostExpectedCase
  -> replay_rejected
```

Held-out replay completion remains an independent receipt axis. The refinement-episode validator already permits kept candidates with:

```text
held_out_status = passed | not_run | unknown
```

and rejects kept candidates with:

```text
held_out_status = failed
```

Investigation demand therefore does not promote `not_run` or `unknown` to `passed`, and it does not invent a second rejection rule for them.

## Visibility requirement

`RefinementInvestigationDisposition` carries the complete source `ReplayResult` alongside the existing convenience `replay_status` field.

That preserves:

```text
expected cases retained/lost
counterexamples resolved/remaining
held_out_status
```

for downstream readers such as #255. A caller can distinguish:

```text
selected survivor + held_out passed
selected survivor + held_out not_run
selected survivor + held_out unknown
```

while receiving the same current investigation disposition when the evidence state is otherwise identical.

## Controls

`tests/refinement_investigation_held_out_contract.rs` proves two explicit cases:

1. selected Oxc survivor + current evidence + held-out `unknown` -> `satisfied`, with `unknown` preserved in the emitted replay result;
2. selected Oxc survivor + exact mapped observation missing + held-out `not_run` -> `observation_acquisition_needed`, with `not_run` preserved and the exact missing subject frontier emitted.

This makes the policy explicit without claiming held-out replay completion.

## Boundary

- no held-out `not_run`/`unknown` -> `passed` rewrite;
- no new candidate selection or promotion authority;
- no probe execution authority;
- downstream planning may apply a stricter held-out prerequisite later, but it must do so explicitly from the preserved replay result rather than inferring completion from `replay_status`.

Refs #179 #243 #248 #251 #255.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrialSpec {
    schema_version: u32,
    trial_id: String,
    repository: String,
    revision: String,
    target_path: String,
    target_blob_sha: String,
    worker_task: WorkerTask,
    oracle: Oracle,
    conditions: Vec<Condition>,
    oracle_leak_control: OracleLeakControl,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerTask {
    prompt: String,
    patch: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Oracle {
    expected_disposition: String,
    blocking_reason: String,
    max_identifier_length: usize,
    proposed_identifier: String,
    proposed_identifier_length: usize,
    corrective_action: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PacketKind {
    FileLocal,
    Scoped,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Condition {
    id: String,
    packet_kind: PacketKind,
    budget_bytes: usize,
    scope: Option<String>,
    decisive_evidence_present: bool,
    decisive_evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleLeakControl {
    historical_issue: String,
    allowed_as_worker_prompt: bool,
    prohibited_worker_prompt_fragments: Vec<String>,
}

fn spec() -> TrialSpec {
    serde_json::from_slice(include_bytes!(
        "../research/capability-demand-retirement/stensibly-convex-index-review-v1.json"
    ))
    .expect("valid capability-demand retirement trial JSON")
}

fn is_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[test]
fn held_out_trial_is_exact_non_leaky_and_uses_a_new_failure_instance() {
    let spec = spec();

    assert_eq!(spec.schema_version, 1);
    assert_eq!(spec.trial_id, "stensibly-convex-index-review-v1");
    assert_eq!(spec.repository, "teamleaderleo/stensibly");
    assert_eq!(spec.revision, "85cecf2608ad9e734a67518577fa85b9a08a550c");
    assert_eq!(spec.target_path, "convex/schema.ts");
    assert_eq!(
        spec.target_blob_sha,
        "7fdd51e2f9fba80d1c0a814cea708d601a7b9925"
    );
    assert!(is_sha(&spec.revision));
    assert!(is_sha(&spec.target_blob_sha));

    assert_eq!(spec.oracle.expected_disposition, "block");
    assert_eq!(spec.oracle.blocking_reason, "convex_index_identifier_limit");
    assert_eq!(spec.oracle.max_identifier_length, 64);
    assert_eq!(
        spec.oracle.proposed_identifier.chars().count(),
        spec.oracle.proposed_identifier_length
    );
    assert!(spec.oracle.proposed_identifier_length > spec.oracle.max_identifier_length);
    assert_eq!(
        spec.oracle.corrective_action,
        "shorten_identifier_preserve_field_order"
    );
    assert!(
        spec.worker_task
            .patch
            .contains(&spec.oracle.proposed_identifier)
    );

    let historical_names = [
        "by_workspace_id_and_provider_and_mailbox_binding_id_and_provider_message_id",
        "by_workspace_id_and_provider_and_account_binding_and_mailbox_address_and_provider_thread_id_and_provider_message_id",
    ];
    for historical in historical_names {
        assert_ne!(spec.oracle.proposed_identifier, historical);
        assert!(!spec.worker_task.patch.contains(historical));
    }

    let field_positions = [
        "projectId",
        "issueExternalId",
        "sourceRevision",
        "instructionSetSha256",
        "providerUpdatedAt",
    ]
    .map(|field| {
        spec.worker_task
            .patch
            .find(field)
            .expect("field in proposed patch")
    });
    assert!(field_positions.windows(2).all(|pair| pair[0] < pair[1]));

    assert!(!spec.oracle_leak_control.allowed_as_worker_prompt);
    assert_eq!(
        spec.oracle_leak_control.historical_issue,
        "https://github.com/teamleaderleo/stensibly/issues/1574"
    );
    let prompt_lower = spec.worker_task.prompt.to_ascii_lowercase();
    for fragment in &spec.oracle_leak_control.prohibited_worker_prompt_fragments {
        assert!(
            !prompt_lower.contains(&fragment.to_ascii_lowercase()),
            "worker prompt leaked oracle fragment {fragment:?}"
        );
    }
}

#[test]
fn paired_conditions_change_evidence_not_repository_task_or_budget() {
    let spec = spec();
    assert_eq!(spec.conditions.len(), 2);

    let conditions = spec
        .conditions
        .iter()
        .map(|condition| (condition.id.as_str(), condition))
        .collect::<BTreeMap<_, _>>();
    let baseline = conditions
        .get("file_local_jei")
        .expect("file-local baseline condition");
    let treatment = conditions
        .get("scoped_jei")
        .expect("scoped JEI treatment condition");

    assert_eq!(baseline.packet_kind, PacketKind::FileLocal);
    assert_eq!(baseline.budget_bytes, 32_768);
    assert!(baseline.scope.is_none());
    assert!(!baseline.decisive_evidence_present);
    assert!(baseline.decisive_evidence_refs.is_empty());

    assert_eq!(treatment.packet_kind, PacketKind::Scoped);
    assert_eq!(treatment.budget_bytes, baseline.budget_bytes);
    assert_eq!(treatment.scope.as_deref(), Some("convex"));
    assert!(treatment.decisive_evidence_present);

    let refs = treatment
        .decisive_evidence_refs
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        refs,
        BTreeSet::from([
            "85cecf2608ad9e734a67518577fa85b9a08a550c",
            "ca5d2c7fdf89666e523972ab6e81610d17b9611b",
        ])
    );
    assert!(refs.iter().all(|reference| is_sha(reference)));
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ReplayIdentity {
    repository_revision: &'static str,
    target_blob: &'static str,
    task_fingerprint: &'static str,
    patch_fingerprint: &'static str,
    worker_identity: &'static str,
    harness_identity: &'static str,
    affordance_identity: &'static str,
    completion_contract_fingerprint: &'static str,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RunOutcome {
    Success,
    Failed,
    CorrectEscalation,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct RunReceipt {
    identity: ReplayIdentity,
    decisive_evidence_present: bool,
    outcome: RunOutcome,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RetirementVerdict {
    ObservedPairedRetirement,
    CorrectEscalationThenSuccess,
    NoDemandObserved,
    DemandPersists,
    Confounded,
    InvalidEvidencePair,
}

fn evaluate_pair(baseline: &RunReceipt, treatment: &RunReceipt) -> RetirementVerdict {
    if baseline.identity != treatment.identity {
        return RetirementVerdict::Confounded;
    }
    if baseline.decisive_evidence_present || !treatment.decisive_evidence_present {
        return RetirementVerdict::InvalidEvidencePair;
    }

    match (baseline.outcome, treatment.outcome) {
        (RunOutcome::Failed, RunOutcome::Success) => RetirementVerdict::ObservedPairedRetirement,
        (RunOutcome::CorrectEscalation, RunOutcome::Success) => {
            RetirementVerdict::CorrectEscalationThenSuccess
        }
        (RunOutcome::Success, _) => RetirementVerdict::NoDemandObserved,
        _ => RetirementVerdict::DemandPersists,
    }
}

fn fixed_identity() -> ReplayIdentity {
    ReplayIdentity {
        repository_revision: "repo@85cecf2",
        target_blob: "schema@7fdd51e",
        task_fingerprint: "task-v1",
        patch_fingerprint: "patch-v1",
        worker_identity: "worker-a@v1",
        harness_identity: "harness-a@v1",
        affordance_identity: "read-only-review-tools@v1",
        completion_contract_fingerprint: "block-index-limit@v1",
    }
}

fn run(
    identity: ReplayIdentity,
    decisive_evidence_present: bool,
    outcome: RunOutcome,
) -> RunReceipt {
    RunReceipt {
        identity,
        decisive_evidence_present,
        outcome,
    }
}

fn assert_confounded(changed: ReplayIdentity) {
    let baseline = run(fixed_identity(), false, RunOutcome::Failed);
    let treatment = run(changed, true, RunOutcome::Success);
    assert_eq!(
        evaluate_pair(&baseline, &treatment),
        RetirementVerdict::Confounded
    );
}

#[test]
fn fixed_worker_failure_then_treatment_success_is_local_retirement_evidence() {
    let baseline = run(fixed_identity(), false, RunOutcome::Failed);
    let treatment = run(fixed_identity(), true, RunOutcome::Success);

    assert_eq!(
        evaluate_pair(&baseline, &treatment),
        RetirementVerdict::ObservedPairedRetirement
    );
}

#[test]
fn baseline_success_means_no_success_capability_demand_was_observed() {
    let baseline = run(fixed_identity(), false, RunOutcome::Success);
    let treatment = run(fixed_identity(), true, RunOutcome::Success);

    assert_eq!(
        evaluate_pair(&baseline, &treatment),
        RetirementVerdict::NoDemandObserved
    );
}

#[test]
fn correct_baseline_escalation_then_treatment_success_is_not_a_model_mistake() {
    let baseline = run(fixed_identity(), false, RunOutcome::CorrectEscalation);
    let treatment = run(fixed_identity(), true, RunOutcome::Success);

    assert_eq!(
        evaluate_pair(&baseline, &treatment),
        RetirementVerdict::CorrectEscalationThenSuccess
    );
}

#[test]
fn treatment_failure_preserves_residual_capability_demand() {
    let baseline = run(fixed_identity(), false, RunOutcome::Failed);
    let treatment = run(fixed_identity(), true, RunOutcome::Failed);

    assert_eq!(
        evaluate_pair(&baseline, &treatment),
        RetirementVerdict::DemandPersists
    );
}

#[test]
fn every_frozen_identity_axis_can_confound_a_retirement_claim() {
    let mut changed = fixed_identity();
    changed.repository_revision = "repo@different";
    assert_confounded(changed);

    let mut changed = fixed_identity();
    changed.target_blob = "schema@different";
    assert_confounded(changed);

    let mut changed = fixed_identity();
    changed.task_fingerprint = "task-v2";
    assert_confounded(changed);

    let mut changed = fixed_identity();
    changed.patch_fingerprint = "patch-v2";
    assert_confounded(changed);

    let mut changed = fixed_identity();
    changed.worker_identity = "worker-b@v1";
    assert_confounded(changed);

    let mut changed = fixed_identity();
    changed.harness_identity = "harness-b@v1";
    assert_confounded(changed);

    let mut changed = fixed_identity();
    changed.affordance_identity = "expanded-tools@v2";
    assert_confounded(changed);

    let mut changed = fixed_identity();
    changed.completion_contract_fingerprint = "different-oracle@v2";
    assert_confounded(changed);
}

#[test]
fn decisive_evidence_presence_must_actually_flip_across_the_pair() {
    let both_absent = (
        run(fixed_identity(), false, RunOutcome::Failed),
        run(fixed_identity(), false, RunOutcome::Success),
    );
    let both_present = (
        run(fixed_identity(), true, RunOutcome::Failed),
        run(fixed_identity(), true, RunOutcome::Success),
    );

    assert_eq!(
        evaluate_pair(&both_absent.0, &both_absent.1),
        RetirementVerdict::InvalidEvidencePair
    );
    assert_eq!(
        evaluate_pair(&both_present.0, &both_present.1),
        RetirementVerdict::InvalidEvidencePair
    );
}

#[path = "../examples/support/capability_demand_retirement_impl.rs"]
mod capability_demand_retirement_impl;

use capability_demand_retirement_impl::{
    ConditionId, ReplayIdentity, RetirementVerdict, RunOutcome, RunReceipt, TrialInputManifest,
    evaluate_pair,
};

fn manifest() -> TrialInputManifest {
    serde_json::from_slice(include_bytes!(
        "../research/capability-demand-retirement/stensibly-convex-index-review-v1.manifest.json"
    ))
    .expect("valid frozen trial manifest")
}

fn identity(manifest: &TrialInputManifest) -> ReplayIdentity {
    ReplayIdentity {
        repository: manifest.repository.clone(),
        repository_revision: manifest.revision.clone(),
        target_path: manifest.target_path.clone(),
        target_blob_sha: manifest.target_blob_sha.clone(),
        task_sha256: manifest.worker_visible_common.task.sha256.clone(),
        patch_sha256: manifest.worker_visible_common.patch.sha256.clone(),
        worker_identity: "worker-a@v1".to_string(),
        harness_identity: "review-harness@v1".to_string(),
        affordance_identity: "read-only-repo-tools@v1".to_string(),
        sampling_config_fingerprint: "sampling@v1".to_string(),
        completion_contract_sha256: manifest.evaluator_only.oracle.sha256.clone(),
    }
}

fn receipt(
    manifest: &TrialInputManifest,
    condition_id: ConditionId,
    condition_order: u32,
    session_id: &str,
    outcome: RunOutcome,
) -> RunReceipt {
    let condition = match condition_id {
        ConditionId::FileLocalJei => &manifest.conditions.file_local_jei,
        ConditionId::ScopedJei => &manifest.conditions.scoped_jei,
    };
    RunReceipt {
        schema_version: 1,
        trial_id: manifest.trial_id.clone(),
        pair_id: "pair-001".to_string(),
        replicate_id: "replicate-001".to_string(),
        condition_id,
        condition_order,
        session_id: session_id.to_string(),
        fresh_session: true,
        prior_condition_exposure: false,
        packet_sha256: condition.packet.sha256.clone(),
        decisive_evidence_present: condition.decisive_evidence_present,
        identity: identity(manifest),
        outcome,
    }
}

#[test]
fn failed_baseline_and_successful_treatment_emit_only_a_paired_signal() {
    let manifest = manifest();
    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-baseline",
        RunOutcome::Failed,
    );
    let treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "session-treatment",
        RunOutcome::Success,
    );

    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::PairedRetirementSignal
    );
}

#[test]
fn condition_execution_order_can_reverse_without_changing_semantic_roles() {
    let manifest = manifest();
    let treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        0,
        "session-treatment",
        RunOutcome::Success,
    );
    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        1,
        "session-baseline",
        RunOutcome::Failed,
    );

    assert_eq!(
        evaluate_pair(&manifest, &treatment, &baseline).verdict,
        RetirementVerdict::PairedRetirementSignal
    );
}

#[test]
fn baseline_success_means_no_success_demand_was_observed() {
    let manifest = manifest();
    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-baseline",
        RunOutcome::Success,
    );
    let treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "session-treatment",
        RunOutcome::Success,
    );

    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::NoDemandObserved
    );
}

#[test]
fn correct_escalation_then_treatment_success_is_separate_from_worker_failure() {
    let manifest = manifest();
    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-baseline",
        RunOutcome::CorrectEscalation,
    );
    let treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "session-treatment",
        RunOutcome::Success,
    );

    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::CorrectEscalationThenSuccess
    );
}

#[test]
fn treatment_failure_preserves_residual_demand() {
    let manifest = manifest();
    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-baseline",
        RunOutcome::Failed,
    );
    let treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "session-treatment",
        RunOutcome::Failed,
    );

    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::DemandPersists
    );
}

#[test]
fn wrong_packet_hash_is_confounded_even_when_claimed_evidence_flag_looks_right() {
    let manifest = manifest();
    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-baseline",
        RunOutcome::Failed,
    );
    let mut treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "session-treatment",
        RunOutcome::Success,
    );
    treatment.packet_sha256 = "different-packet".to_string();

    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::Confounded
    );
}

#[test]
fn same_session_or_prior_condition_exposure_is_confounded() {
    let manifest = manifest();
    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "same-session",
        RunOutcome::Failed,
    );
    let treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "same-session",
        RunOutcome::Success,
    );
    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::Confounded
    );

    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-baseline",
        RunOutcome::Failed,
    );
    let mut treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "session-treatment",
        RunOutcome::Success,
    );
    treatment.prior_condition_exposure = true;
    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::Confounded
    );
}

#[test]
fn changed_worker_or_sampling_config_is_confounded() {
    let manifest = manifest();
    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-baseline",
        RunOutcome::Failed,
    );
    let mut treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "session-treatment",
        RunOutcome::Success,
    );
    treatment.identity.worker_identity = "worker-b@v1".to_string();
    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::Confounded
    );

    let baseline = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-baseline",
        RunOutcome::Failed,
    );
    let mut treatment = receipt(
        &manifest,
        ConditionId::ScopedJei,
        1,
        "session-treatment",
        RunOutcome::Success,
    );
    treatment.identity.sampling_config_fingerprint = "sampling@v2".to_string();
    assert_eq!(
        evaluate_pair(&manifest, &baseline, &treatment).verdict,
        RetirementVerdict::Confounded
    );
}

#[test]
fn duplicate_conditions_are_an_invalid_evidence_pair() {
    let manifest = manifest();
    let first = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-a",
        RunOutcome::Failed,
    );
    let second = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        1,
        "session-b",
        RunOutcome::Success,
    );

    assert_eq!(
        evaluate_pair(&manifest, &first, &second).verdict,
        RetirementVerdict::InvalidEvidencePair
    );
}

#[test]
fn run_receipt_json_rejects_unknown_fields() {
    let manifest = manifest();
    let receipt = receipt(
        &manifest,
        ConditionId::FileLocalJei,
        0,
        "session-a",
        RunOutcome::Failed,
    );
    let mut value = serde_json::to_value(&serde_json::json!({
        "schema_version": receipt.schema_version,
        "trial_id": receipt.trial_id,
        "pair_id": receipt.pair_id,
        "replicate_id": receipt.replicate_id,
        "condition_id": "file_local_jei",
        "condition_order": receipt.condition_order,
        "session_id": receipt.session_id,
        "fresh_session": receipt.fresh_session,
        "prior_condition_exposure": receipt.prior_condition_exposure,
        "packet_sha256": receipt.packet_sha256,
        "decisive_evidence_present": receipt.decisive_evidence_present,
        "identity": {
            "repository": receipt.identity.repository,
            "repository_revision": receipt.identity.repository_revision,
            "target_path": receipt.identity.target_path,
            "target_blob_sha": receipt.identity.target_blob_sha,
            "task_sha256": receipt.identity.task_sha256,
            "patch_sha256": receipt.identity.patch_sha256,
            "worker_identity": receipt.identity.worker_identity,
            "harness_identity": receipt.identity.harness_identity,
            "affordance_identity": receipt.identity.affordance_identity,
            "sampling_config_fingerprint": receipt.identity.sampling_config_fingerprint,
            "completion_contract_sha256": receipt.identity.completion_contract_sha256
        },
        "outcome": "failed"
    }))
    .expect("receipt JSON value");
    value
        .as_object_mut()
        .expect("object")
        .insert("invented_confidence".to_string(), serde_json::json!(0.99));

    assert!(serde_json::from_value::<RunReceipt>(value).is_err());
}

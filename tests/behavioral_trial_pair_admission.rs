#![allow(dead_code)]

#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[path = "../src/behavioral_trial_pair_admission.rs"]
mod behavioral_trial_pair_admission;
#[path = "../src/behavioral_trial_run.rs"]
mod behavioral_trial_run;

use behavioral_trial::{
    BEHAVIORAL_TRIAL_SCHEMA_VERSION, BehavioralTrialArmKind, BehavioralTrialObservation,
    BehavioralTrialPlan, fingerprint_plan, materialize_worker_packet, parse_behavioral_trial_plan,
};
use behavioral_trial_pair_admission::{
    BehavioralTrialPairAdmissionReason, BehavioralTrialPairAdmissionVerdict,
    evaluate_behavioral_trial_pair_admission,
};
use behavioral_trial_run::{
    BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION, BehavioralTrialExecutionOrigin,
    BehavioralTrialRunMetadata, BehavioralTrialRunPair, BehavioralTrialRunReceipt,
    build_behavioral_trial_run_receipt,
};

const PLAN: &[u8] =
    include_bytes!("../research/behavioral-trials/stensibly-index-guard-detail.json");
const SAMPLING_SHA: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_SAMPLING_SHA: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn plan() -> BehavioralTrialPlan {
    parse_behavioral_trial_plan(PLAN).expect("retained neutral Stensibly plan should parse")
}

fn metadata(
    sequence_index: u8,
    session_id: &str,
    freshness_receipt: &str,
) -> BehavioralTrialRunMetadata {
    BehavioralTrialRunMetadata {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        execution_origin: BehavioralTrialExecutionOrigin::ExternalHarness,
        sequence_index,
        worker_identity: "fixed-worker@v1".into(),
        harness_identity: "first-action-harness@v1".into(),
        affordance_identity: "packet-only-choice@v1".into(),
        sampling_config_sha256: SAMPLING_SHA.into(),
        session_id: session_id.into(),
        freshness_receipt: freshness_receipt.into(),
        fresh_session: true,
        prior_condition_exposure: false,
    }
}

fn packet_bytes(plan: &BehavioralTrialPlan, arm: BehavioralTrialArmKind) -> Vec<u8> {
    let packet = materialize_worker_packet(plan, arm).unwrap();
    let mut bytes = serde_json::to_vec_pretty(&packet).unwrap();
    bytes.push(b'\n');
    bytes
}

fn output_bytes(
    plan: &BehavioralTrialPlan,
    arm: BehavioralTrialArmKind,
    worker_ref: &str,
    first_action_id: &str,
) -> Vec<u8> {
    let packet = materialize_worker_packet(plan, arm).unwrap();
    let observation = BehavioralTrialObservation {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        trial_id: plan.trial_id.clone(),
        plan_fingerprint: fingerprint_plan(plan).unwrap(),
        worker_packet_fingerprint: packet.worker_packet_fingerprint,
        worker_ref: worker_ref.into(),
        first_action_id: first_action_id.into(),
    };
    let mut bytes = serde_json::to_vec_pretty(&observation).unwrap();
    bytes.push(b'\n');
    bytes
}

fn receipt(
    plan: &BehavioralTrialPlan,
    arm: BehavioralTrialArmKind,
    metadata: BehavioralTrialRunMetadata,
    worker_ref: &str,
    first_action_id: &str,
) -> BehavioralTrialRunReceipt {
    build_behavioral_trial_run_receipt(
        plan,
        metadata,
        &packet_bytes(plan, arm),
        &output_bytes(plan, arm, worker_ref, first_action_id),
    )
    .unwrap()
}

fn pair(
    plan: &BehavioralTrialPlan,
    first: BehavioralTrialRunReceipt,
    second: BehavioralTrialRunReceipt,
) -> BehavioralTrialRunPair {
    BehavioralTrialRunPair {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        plan: Box::new(plan.clone()),
        runs: vec![first, second],
    }
}

#[test]
fn admitted_ab_pair_reuses_strict_descriptive_evaluator() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:control", "freshness:control"),
        "worker:control",
        "inspect_accepted_guard_detail",
    );
    let treatment = receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        metadata(2, "session:treatment", "freshness:treatment"),
        "worker:treatment",
        "block_patch",
    );

    let result =
        evaluate_behavioral_trial_pair_admission(&pair(&plan, control, treatment)).unwrap();
    assert_eq!(
        result.verdict,
        BehavioralTrialPairAdmissionVerdict::Admitted
    );
    assert!(result.reasons.is_empty());
    assert!(result.frozen_identity_match);
    assert!(result.fresh_uncontaminated_sessions);
    assert!(result.distinct_arm_coverage);
    assert!(!result.automatic_effect_claim);
    assert!(!result.automatic_generalization);

    let behavioral = result.behavioral_evaluation.unwrap();
    assert_eq!(
        behavioral.trial.control.first_action_id,
        "inspect_accepted_guard_detail"
    );
    assert_eq!(behavioral.trial.treatment.first_action_id, "block_patch");
    assert!(!behavioral.trial.same_first_action);
}

#[test]
fn admitted_ba_vector_order_preserves_recorded_execution_sequence_and_arm_mapping() {
    let plan = plan();
    let treatment = receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        metadata(1, "session:treatment", "freshness:treatment"),
        "worker:treatment",
        "block_patch",
    );
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(2, "session:control", "freshness:control"),
        "worker:control",
        "inspect_accepted_guard_detail",
    );

    let result =
        evaluate_behavioral_trial_pair_admission(&pair(&plan, treatment, control)).unwrap();
    assert_eq!(
        result.verdict,
        BehavioralTrialPairAdmissionVerdict::Admitted
    );
    assert!(result.reasons.is_empty());
    assert_eq!(result.sequence_indexes, vec![1, 2]);
    let behavioral = result.behavioral_evaluation.unwrap();
    assert_eq!(behavioral.treatment_sequence_index, 1);
    assert_eq!(behavioral.control_sequence_index, 2);
    assert_eq!(behavioral.trial.treatment.first_action_id, "block_patch");
    assert_eq!(
        behavioral.trial.control.first_action_id,
        "inspect_accepted_guard_detail"
    );
}

#[test]
fn admitted_pair_can_preserve_same_first_action_without_effect_claim() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:control", "freshness:control"),
        "worker:control",
        "inspect_more_repository_context",
    );
    let treatment = receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        metadata(2, "session:treatment", "freshness:treatment"),
        "worker:treatment",
        "inspect_more_repository_context",
    );

    let result =
        evaluate_behavioral_trial_pair_admission(&pair(&plan, control, treatment)).unwrap();
    assert_eq!(
        result.verdict,
        BehavioralTrialPairAdmissionVerdict::Admitted
    );
    assert!(result.reasons.is_empty());
    assert!(
        result
            .behavioral_evaluation
            .unwrap()
            .trial
            .same_first_action
    );
    assert!(!result.automatic_effect_claim);
    assert!(!result.automatic_generalization);
}

#[test]
fn frozen_execution_axis_drift_is_confounded_with_exact_reason() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:control", "freshness:control"),
        "worker:control",
        "inspect_accepted_guard_detail",
    );

    let mut variants = Vec::new();
    let mut worker = metadata(2, "session:treatment-a", "freshness:treatment-a");
    worker.worker_identity = "other-worker@v1".into();
    variants.push((
        worker,
        BehavioralTrialPairAdmissionReason::WorkerIdentityDrift,
    ));
    let mut harness = metadata(2, "session:treatment-b", "freshness:treatment-b");
    harness.harness_identity = "other-harness@v1".into();
    variants.push((
        harness,
        BehavioralTrialPairAdmissionReason::HarnessIdentityDrift,
    ));
    let mut affordance = metadata(2, "session:treatment-c", "freshness:treatment-c");
    affordance.affordance_identity = "other-affordance@v1".into();
    variants.push((
        affordance,
        BehavioralTrialPairAdmissionReason::AffordanceIdentityDrift,
    ));
    let mut sampling = metadata(2, "session:treatment-d", "freshness:treatment-d");
    sampling.sampling_config_sha256 = OTHER_SAMPLING_SHA.into();
    variants.push((
        sampling,
        BehavioralTrialPairAdmissionReason::SamplingConfigDrift,
    ));

    for (metadata, expected_reason) in variants {
        let treatment = receipt(
            &plan,
            BehavioralTrialArmKind::Treatment,
            metadata,
            "worker:treatment",
            "block_patch",
        );
        let result =
            evaluate_behavioral_trial_pair_admission(&pair(&plan, control.clone(), treatment))
                .unwrap();
        assert_eq!(
            result.verdict,
            BehavioralTrialPairAdmissionVerdict::Confounded
        );
        assert_eq!(result.reasons, vec![expected_reason]);
        assert!(!result.frozen_identity_match);
        assert!(result.behavioral_evaluation.is_none());
    }
}

#[test]
fn authentic_session_confounds_preserve_exact_reasons() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:control", "freshness:control"),
        "worker:control",
        "inspect_accepted_guard_detail",
    );

    let mut nonfresh = metadata(2, "session:nonfresh", "freshness:nonfresh");
    nonfresh.fresh_session = false;
    let mut preexposed = metadata(2, "session:preexposed", "freshness:preexposed");
    preexposed.prior_condition_exposure = true;
    let reused_session = metadata(2, "session:control", "freshness:other");
    let reused_freshness = metadata(2, "session:other", "freshness:control");
    let duplicate_sequence = metadata(1, "session:other-two", "freshness:other-two");

    for (metadata, expected_reason) in [
        (
            nonfresh,
            BehavioralTrialPairAdmissionReason::NonFreshSession,
        ),
        (
            preexposed,
            BehavioralTrialPairAdmissionReason::PriorConditionExposure,
        ),
        (
            reused_session,
            BehavioralTrialPairAdmissionReason::ReusedSessionId,
        ),
        (
            reused_freshness,
            BehavioralTrialPairAdmissionReason::ReusedFreshnessReceipt,
        ),
        (
            duplicate_sequence,
            BehavioralTrialPairAdmissionReason::InvalidSequenceCoverage,
        ),
    ] {
        let treatment = receipt(
            &plan,
            BehavioralTrialArmKind::Treatment,
            metadata,
            "worker:treatment",
            "block_patch",
        );
        let result =
            evaluate_behavioral_trial_pair_admission(&pair(&plan, control.clone(), treatment))
                .unwrap();
        assert_eq!(
            result.verdict,
            BehavioralTrialPairAdmissionVerdict::Confounded
        );
        assert_eq!(result.reasons, vec![expected_reason]);
        assert!(!result.fresh_uncontaminated_sessions);
        assert!(result.behavioral_evaluation.is_none());
    }

    let treatment = receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        metadata(2, "session:unique", "freshness:unique"),
        "worker:control",
        "block_patch",
    );
    let result =
        evaluate_behavioral_trial_pair_admission(&pair(&plan, control, treatment)).unwrap();
    assert_eq!(
        result.verdict,
        BehavioralTrialPairAdmissionVerdict::Confounded
    );
    assert_eq!(
        result.reasons,
        vec![BehavioralTrialPairAdmissionReason::ReusedWorkerRef]
    );
    assert!(!result.fresh_uncontaminated_sessions);
}

#[test]
fn same_authentic_arm_twice_is_invalid_pair_even_if_execution_axes_match() {
    let plan = plan();
    let first = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:first", "freshness:first"),
        "worker:first",
        "inspect_accepted_guard_detail",
    );
    let second = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(2, "session:second", "freshness:second"),
        "worker:second",
        "approve_patch",
    );

    let result = evaluate_behavioral_trial_pair_admission(&pair(&plan, first, second)).unwrap();
    assert_eq!(
        result.verdict,
        BehavioralTrialPairAdmissionVerdict::InvalidPair
    );
    assert_eq!(
        result.reasons,
        vec![BehavioralTrialPairAdmissionReason::SameArm]
    );
    assert!(!result.distinct_arm_coverage);
    assert!(result.behavioral_evaluation.is_none());
}

#[test]
fn simultaneous_confounds_are_preserved_together() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:shared", "freshness:shared"),
        "worker:shared",
        "inspect_accepted_guard_detail",
    );
    let mut treatment_metadata = metadata(1, "session:shared", "freshness:shared");
    treatment_metadata.worker_identity = "other-worker@v1".into();
    treatment_metadata.fresh_session = false;
    let treatment = receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        treatment_metadata,
        "worker:shared",
        "block_patch",
    );

    let result =
        evaluate_behavioral_trial_pair_admission(&pair(&plan, control, treatment)).unwrap();
    assert_eq!(
        result.verdict,
        BehavioralTrialPairAdmissionVerdict::Confounded
    );
    assert_eq!(
        result.reasons,
        vec![
            BehavioralTrialPairAdmissionReason::WorkerIdentityDrift,
            BehavioralTrialPairAdmissionReason::NonFreshSession,
            BehavioralTrialPairAdmissionReason::ReusedSessionId,
            BehavioralTrialPairAdmissionReason::ReusedFreshnessReceipt,
            BehavioralTrialPairAdmissionReason::ReusedWorkerRef,
            BehavioralTrialPairAdmissionReason::InvalidSequenceCoverage,
        ]
    );
    assert!(!result.frozen_identity_match);
    assert!(!result.fresh_uncontaminated_sessions);
    assert!(result.behavioral_evaluation.is_none());
}

#[test]
fn tampered_individual_receipt_rejects_before_comparability_verdict() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:control", "freshness:control"),
        "worker:control",
        "inspect_accepted_guard_detail",
    );
    let mut treatment = receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        metadata(2, "session:treatment", "freshness:treatment"),
        "worker:treatment",
        "block_patch",
    );
    treatment.raw_output_sha256 =
        "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into();

    let error =
        evaluate_behavioral_trial_pair_admission(&pair(&plan, control, treatment)).unwrap_err();
    assert!(error.contains("fields do not match the byte-authentic rebuild"));
}

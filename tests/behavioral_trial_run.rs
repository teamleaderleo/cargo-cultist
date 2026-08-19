#![allow(dead_code)]

#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[path = "../src/behavioral_trial_run.rs"]
mod behavioral_trial_run;

use behavioral_trial::{
    BEHAVIORAL_TRIAL_SCHEMA_VERSION, BehavioralTrialArmKind, BehavioralTrialObservation,
    BehavioralTrialPlan, fingerprint_plan, materialize_worker_packet, parse_behavioral_trial_plan,
};
use behavioral_trial_run::{
    BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION, BehavioralTrialExecutionOrigin,
    BehavioralTrialRunMetadata, BehavioralTrialRunPair, build_behavioral_trial_run_receipt,
    evaluate_behavioral_trial_run_pair,
};

const PLAN: &[u8] =
    include_bytes!("../research/behavioral-trials/stensibly-index-guard-detail.json");
const SAMPLING_SHA256: &str =
    "sha256:3c0474dcc347d480cbfa4a9590e8a4361cbaf0ebd75504444a6340bbc0f6109c";

fn plan() -> BehavioralTrialPlan {
    parse_behavioral_trial_plan(PLAN).unwrap()
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
        worker_identity: "worker:gpt-config-v1".to_string(),
        harness_identity: "harness:fresh-session-v1".to_string(),
        affordance_identity: "affordances:first-action-only-v1".to_string(),
        sampling_config_sha256: SAMPLING_SHA256.to_string(),
        session_id: session_id.to_string(),
        freshness_receipt: freshness_receipt.to_string(),
        fresh_session: true,
        prior_condition_exposure: false,
    }
}

fn raw_observation(
    plan: &BehavioralTrialPlan,
    arm: BehavioralTrialArmKind,
    worker_ref: &str,
    first_action_id: &str,
) -> Vec<u8> {
    let packet = materialize_worker_packet(plan, arm).unwrap();
    serde_json::to_vec(&BehavioralTrialObservation {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        trial_id: plan.trial_id.clone(),
        plan_fingerprint: fingerprint_plan(plan).unwrap(),
        worker_packet_fingerprint: packet.worker_packet_fingerprint,
        worker_ref: worker_ref.to_string(),
        first_action_id: first_action_id.to_string(),
    })
    .unwrap()
}

fn run_receipt(
    plan: &BehavioralTrialPlan,
    arm: BehavioralTrialArmKind,
    metadata: BehavioralTrialRunMetadata,
    worker_ref: &str,
    first_action_id: &str,
) -> behavioral_trial_run::BehavioralTrialRunReceipt {
    let packet = materialize_worker_packet(plan, arm).unwrap();
    let raw_packet = serde_json::to_vec(&packet).unwrap();
    let raw_output = raw_observation(plan, arm, worker_ref, first_action_id);
    build_behavioral_trial_run_receipt(plan, metadata, &raw_packet, &raw_output).unwrap()
}

#[test]
fn real_guard_detail_plan_admits_fresh_ba_pair_and_preserves_exact_run_metadata() {
    let plan = plan();
    let action_ids = plan
        .allowed_first_actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<Vec<_>>();
    assert!(action_ids.contains(&"block_patch"));
    assert!(!action_ids.contains(&"block_and_shorten_identifier"));
    let treatment = run_receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        metadata(
            1,
            "session:treatment-fresh",
            "provider:session/treatment-fresh",
        ),
        "worker-run:treatment",
        "block_patch",
    );
    let control = run_receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(2, "session:control-fresh", "provider:session/control-fresh"),
        "worker-run:control",
        "inspect_accepted_guard_detail",
    );

    assert!(treatment.worker_packet_sha256.starts_with("sha256:"));
    assert!(treatment.raw_output_sha256.starts_with("sha256:"));
    assert_eq!(
        serde_json::from_str::<BehavioralTrialObservation>(&treatment.raw_output).unwrap(),
        treatment.observation
    );

    let evaluation = evaluate_behavioral_trial_run_pair(&BehavioralTrialRunPair {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        plan: Box::new(plan),
        runs: vec![treatment, control],
    })
    .unwrap();

    assert_eq!(evaluation.control_sequence_index, 2);
    assert_eq!(evaluation.treatment_sequence_index, 1);
    assert_eq!(evaluation.control_session_id, "session:control-fresh");
    assert_eq!(evaluation.treatment_session_id, "session:treatment-fresh");
    assert_eq!(evaluation.worker_identity, "worker:gpt-config-v1");
    assert_eq!(evaluation.harness_identity, "harness:fresh-session-v1");
    assert_eq!(
        evaluation.trial.control.first_action_id,
        "inspect_accepted_guard_detail"
    );
    assert_eq!(evaluation.trial.treatment.first_action_id, "block_patch");
    assert!(!evaluation.trial.same_first_action);
}

#[test]
fn receipt_builder_rejects_nonfresh_or_preexposed_run_metadata() {
    let plan = plan();
    let packet = materialize_worker_packet(&plan, BehavioralTrialArmKind::Control).unwrap();
    let raw_packet = serde_json::to_vec(&packet).unwrap();
    let raw_output = raw_observation(
        &plan,
        BehavioralTrialArmKind::Control,
        "worker-run:control",
        "inspect_accepted_guard_detail",
    );

    let mut nonfresh = metadata(1, "session:one", "provider:session/one");
    nonfresh.fresh_session = false;
    let error =
        build_behavioral_trial_run_receipt(&plan, nonfresh, &raw_packet, &raw_output).unwrap_err();
    assert!(error.to_string().contains("fresh_session=true"));

    let mut exposed = metadata(1, "session:two", "provider:session/two");
    exposed.prior_condition_exposure = true;
    let error =
        build_behavioral_trial_run_receipt(&plan, exposed, &raw_packet, &raw_output).unwrap_err();
    assert!(error.to_string().contains("prior_condition_exposure=false"));
}

#[test]
fn raw_worker_output_must_bind_to_exact_packet_and_registered_action() {
    let plan = plan();
    let packet = materialize_worker_packet(&plan, BehavioralTrialArmKind::Control).unwrap();
    let raw_packet = serde_json::to_vec(&packet).unwrap();

    let wrong_arm_output = raw_observation(
        &plan,
        BehavioralTrialArmKind::Treatment,
        "worker-run:wrong-arm",
        "block_patch",
    );
    let error = build_behavioral_trial_run_receipt(
        &plan,
        metadata(1, "session:wrong-arm", "provider:session/wrong-arm"),
        &raw_packet,
        &wrong_arm_output,
    )
    .unwrap_err();
    assert!(error.to_string().contains("exact worker packet"));

    let invalid_action = raw_observation(
        &plan,
        BehavioralTrialArmKind::Control,
        "worker-run:invalid-action",
        "not_registered",
    );
    let error = build_behavioral_trial_run_receipt(
        &plan,
        metadata(
            1,
            "session:invalid-action",
            "provider:session/invalid-action",
        ),
        &raw_packet,
        &invalid_action,
    )
    .unwrap_err();
    assert!(error.to_string().contains("outside the frozen vocabulary"));
}

#[test]
fn pair_rejects_same_session_and_configuration_drift() {
    let plan = plan();
    let control = run_receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:same", "provider:session/control"),
        "worker-run:control",
        "inspect_accepted_guard_detail",
    );
    let treatment = run_receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        metadata(2, "session:same", "provider:session/treatment"),
        "worker-run:treatment",
        "block_patch",
    );
    let error = evaluate_behavioral_trial_run_pair(&BehavioralTrialRunPair {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        plan: Box::new(plan.clone()),
        runs: vec![control.clone(), treatment.clone()],
    })
    .unwrap_err();
    assert!(error.to_string().contains("distinct fresh session ids"));

    let mut drifted = treatment;
    drifted.metadata.harness_identity = "harness:other".to_string();
    drifted.metadata.session_id = "session:treatment".to_string();
    let error = evaluate_behavioral_trial_run_pair(&BehavioralTrialRunPair {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        plan: Box::new(plan),
        runs: vec![control, drifted],
    })
    .unwrap_err();
    assert!(error.to_string().contains("identical harness_identity"));
}

#[test]
fn retained_raw_bytes_are_revalidated_during_pair_admission() {
    let plan = plan();
    let control = run_receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        metadata(1, "session:control", "provider:session/control"),
        "worker-run:control",
        "inspect_accepted_guard_detail",
    );
    let mut treatment = run_receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        metadata(2, "session:treatment", "provider:session/treatment"),
        "worker-run:treatment",
        "block_patch",
    );
    treatment.raw_output.push(' ');

    let error = evaluate_behavioral_trial_run_pair(&BehavioralTrialRunPair {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        plan: Box::new(plan),
        runs: vec![control, treatment],
    })
    .unwrap_err();
    assert!(error.to_string().contains("raw_output_sha256"));
}

#[allow(dead_code)]
#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[allow(dead_code)]
#[path = "../src/behavioral_trial_run.rs"]
mod behavioral_trial_run;
#[allow(dead_code)]
#[path = "../src/behavioral_trial_pair_classification.rs"]
mod behavioral_trial_pair_classification;

use behavioral_trial::{
    BEHAVIORAL_TRIAL_SCHEMA_VERSION, BehavioralTrialArmKind, BehavioralTrialObservation,
    BehavioralTrialPlan, fingerprint_plan, materialize_worker_packet, parse_behavioral_trial_plan,
};
use behavioral_trial_pair_classification::{
    BehavioralTrialPairReason, BehavioralTrialPairVerdict, classify_behavioral_trial_run_pair,
};
use behavioral_trial_run::{
    BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION, BehavioralTrialExecutionOrigin, BehavioralTrialRunMetadata,
    BehavioralTrialRunPair, BehavioralTrialRunReceipt, build_behavioral_trial_run_receipt,
    canonical_materialized_worker_packet_bytes,
};

const PLAN: &[u8] =
    include_bytes!("../research/behavioral-trials/stensibly-index-guard-detail.json");
const SAMPLING: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_SAMPLING: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn plan() -> BehavioralTrialPlan {
    parse_behavioral_trial_plan(PLAN).expect("retained neutral Stensibly plan should parse")
}

fn metadata(index: u8, session: &str, freshness: &str) -> BehavioralTrialRunMetadata {
    BehavioralTrialRunMetadata {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        execution_origin: BehavioralTrialExecutionOrigin::ExternalHarness,
        sequence_index: index,
        worker_identity: "fixed-worker@v1".into(),
        harness_identity: "first-action-harness@v1".into(),
        affordance_identity: "packet-only-choice@v1".into(),
        sampling_config_sha256: SAMPLING.into(),
        session_id: session.into(),
        freshness_receipt: freshness.into(),
        fresh_session: true,
        prior_condition_exposure: false,
    }
}

fn run(
    plan: &BehavioralTrialPlan,
    arm: BehavioralTrialArmKind,
    index: u8,
    session: &str,
    freshness: &str,
    worker_ref: &str,
    first_action: &str,
) -> BehavioralTrialRunReceipt {
    let packet = materialize_worker_packet(plan, arm).unwrap();
    let raw_packet = canonical_materialized_worker_packet_bytes(plan, arm).unwrap();
    let observation = BehavioralTrialObservation {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        trial_id: plan.trial_id.clone(),
        plan_fingerprint: fingerprint_plan(plan).unwrap(),
        worker_packet_fingerprint: packet.worker_packet_fingerprint,
        worker_ref: worker_ref.into(),
        first_action_id: first_action.into(),
    };
    let mut raw_output = serde_json::to_vec_pretty(&observation).unwrap();
    raw_output.push(b'\n');
    build_behavioral_trial_run_receipt(
        plan,
        metadata(index, session, freshness),
        &raw_packet,
        &raw_output,
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
fn admitted_ab_pair_reuses_existing_descriptive_evaluation() {
    let plan = plan();
    let control = run(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "session-control",
        "fresh-control",
        "worker:control",
        "inspect_accepted_guard_detail",
    );
    let treatment = run(
        &plan,
        BehavioralTrialArmKind::Treatment,
        2,
        "session-treatment",
        "fresh-treatment",
        "worker:treatment",
        "block_patch",
    );

    let result = classify_behavioral_trial_run_pair(&pair(&plan, control, treatment)).unwrap();
    assert_eq!(result.verdict, BehavioralTrialPairVerdict::Admitted);
    assert!(result.reasons.is_empty());
    assert!(!result.automatic_effect_claim);
    assert!(!result.automatic_generalization);
    let evaluation = result.evaluation.expect("admitted pair should reconcile");
    assert_eq!(
        evaluation.trial.control.first_action_id,
        "inspect_accepted_guard_detail"
    );
    assert_eq!(evaluation.trial.treatment.first_action_id, "block_patch");
    assert!(!evaluation.trial.same_first_action);
}

#[test]
fn admitted_ba_order_is_classified_by_packet_identity_not_vector_order() {
    let plan = plan();
    let treatment = run(
        &plan,
        BehavioralTrialArmKind::Treatment,
        1,
        "session-treatment",
        "fresh-treatment",
        "worker:treatment",
        "block_patch",
    );
    let control = run(
        &plan,
        BehavioralTrialArmKind::Control,
        2,
        "session-control",
        "fresh-control",
        "worker:control",
        "inspect_accepted_guard_detail",
    );

    let result = classify_behavioral_trial_run_pair(&pair(&plan, treatment, control)).unwrap();
    assert_eq!(result.verdict, BehavioralTrialPairVerdict::Admitted);
    let evaluation = result.evaluation.unwrap();
    assert_eq!(
        evaluation.trial.control.first_action_id,
        "inspect_accepted_guard_detail"
    );
    assert_eq!(evaluation.trial.treatment.first_action_id, "block_patch");
}

#[test]
fn same_arm_twice_is_invalid_pair_without_behavioral_evaluation() {
    let plan = plan();
    let first = run(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "session-one",
        "fresh-one",
        "worker:one",
        "inspect_accepted_guard_detail",
    );
    let second = run(
        &plan,
        BehavioralTrialArmKind::Control,
        2,
        "session-two",
        "fresh-two",
        "worker:two",
        "approve_patch",
    );

    let result = classify_behavioral_trial_run_pair(&pair(&plan, first, second)).unwrap();
    assert_eq!(result.verdict, BehavioralTrialPairVerdict::InvalidPair);
    assert_eq!(result.reasons, vec![BehavioralTrialPairReason::SameArm]);
    assert!(result.evaluation.is_none());
}

#[test]
fn frozen_axis_drift_is_typed_confounded() {
    let plan = plan();
    let control = run(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "session-control",
        "fresh-control",
        "worker:control",
        "inspect_accepted_guard_detail",
    );

    let cases: [(BehavioralTrialPairReason, fn(&mut BehavioralTrialRunReceipt)); 4] = [
        (BehavioralTrialPairReason::WorkerIdentityDrift, |run| {
            run.metadata.worker_identity = "other-worker@v1".into();
        }),
        (BehavioralTrialPairReason::HarnessIdentityDrift, |run| {
            run.metadata.harness_identity = "other-harness@v1".into();
        }),
        (BehavioralTrialPairReason::AffordanceIdentityDrift, |run| {
            run.metadata.affordance_identity = "other-affordance@v1".into();
        }),
        (BehavioralTrialPairReason::SamplingConfigDrift, |run| {
            run.metadata.sampling_config_sha256 = OTHER_SAMPLING.into();
        }),
    ];

    for (expected, mutate) in cases {
        let mut treatment = run(
            &plan,
            BehavioralTrialArmKind::Treatment,
            2,
            "session-treatment",
            "fresh-treatment",
            "worker:treatment",
            "block_patch",
        );
        mutate(&mut treatment);
        let result = classify_behavioral_trial_run_pair(&pair(&plan, control.clone(), treatment))
            .unwrap();
        assert_eq!(result.verdict, BehavioralTrialPairVerdict::Confounded);
        assert_eq!(result.reasons, vec![expected]);
        assert!(result.evaluation.is_none());
    }
}

#[test]
fn session_freshness_worker_ref_and_sequence_reuse_are_typed_confounded() {
    let plan = plan();
    let control = run(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "session-control",
        "fresh-control",
        "worker:shared",
        "inspect_accepted_guard_detail",
    );

    let mut treatment = run(
        &plan,
        BehavioralTrialArmKind::Treatment,
        2,
        "session-control",
        "fresh-treatment",
        "worker:treatment",
        "block_patch",
    );
    let result = classify_behavioral_trial_run_pair(&pair(&plan, control.clone(), treatment))
        .unwrap();
    assert_eq!(result.verdict, BehavioralTrialPairVerdict::Confounded);
    assert_eq!(result.reasons, vec![BehavioralTrialPairReason::ReusedSessionId]);

    treatment = run(
        &plan,
        BehavioralTrialArmKind::Treatment,
        2,
        "session-treatment",
        "fresh-control",
        "worker:treatment",
        "block_patch",
    );
    let result = classify_behavioral_trial_run_pair(&pair(&plan, control.clone(), treatment))
        .unwrap();
    assert_eq!(
        result.reasons,
        vec![BehavioralTrialPairReason::ReusedFreshnessReceipt]
    );

    treatment = run(
        &plan,
        BehavioralTrialArmKind::Treatment,
        2,
        "session-treatment",
        "fresh-treatment",
        "worker:shared",
        "block_patch",
    );
    let result = classify_behavioral_trial_run_pair(&pair(&plan, control.clone(), treatment))
        .unwrap();
    assert_eq!(result.reasons, vec![BehavioralTrialPairReason::ReusedWorkerRef]);

    treatment = run(
        &plan,
        BehavioralTrialArmKind::Treatment,
        1,
        "session-treatment",
        "fresh-treatment",
        "worker:treatment",
        "block_patch",
    );
    let result = classify_behavioral_trial_run_pair(&pair(&plan, control, treatment)).unwrap();
    assert_eq!(
        result.reasons,
        vec![BehavioralTrialPairReason::InvalidSequenceCoverage]
    );
}

#[test]
fn multiple_pair_confounds_are_preserved_together() {
    let plan = plan();
    let control = run(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "session-shared",
        "fresh-shared",
        "worker:shared",
        "inspect_accepted_guard_detail",
    );
    let mut treatment = run(
        &plan,
        BehavioralTrialArmKind::Treatment,
        1,
        "session-shared",
        "fresh-shared",
        "worker:shared",
        "block_patch",
    );
    treatment.metadata.worker_identity = "other-worker@v1".into();

    let result = classify_behavioral_trial_run_pair(&pair(&plan, control, treatment)).unwrap();
    assert_eq!(result.verdict, BehavioralTrialPairVerdict::Confounded);
    assert_eq!(
        result.reasons,
        vec![
            BehavioralTrialPairReason::WorkerIdentityDrift,
            BehavioralTrialPairReason::ReusedSessionId,
            BehavioralTrialPairReason::ReusedFreshnessReceipt,
            BehavioralTrialPairReason::ReusedWorkerRef,
            BehavioralTrialPairReason::InvalidSequenceCoverage,
        ]
    );
}

#[test]
fn tampered_individual_receipt_rejects_instead_of_becoming_confounded() {
    let plan = plan();
    let control = run(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "session-control",
        "fresh-control",
        "worker:control",
        "inspect_accepted_guard_detail",
    );
    let mut treatment = run(
        &plan,
        BehavioralTrialArmKind::Treatment,
        2,
        "session-treatment",
        "fresh-treatment",
        "worker:treatment",
        "block_patch",
    );
    treatment.raw_output.push(' ');

    let error = classify_behavioral_trial_run_pair(&pair(&plan, control, treatment)).unwrap_err();
    assert!(error.to_string().contains("retained bytes"));
}

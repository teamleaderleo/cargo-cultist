#[allow(dead_code)]
#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[allow(dead_code)]
#[path = "../src/behavioral_trial_run.rs"]
mod behavioral_trial_run;

use behavioral_trial::{
    BehavioralTrialArmKind, BehavioralTrialPlan, fingerprint_plan, materialize_worker_packet,
    parse_behavioral_trial_plan,
};
use behavioral_trial_run::{
    BehavioralTrialRunReceipt, BehavioralTrialRunVerdict, canonical_worker_packet_file_sha256,
    evaluate_behavioral_trial_runs,
};

const PLAN: &[u8] =
    include_bytes!("../research/behavioral-trials/stensibly-index-guard-detail.json");
const CONTROL_FILE_SHA: &str = "6a568aed1eb660141cd7e7759e47edeb10a5c759fe1402689699dd7b6837149e";
const TREATMENT_FILE_SHA: &str = "1063efc8ecdf0313b947923dad8216fb9fa43b2e8b8cafa7ab6b63d53eb65c7d";
const SAMPLING_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OUTPUT_SHA: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn plan() -> BehavioralTrialPlan {
    parse_behavioral_trial_plan(PLAN).expect("retained Stensibly plan should parse")
}

fn receipt(
    plan: &BehavioralTrialPlan,
    arm: BehavioralTrialArmKind,
    sequence_index: u32,
    first_action_id: &str,
) -> BehavioralTrialRunReceipt {
    let packet = materialize_worker_packet(plan, arm).expect("fixture packet should materialize");
    BehavioralTrialRunReceipt {
        schema_version: 1,
        trial_id: plan.trial_id.clone(),
        pair_id: "pair-001".into(),
        run_id: format!("run-{sequence_index}"),
        sequence_index,
        plan_fingerprint: fingerprint_plan(plan).expect("fixture plan should fingerprint"),
        worker_packet_fingerprint: packet.worker_packet_fingerprint.clone(),
        worker_packet_file_sha256: canonical_worker_packet_file_sha256(&packet)
            .expect("fixture packet should serialize"),
        worker_ref: format!("worker-session-{sequence_index}"),
        worker_identity: "fixed-worker@v1".into(),
        harness_identity: "first-action-harness@v1".into(),
        affordance_identity: "packet-only-choice@v1".into(),
        sampling_config_sha256: SAMPLING_SHA.into(),
        session_id: format!("session-{sequence_index}"),
        fresh_session: true,
        prior_condition_exposure: false,
        raw_worker_output_sha256: OUTPUT_SHA.into(),
        first_action_id: first_action_id.into(),
    }
}

#[test]
fn retained_neutral_plan_reproduces_exact_262_packet_file_hashes() {
    let plan = plan();
    let control = materialize_worker_packet(&plan, BehavioralTrialArmKind::Control).unwrap();
    let treatment = materialize_worker_packet(&plan, BehavioralTrialArmKind::Treatment).unwrap();

    assert_eq!(
        canonical_worker_packet_file_sha256(&control).unwrap(),
        CONTROL_FILE_SHA
    );
    assert_eq!(
        canonical_worker_packet_file_sha256(&treatment).unwrap(),
        TREATMENT_FILE_SHA
    );
}

#[test]
fn admitted_ab_pair_reuses_existing_behavioral_reconciliation() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "inspect_accepted_guard_detail",
    );
    let treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 2, "block_patch");

    let result = evaluate_behavioral_trial_runs(&plan, &control, &treatment).unwrap();
    assert_eq!(result.verdict, BehavioralTrialRunVerdict::Admitted);
    assert!(result.frozen_identity_match);
    assert!(result.fresh_uncontaminated_sessions);
    assert!(result.distinct_arm_coverage);
    assert!(!result.automatic_effect_claim);
    assert!(!result.automatic_generalization);

    let behavioral = result
        .behavioral_evaluation
        .expect("admitted pair should reconcile");
    assert_eq!(
        behavioral.control.first_action_id,
        "inspect_accepted_guard_detail"
    );
    assert_eq!(behavioral.treatment.first_action_id, "block_patch");
    assert!(!behavioral.same_first_action);
}

#[test]
fn admitted_ba_order_maps_arms_by_packet_fingerprint() {
    let plan = plan();
    let treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 1, "block_patch");
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        2,
        "inspect_accepted_guard_detail",
    );

    let result = evaluate_behavioral_trial_runs(&plan, &treatment, &control).unwrap();
    assert_eq!(result.verdict, BehavioralTrialRunVerdict::Admitted);
    let behavioral = result.behavioral_evaluation.unwrap();
    assert_eq!(
        behavioral.control.first_action_id,
        "inspect_accepted_guard_detail"
    );
    assert_eq!(behavioral.treatment.first_action_id, "block_patch");
}

#[test]
fn admitted_pair_can_preserve_the_same_first_action() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "inspect_more_repository_context",
    );
    let treatment = receipt(
        &plan,
        BehavioralTrialArmKind::Treatment,
        2,
        "inspect_more_repository_context",
    );

    let result = evaluate_behavioral_trial_runs(&plan, &control, &treatment).unwrap();
    assert_eq!(result.verdict, BehavioralTrialRunVerdict::Admitted);
    assert!(result.behavioral_evaluation.unwrap().same_first_action);
}

#[test]
fn frozen_worker_harness_affordance_or_sampling_drift_confounds_pair() {
    let plan = plan();
    let baseline = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "inspect_accepted_guard_detail",
    );

    let mut mutations: Vec<Box<dyn Fn(&mut BehavioralTrialRunReceipt)>> = vec![
        Box::new(|receipt| receipt.worker_identity = "other-worker@v1".into()),
        Box::new(|receipt| receipt.harness_identity = "other-harness@v1".into()),
        Box::new(|receipt| receipt.affordance_identity = "other-affordance@v1".into()),
        Box::new(|receipt| {
            receipt.sampling_config_sha256 =
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into()
        }),
    ];

    for mutate in mutations.drain(..) {
        let mut treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 2, "block_patch");
        mutate(&mut treatment);
        let result = evaluate_behavioral_trial_runs(&plan, &baseline, &treatment).unwrap();
        assert_eq!(result.verdict, BehavioralTrialRunVerdict::Confounded);
        assert!(!result.frozen_identity_match);
        assert!(result.behavioral_evaluation.is_none());
    }
}

#[test]
fn reused_nonfresh_or_preexposed_sessions_confound_pair() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "inspect_accepted_guard_detail",
    );

    let mut treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 2, "block_patch");
    treatment.session_id = control.session_id.clone();
    let result = evaluate_behavioral_trial_runs(&plan, &control, &treatment).unwrap();
    assert_eq!(result.verdict, BehavioralTrialRunVerdict::Confounded);
    assert!(!result.fresh_uncontaminated_sessions);

    let mut treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 2, "block_patch");
    treatment.fresh_session = false;
    let result = evaluate_behavioral_trial_runs(&plan, &control, &treatment).unwrap();
    assert_eq!(result.verdict, BehavioralTrialRunVerdict::Confounded);

    let mut treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 2, "block_patch");
    treatment.prior_condition_exposure = true;
    let result = evaluate_behavioral_trial_runs(&plan, &control, &treatment).unwrap();
    assert_eq!(result.verdict, BehavioralTrialRunVerdict::Confounded);
}

#[test]
fn same_packet_twice_is_an_invalid_pair_without_behavioral_interpretation() {
    let plan = plan();
    let first = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "inspect_accepted_guard_detail",
    );
    let second = receipt(&plan, BehavioralTrialArmKind::Control, 2, "approve_patch");

    let result = evaluate_behavioral_trial_runs(&plan, &first, &second).unwrap();
    assert_eq!(result.verdict, BehavioralTrialRunVerdict::InvalidPair);
    assert!(!result.distinct_arm_coverage);
    assert!(result.behavioral_evaluation.is_none());
}

#[test]
fn packet_file_hash_tampering_rejects_before_pair_interpretation() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "inspect_accepted_guard_detail",
    );
    let mut treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 2, "block_patch");
    treatment.worker_packet_file_sha256 = CONTROL_FILE_SHA.into();

    let error = evaluate_behavioral_trial_runs(&plan, &control, &treatment).unwrap_err();
    assert!(error.contains("file SHA256"));
}

#[test]
fn unknown_packet_fingerprint_rejects_before_pair_interpretation() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "inspect_accepted_guard_detail",
    );
    let mut treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 2, "block_patch");
    treatment.worker_packet_fingerprint =
        "cultist-behavioral-worker-packet-sha256-v1:0000000000000000000000000000000000000000000000000000000000000000"
            .into();

    let error = evaluate_behavioral_trial_runs(&plan, &control, &treatment).unwrap_err();
    assert!(error.contains("unknown worker-packet fingerprint"));
}

#[test]
fn action_outside_registered_vocabulary_rejects_before_pair_interpretation() {
    let plan = plan();
    let control = receipt(
        &plan,
        BehavioralTrialArmKind::Control,
        1,
        "inspect_accepted_guard_detail",
    );
    let mut treatment = receipt(&plan, BehavioralTrialArmKind::Treatment, 2, "block_patch");
    treatment.first_action_id = "invent_new_action".into();

    let error = evaluate_behavioral_trial_runs(&plan, &control, &treatment).unwrap_err();
    assert!(error.contains("outside the registered action vocabulary"));
}

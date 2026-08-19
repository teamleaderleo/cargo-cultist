#[allow(dead_code)]
#[path = "../src/capability_demand_retirement.rs"]
mod capability_demand_retirement;

use capability_demand_retirement::{
    EvidenceInspection, RetirementVerdict, RunOutcome, WorkerRunReceipt, evaluate_pair,
    parse_trial_manifest, parse_trial_spec,
};

const TRIAL: &[u8] = include_bytes!(
    "../research/capability-demand-retirement/stensibly-convex-index-review-v1.json"
);
const MANIFEST: &[u8] = include_bytes!(
    "../research/capability-demand-retirement/stensibly-convex-index-review-v1-input-manifest-32264661913.json"
);

const TASK_SHA: &str = "84aaf8f8d3b6880017d25432c763fea5732306117144fc37415992969754f873";
const PATCH_SHA: &str = "647281f95818f22784c7468e208bd2b9b5cb2c34ecb38be4b929cf4019c89ba5";
const ORACLE_SHA: &str = "bce60719e97d861a9223661d349b3171ee3eafb3f87e9042999a32a6bd39771b";
const BASELINE_PACKET_SHA: &str =
    "e4549a10f86448779a21307d97ea75f2ae1acfd15b43099cbca6f600f0781bdf";
const TREATMENT_PACKET_SHA: &str =
    "2e6acd81e324f3b290b9f93c5bf0ec3d9a66004bd44a855069cc65802f94af46";
const SAMPLING_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RESET_SHA: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OUTPUT_SHA: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn trial_and_manifest() -> (
    capability_demand_retirement::TrialSpec,
    capability_demand_retirement::TrialInputManifest,
) {
    (
        parse_trial_spec(TRIAL).expect("valid retained trial"),
        parse_trial_manifest(MANIFEST).expect("valid retained manifest"),
    )
}

fn receipt(condition: &str, sequence_index: u32, outcome: RunOutcome) -> WorkerRunReceipt {
    let packet = match condition {
        "file_local_jei" => BASELINE_PACKET_SHA,
        "scoped_jei" => TREATMENT_PACKET_SHA,
        _ => panic!("unknown fixture condition"),
    };
    WorkerRunReceipt {
        schema_version: 1,
        trial_id: "stensibly-convex-index-review-v1".into(),
        pair_id: "pair-001".into(),
        run_id: format!("run-{sequence_index}"),
        condition_id: condition.into(),
        sequence_index,
        repository: "teamleaderleo/stensibly".into(),
        revision: "85cecf2608ad9e734a67518577fa85b9a08a550c".into(),
        target_path: "convex/schema.ts".into(),
        target_blob_sha: "7fdd51e2f9fba80d1c0a814cea708d601a7b9925".into(),
        task_sha256: TASK_SHA.into(),
        patch_sha256: PATCH_SHA.into(),
        evidence_packet_sha256: packet.into(),
        completion_contract_sha256: ORACLE_SHA.into(),
        worker_identity: "fixed-worker@v1".into(),
        harness_identity: "review-harness@v1".into(),
        affordance_identity: "read-only-repository-review@v1".into(),
        sampling_config_sha256: SAMPLING_SHA.into(),
        session_id: format!("session-{sequence_index}"),
        fresh_session: true,
        prior_condition_exposure: false,
        checkout_reset_receipt_sha256: RESET_SHA.into(),
        worker_output_sha256: OUTPUT_SHA.into(),
        evaluated_outcome: outcome,
        evidence_inspection: EvidenceInspection::Unobservable,
        context_expanded: false,
    }
}

#[test]
fn retained_manifest_binds_the_executed_input_artifacts() {
    let (trial, manifest) = trial_and_manifest();
    assert_eq!(manifest.trial_id, trial.trial_id);
    assert_eq!(manifest.worker_visible_common.task.sha256, TASK_SHA);
    assert_eq!(manifest.worker_visible_common.task.bytes, 223);
    assert_eq!(manifest.worker_visible_common.patch.sha256, PATCH_SHA);
    assert_eq!(manifest.worker_visible_common.patch.bytes, 321);
    assert_eq!(manifest.evaluator_only.oracle.sha256, ORACLE_SHA);
    assert_eq!(manifest.evaluator_only.oracle.bytes, 322);
    assert_eq!(
        manifest.conditions["file_local_jei"].packet.sha256,
        BASELINE_PACKET_SHA
    );
    assert_eq!(manifest.conditions["file_local_jei"].packet.bytes, 14_086);
    assert!(!manifest.conditions["file_local_jei"].decisive_evidence_present);
    assert_eq!(
        manifest.conditions["scoped_jei"].packet.sha256,
        TREATMENT_PACKET_SHA
    );
    assert_eq!(manifest.conditions["scoped_jei"].packet.bytes, 18_795);
    assert!(manifest.conditions["scoped_jei"].decisive_evidence_present);
}

#[test]
fn failed_baseline_then_successful_treatment_is_a_paired_signal() {
    let (trial, manifest) = trial_and_manifest();
    let baseline = receipt("file_local_jei", 1, RunOutcome::Failed);
    let treatment = receipt("scoped_jei", 2, RunOutcome::Success);
    let result = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap();

    assert_eq!(result.verdict, RetirementVerdict::PairedRetirementSignal);
    assert_eq!(
        result.baseline_condition_id.as_deref(),
        Some("file_local_jei")
    );
    assert_eq!(result.treatment_condition_id.as_deref(), Some("scoped_jei"));
    assert!(result.frozen_identity_match);
    assert!(result.fresh_uncontaminated_sessions);
    assert!(result.decisive_evidence_flip);
    assert!(!result.automatic_causal_claim);
    assert!(!result.automatic_generalization);
}

#[test]
fn pair_order_does_not_define_baseline_and_treatment_roles() {
    let (trial, manifest) = trial_and_manifest();
    let treatment = receipt("scoped_jei", 1, RunOutcome::Success);
    let baseline = receipt("file_local_jei", 2, RunOutcome::Failed);
    let result = evaluate_pair(&trial, &manifest, &treatment, &baseline).unwrap();

    assert_eq!(result.verdict, RetirementVerdict::PairedRetirementSignal);
    assert_eq!(
        result.baseline_condition_id.as_deref(),
        Some("file_local_jei")
    );
    assert_eq!(result.treatment_condition_id.as_deref(), Some("scoped_jei"));
}

#[test]
fn baseline_success_means_no_success_demand_was_observed() {
    let (trial, manifest) = trial_and_manifest();
    let baseline = receipt("file_local_jei", 1, RunOutcome::Success);
    let treatment = receipt("scoped_jei", 2, RunOutcome::Success);
    let result = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap();
    assert_eq!(result.verdict, RetirementVerdict::NoDemandObserved);
}

#[test]
fn correct_escalation_then_success_is_preserved_separately() {
    let (trial, manifest) = trial_and_manifest();
    let baseline = receipt("file_local_jei", 1, RunOutcome::CorrectEscalation);
    let treatment = receipt("scoped_jei", 2, RunOutcome::Success);
    let result = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap();
    assert_eq!(
        result.verdict,
        RetirementVerdict::CorrectEscalationThenSuccess
    );
}

#[test]
fn treatment_failure_preserves_demand() {
    let (trial, manifest) = trial_and_manifest();
    let baseline = receipt("file_local_jei", 1, RunOutcome::Failed);
    let treatment = receipt("scoped_jei", 2, RunOutcome::Failed);
    let result = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap();
    assert_eq!(result.verdict, RetirementVerdict::DemandPersists);
}

#[test]
fn worker_or_harness_identity_drift_confounds_the_pair() {
    let (trial, manifest) = trial_and_manifest();
    let baseline = receipt("file_local_jei", 1, RunOutcome::Failed);
    let mut treatment = receipt("scoped_jei", 2, RunOutcome::Success);
    treatment.worker_identity = "different-worker@v1".into();
    let result = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap();
    assert_eq!(result.verdict, RetirementVerdict::Confounded);

    let mut treatment = receipt("scoped_jei", 2, RunOutcome::Success);
    treatment.harness_identity = "different-harness@v1".into();
    let result = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap();
    assert_eq!(result.verdict, RetirementVerdict::Confounded);
}

#[test]
fn reused_or_treatment_contaminated_sessions_confound_the_pair() {
    let (trial, manifest) = trial_and_manifest();
    let baseline = receipt("file_local_jei", 1, RunOutcome::Failed);
    let mut treatment = receipt("scoped_jei", 2, RunOutcome::Success);
    treatment.session_id = baseline.session_id.clone();
    let result = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap();
    assert_eq!(result.verdict, RetirementVerdict::Confounded);

    let mut treatment = receipt("scoped_jei", 2, RunOutcome::Success);
    treatment.prior_condition_exposure = true;
    let result = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap();
    assert_eq!(result.verdict, RetirementVerdict::Confounded);
}

#[test]
fn same_condition_twice_is_invalid_evidence_pair() {
    let (trial, manifest) = trial_and_manifest();
    let first = receipt("file_local_jei", 1, RunOutcome::Failed);
    let second = receipt("file_local_jei", 2, RunOutcome::Success);
    let result = evaluate_pair(&trial, &manifest, &first, &second).unwrap();
    assert_eq!(result.verdict, RetirementVerdict::InvalidEvidencePair);
    assert!(!result.decisive_evidence_flip);
    assert!(result.baseline_condition_id.is_none());
    assert!(result.treatment_condition_id.is_none());
}

#[test]
fn packet_fingerprint_tampering_rejects_before_pair_interpretation() {
    let (trial, manifest) = trial_and_manifest();
    let baseline = receipt("file_local_jei", 1, RunOutcome::Failed);
    let mut treatment = receipt("scoped_jei", 2, RunOutcome::Success);
    treatment.evidence_packet_sha256 = BASELINE_PACKET_SHA.into();

    let error = evaluate_pair(&trial, &manifest, &baseline, &treatment).unwrap_err();
    assert!(error.contains("evidence packet fingerprint"));
}

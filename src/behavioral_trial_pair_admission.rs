use serde::{Deserialize, Serialize};

use crate::behavioral_trial::{
    BehavioralTrialArmKind, BehavioralTrialPlan, fingerprint_plan, materialize_worker_packet,
};
use crate::behavioral_trial_run::{
    BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION, BehavioralTrialRunPair, BehavioralTrialRunPairEvaluation,
    BehavioralTrialRunReceipt, build_behavioral_trial_run_receipt,
    evaluate_behavioral_trial_run_pair,
};

pub const BEHAVIORAL_TRIAL_PAIR_ADMISSION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralTrialPairAdmissionVerdict {
    Admitted,
    Confounded,
    InvalidPair,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralTrialPairAdmissionReason {
    SameArm,
    WorkerIdentityDrift,
    HarnessIdentityDrift,
    AffordanceIdentityDrift,
    SamplingConfigDrift,
    NonFreshSession,
    PriorConditionExposure,
    ReusedSessionId,
    ReusedFreshnessReceipt,
    ReusedWorkerRef,
    InvalidSequenceCoverage,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialPairAdmissionEvaluation {
    pub schema_version: u32,
    pub trial_id: String,
    pub plan_fingerprint: String,
    pub execution_order_packet_fingerprints: Vec<String>,
    pub sequence_indexes: Vec<u8>,
    pub session_ids: Vec<String>,
    pub freshness_receipts: Vec<String>,
    pub worker_refs: Vec<String>,
    pub verdict: BehavioralTrialPairAdmissionVerdict,
    pub reasons: Vec<BehavioralTrialPairAdmissionReason>,
    pub frozen_identity_match: bool,
    pub fresh_uncontaminated_sessions: bool,
    pub distinct_arm_coverage: bool,
    pub behavioral_evaluation: Option<BehavioralTrialRunPairEvaluation>,
    pub automatic_effect_claim: bool,
    pub automatic_generalization: bool,
}

pub fn evaluate_behavioral_trial_pair_admission(
    pair: &BehavioralTrialRunPair,
) -> Result<BehavioralTrialPairAdmissionEvaluation, String> {
    if pair.schema_version != BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION {
        return Err(format!(
            "unsupported behavioral-trial run-pair schema {}; expected {BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION}",
            pair.schema_version
        ));
    }
    if pair.runs.len() != 2 {
        return Err("behavioral-trial pair admission requires exactly two run receipts".into());
    }

    let plan = pair.plan.as_ref();
    for receipt in &pair.runs {
        authenticate_receipt(plan, receipt)?;
    }

    let first = &pair.runs[0];
    let second = &pair.runs[1];
    let first_arm = receipt_arm(plan, first)?;
    let second_arm = receipt_arm(plan, second)?;

    let mut reasons = Vec::new();
    if first_arm == second_arm {
        reasons.push(BehavioralTrialPairAdmissionReason::SameArm);
    }
    if first.metadata.worker_identity != second.metadata.worker_identity {
        reasons.push(BehavioralTrialPairAdmissionReason::WorkerIdentityDrift);
    }
    if first.metadata.harness_identity != second.metadata.harness_identity {
        reasons.push(BehavioralTrialPairAdmissionReason::HarnessIdentityDrift);
    }
    if first.metadata.affordance_identity != second.metadata.affordance_identity {
        reasons.push(BehavioralTrialPairAdmissionReason::AffordanceIdentityDrift);
    }
    if first.metadata.sampling_config_sha256 != second.metadata.sampling_config_sha256 {
        reasons.push(BehavioralTrialPairAdmissionReason::SamplingConfigDrift);
    }
    if !first.metadata.fresh_session || !second.metadata.fresh_session {
        reasons.push(BehavioralTrialPairAdmissionReason::NonFreshSession);
    }
    if first.metadata.prior_condition_exposure || second.metadata.prior_condition_exposure {
        reasons.push(BehavioralTrialPairAdmissionReason::PriorConditionExposure);
    }
    if first.metadata.session_id == second.metadata.session_id {
        reasons.push(BehavioralTrialPairAdmissionReason::ReusedSessionId);
    }
    if first.metadata.freshness_receipt == second.metadata.freshness_receipt {
        reasons.push(BehavioralTrialPairAdmissionReason::ReusedFreshnessReceipt);
    }
    if first.observation.worker_ref == second.observation.worker_ref {
        reasons.push(BehavioralTrialPairAdmissionReason::ReusedWorkerRef);
    }
    let mut sequence = [
        first.metadata.sequence_index,
        second.metadata.sequence_index,
    ];
    sequence.sort_unstable();
    if sequence != [1, 2] {
        reasons.push(BehavioralTrialPairAdmissionReason::InvalidSequenceCoverage);
    }
    reasons.sort_unstable();
    reasons.dedup();

    let distinct_arm_coverage = !reasons.contains(&BehavioralTrialPairAdmissionReason::SameArm);
    let frozen_identity_match = !reasons.iter().any(|reason| {
        matches!(
            reason,
            BehavioralTrialPairAdmissionReason::WorkerIdentityDrift
                | BehavioralTrialPairAdmissionReason::HarnessIdentityDrift
                | BehavioralTrialPairAdmissionReason::AffordanceIdentityDrift
                | BehavioralTrialPairAdmissionReason::SamplingConfigDrift
        )
    });
    let fresh_uncontaminated_sessions = !reasons.iter().any(|reason| {
        matches!(
            reason,
            BehavioralTrialPairAdmissionReason::NonFreshSession
                | BehavioralTrialPairAdmissionReason::PriorConditionExposure
                | BehavioralTrialPairAdmissionReason::ReusedSessionId
                | BehavioralTrialPairAdmissionReason::ReusedFreshnessReceipt
                | BehavioralTrialPairAdmissionReason::ReusedWorkerRef
                | BehavioralTrialPairAdmissionReason::InvalidSequenceCoverage
        )
    });

    let verdict = if !distinct_arm_coverage {
        BehavioralTrialPairAdmissionVerdict::InvalidPair
    } else if reasons.is_empty() {
        BehavioralTrialPairAdmissionVerdict::Admitted
    } else {
        BehavioralTrialPairAdmissionVerdict::Confounded
    };

    let behavioral_evaluation = if verdict == BehavioralTrialPairAdmissionVerdict::Admitted {
        Some(evaluate_behavioral_trial_run_pair(pair).map_err(|error| error.to_string())?)
    } else {
        None
    };

    Ok(BehavioralTrialPairAdmissionEvaluation {
        schema_version: BEHAVIORAL_TRIAL_PAIR_ADMISSION_SCHEMA_VERSION,
        trial_id: plan.trial_id.clone(),
        plan_fingerprint: fingerprint_plan(plan).map_err(|error| error.to_string())?,
        execution_order_packet_fingerprints: pair
            .runs
            .iter()
            .map(|receipt| receipt.observation.worker_packet_fingerprint.clone())
            .collect(),
        sequence_indexes: pair
            .runs
            .iter()
            .map(|receipt| receipt.metadata.sequence_index)
            .collect(),
        session_ids: pair
            .runs
            .iter()
            .map(|receipt| receipt.metadata.session_id.clone())
            .collect(),
        freshness_receipts: pair
            .runs
            .iter()
            .map(|receipt| receipt.metadata.freshness_receipt.clone())
            .collect(),
        worker_refs: pair
            .runs
            .iter()
            .map(|receipt| receipt.observation.worker_ref.clone())
            .collect(),
        verdict,
        reasons,
        frozen_identity_match,
        fresh_uncontaminated_sessions,
        distinct_arm_coverage,
        behavioral_evaluation,
        automatic_effect_claim: false,
        automatic_generalization: false,
    })
}

fn authenticate_receipt(
    plan: &BehavioralTrialPlan,
    receipt: &BehavioralTrialRunReceipt,
) -> Result<(), String> {
    let rebuilt = build_behavioral_trial_run_receipt(
        plan,
        receipt.metadata.clone(),
        receipt.raw_worker_packet.as_bytes(),
        receipt.raw_output.as_bytes(),
    )
    .map_err(|error| error.to_string())?;
    if &rebuilt != receipt {
        return Err(
            "behavioral-trial run receipt fields do not match the byte-authentic rebuild".into(),
        );
    }
    Ok(())
}

fn receipt_arm(
    plan: &BehavioralTrialPlan,
    receipt: &BehavioralTrialRunReceipt,
) -> Result<BehavioralTrialArmKind, String> {
    let control = materialize_worker_packet(plan, BehavioralTrialArmKind::Control)
        .map_err(|error| error.to_string())?;
    let treatment = materialize_worker_packet(plan, BehavioralTrialArmKind::Treatment)
        .map_err(|error| error.to_string())?;
    let fingerprint = &receipt.observation.worker_packet_fingerprint;
    if fingerprint == &control.worker_packet_fingerprint {
        Ok(BehavioralTrialArmKind::Control)
    } else if fingerprint == &treatment.worker_packet_fingerprint {
        Ok(BehavioralTrialArmKind::Treatment)
    } else {
        Err("authenticated run receipt names a packet outside the frozen plan".into())
    }
}

use std::error::Error;
use std::fmt;

use serde::Serialize;

use crate::behavioral_trial::{
    BehavioralTrialArmKind, fingerprint_plan, materialize_worker_packet,
};
use crate::behavioral_trial_run::{
    BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION, BehavioralTrialRunPair,
    BehavioralTrialRunPairEvaluation, BehavioralTrialRunReceipt,
    build_behavioral_trial_run_receipt, evaluate_behavioral_trial_run_pair,
};

pub const BEHAVIORAL_TRIAL_PAIR_CLASSIFICATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralTrialPairVerdict {
    Admitted,
    Confounded,
    InvalidPair,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralTrialPairReason {
    SameArm,
    WorkerIdentityDrift,
    HarnessIdentityDrift,
    AffordanceIdentityDrift,
    SamplingConfigDrift,
    ReusedSessionId,
    ReusedFreshnessReceipt,
    ReusedWorkerRef,
    InvalidSequenceCoverage,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialPairClassification {
    pub schema_version: u32,
    pub trial_id: String,
    pub plan_fingerprint: String,
    pub verdict: BehavioralTrialPairVerdict,
    pub reasons: Vec<BehavioralTrialPairReason>,
    pub evaluation: Option<BehavioralTrialRunPairEvaluation>,
    pub automatic_effect_claim: bool,
    pub automatic_generalization: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BehavioralTrialPairClassificationError {
    message: String,
}

impl BehavioralTrialPairClassificationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BehavioralTrialPairClassificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for BehavioralTrialPairClassificationError {}

pub fn classify_behavioral_trial_run_pair(
    pair: &BehavioralTrialRunPair,
) -> Result<BehavioralTrialPairClassification, BehavioralTrialPairClassificationError> {
    if pair.schema_version != BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION {
        return Err(BehavioralTrialPairClassificationError::new(format!(
            "unsupported behavioral-trial run-pair schema {}; expected {BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION}",
            pair.schema_version
        )));
    }
    if pair.runs.len() != 2 {
        return Err(BehavioralTrialPairClassificationError::new(
            "behavioral-trial pair classification requires exactly two run receipts",
        ));
    }

    let plan = pair.plan.as_ref();
    for receipt in &pair.runs {
        validate_individual_receipt(plan, receipt)?;
    }

    let control = materialize_worker_packet(plan, BehavioralTrialArmKind::Control)
        .map_err(source_error)?;
    let treatment = materialize_worker_packet(plan, BehavioralTrialArmKind::Treatment)
        .map_err(source_error)?;
    let first_arm = arm_for_receipt(&pair.runs[0], &control, &treatment)?;
    let second_arm = arm_for_receipt(&pair.runs[1], &control, &treatment)?;

    let first = &pair.runs[0];
    let second = &pair.runs[1];
    let mut reasons = Vec::new();

    if first_arm == second_arm {
        reasons.push(BehavioralTrialPairReason::SameArm);
    }
    if first.metadata.worker_identity != second.metadata.worker_identity {
        reasons.push(BehavioralTrialPairReason::WorkerIdentityDrift);
    }
    if first.metadata.harness_identity != second.metadata.harness_identity {
        reasons.push(BehavioralTrialPairReason::HarnessIdentityDrift);
    }
    if first.metadata.affordance_identity != second.metadata.affordance_identity {
        reasons.push(BehavioralTrialPairReason::AffordanceIdentityDrift);
    }
    if first.metadata.sampling_config_sha256 != second.metadata.sampling_config_sha256 {
        reasons.push(BehavioralTrialPairReason::SamplingConfigDrift);
    }
    if first.metadata.session_id == second.metadata.session_id {
        reasons.push(BehavioralTrialPairReason::ReusedSessionId);
    }
    if first.metadata.freshness_receipt == second.metadata.freshness_receipt {
        reasons.push(BehavioralTrialPairReason::ReusedFreshnessReceipt);
    }
    if first.observation.worker_ref == second.observation.worker_ref {
        reasons.push(BehavioralTrialPairReason::ReusedWorkerRef);
    }
    let mut sequence = [
        first.metadata.sequence_index,
        second.metadata.sequence_index,
    ];
    sequence.sort_unstable();
    if sequence != [1, 2] {
        reasons.push(BehavioralTrialPairReason::InvalidSequenceCoverage);
    }
    reasons.sort_unstable();
    reasons.dedup();

    let verdict = if reasons.contains(&BehavioralTrialPairReason::SameArm) {
        BehavioralTrialPairVerdict::InvalidPair
    } else if reasons.is_empty() {
        BehavioralTrialPairVerdict::Admitted
    } else {
        BehavioralTrialPairVerdict::Confounded
    };

    let evaluation = if verdict == BehavioralTrialPairVerdict::Admitted {
        Some(evaluate_behavioral_trial_run_pair(pair).map_err(source_error)?)
    } else {
        None
    };

    Ok(BehavioralTrialPairClassification {
        schema_version: BEHAVIORAL_TRIAL_PAIR_CLASSIFICATION_SCHEMA_VERSION,
        trial_id: plan.trial_id.clone(),
        plan_fingerprint: fingerprint_plan(plan).map_err(source_error)?,
        verdict,
        reasons,
        evaluation,
        automatic_effect_claim: false,
        automatic_generalization: false,
    })
}

fn validate_individual_receipt(
    plan: &crate::behavioral_trial::BehavioralTrialPlan,
    receipt: &BehavioralTrialRunReceipt,
) -> Result<(), BehavioralTrialPairClassificationError> {
    let rebuilt = build_behavioral_trial_run_receipt(
        plan,
        receipt.metadata.clone(),
        receipt.raw_worker_packet.as_bytes(),
        receipt.raw_output.as_bytes(),
    )
    .map_err(source_error)?;
    if rebuilt != *receipt {
        return Err(BehavioralTrialPairClassificationError::new(
            "behavioral-trial run receipt differs from the receipt rebuilt from its exact retained bytes",
        ));
    }
    Ok(())
}

fn arm_for_receipt(
    receipt: &BehavioralTrialRunReceipt,
    control: &crate::behavioral_trial::BehavioralWorkerPacket,
    treatment: &crate::behavioral_trial::BehavioralWorkerPacket,
) -> Result<BehavioralTrialArmKind, BehavioralTrialPairClassificationError> {
    let fingerprint = &receipt.observation.worker_packet_fingerprint;
    if fingerprint == &control.worker_packet_fingerprint {
        Ok(BehavioralTrialArmKind::Control)
    } else if fingerprint == &treatment.worker_packet_fingerprint {
        Ok(BehavioralTrialArmKind::Treatment)
    } else {
        Err(BehavioralTrialPairClassificationError::new(
            "validated behavioral-trial receipt does not map to either frozen plan arm",
        ))
    }
}

fn source_error(error: impl fmt::Display) -> BehavioralTrialPairClassificationError {
    BehavioralTrialPairClassificationError::new(format!(
        "behavioral-trial pair source validation failed: {error}"
    ))
}

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::behavioral_trial::BehavioralTrialArmKind;
use crate::behavioral_trial::{
    BEHAVIORAL_TRIAL_SCHEMA_VERSION, BehavioralTrialEvaluation, BehavioralTrialObservation,
    BehavioralTrialPair, BehavioralTrialPlan, BehavioralWorkerPacket,
    evaluate_behavioral_trial_pair, fingerprint_plan, materialize_worker_packet,
};

pub const BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION: u32 = 1;
pub const MAX_BEHAVIORAL_TRIAL_RUN_RECEIPT_BYTES: usize = 128 * 1024;

const MAX_COORDINATE_BYTES: usize = 2048;
const SHA256_HEX_BYTES: usize = 64;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialRunReceipt {
    pub schema_version: u32,
    pub trial_id: String,
    pub pair_id: String,
    pub run_id: String,
    pub sequence_index: u32,
    pub plan_fingerprint: String,
    pub worker_packet_fingerprint: String,
    pub worker_packet_file_sha256: String,
    pub worker_ref: String,
    pub worker_identity: String,
    pub harness_identity: String,
    pub affordance_identity: String,
    pub sampling_config_sha256: String,
    pub session_id: String,
    pub fresh_session: bool,
    pub prior_condition_exposure: bool,
    pub raw_worker_output_sha256: String,
    pub first_action_id: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralTrialRunVerdict {
    Admitted,
    Confounded,
    InvalidPair,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialRunEvaluation {
    pub schema_version: u32,
    pub trial_id: String,
    pub plan_fingerprint: String,
    pub pair_id: String,
    pub run_ids: Vec<String>,
    pub execution_order_packet_fingerprints: Vec<String>,
    pub verdict: BehavioralTrialRunVerdict,
    pub frozen_identity_match: bool,
    pub fresh_uncontaminated_sessions: bool,
    pub distinct_arm_coverage: bool,
    pub behavioral_evaluation: Option<BehavioralTrialEvaluation>,
    pub automatic_effect_claim: bool,
    pub automatic_generalization: bool,
}

pub fn parse_behavioral_trial_run_receipt(
    bytes: &[u8],
) -> Result<BehavioralTrialRunReceipt, String> {
    if bytes.len() > MAX_BEHAVIORAL_TRIAL_RUN_RECEIPT_BYTES {
        return Err(format!(
            "behavioral trial run receipt exceeds the {}-byte limit",
            MAX_BEHAVIORAL_TRIAL_RUN_RECEIPT_BYTES
        ));
    }
    let receipt: BehavioralTrialRunReceipt =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    validate_receipt_shape(&receipt)?;
    Ok(receipt)
}

pub fn canonical_worker_packet_file_sha256(
    packet: &BehavioralWorkerPacket,
) -> Result<String, String> {
    let mut bytes = serde_json::to_string_pretty(packet)
        .map_err(|error| format!("failed to serialize worker packet: {error}"))?
        .into_bytes();
    bytes.push(b'\n');
    Ok(sha256_hex(&bytes))
}

pub fn evaluate_behavioral_trial_runs(
    plan: &BehavioralTrialPlan,
    first: &BehavioralTrialRunReceipt,
    second: &BehavioralTrialRunReceipt,
) -> Result<BehavioralTrialRunEvaluation, String> {
    let plan_fingerprint = fingerprint_plan(plan).map_err(|error| error.to_string())?;
    let control_packet = materialize_worker_packet(plan, BehavioralTrialArmKind::Control)
        .map_err(|error| error.to_string())?;
    let treatment_packet = materialize_worker_packet(plan, BehavioralTrialArmKind::Treatment)
        .map_err(|error| error.to_string())?;

    validate_receipt_against_plan(
        first,
        plan,
        &plan_fingerprint,
        &control_packet,
        &treatment_packet,
    )?;
    validate_receipt_against_plan(
        second,
        plan,
        &plan_fingerprint,
        &control_packet,
        &treatment_packet,
    )?;

    let frozen_identity_match = first.pair_id == second.pair_id
        && first.worker_identity == second.worker_identity
        && first.harness_identity == second.harness_identity
        && first.affordance_identity == second.affordance_identity
        && first.sampling_config_sha256 == second.sampling_config_sha256;

    let fresh_uncontaminated_sessions = first.fresh_session
        && second.fresh_session
        && !first.prior_condition_exposure
        && !second.prior_condition_exposure
        && first.session_id != second.session_id
        && first.run_id != second.run_id
        && BTreeSet::from([first.sequence_index, second.sequence_index]) == BTreeSet::from([1, 2]);

    let packet_set = BTreeSet::from([
        first.worker_packet_fingerprint.as_str(),
        second.worker_packet_fingerprint.as_str(),
    ]);
    let expected_packet_set = BTreeSet::from([
        control_packet.worker_packet_fingerprint.as_str(),
        treatment_packet.worker_packet_fingerprint.as_str(),
    ]);
    let distinct_arm_coverage = packet_set == expected_packet_set;

    let verdict = if !frozen_identity_match || !fresh_uncontaminated_sessions {
        BehavioralTrialRunVerdict::Confounded
    } else if !distinct_arm_coverage {
        BehavioralTrialRunVerdict::InvalidPair
    } else {
        BehavioralTrialRunVerdict::Admitted
    };

    let behavioral_evaluation = if verdict == BehavioralTrialRunVerdict::Admitted {
        let pair = BehavioralTrialPair {
            schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
            plan: Box::new(plan.clone()),
            observations: vec![observation(first), observation(second)],
        };
        Some(evaluate_behavioral_trial_pair(&pair).map_err(|error| error.to_string())?)
    } else {
        None
    };

    Ok(BehavioralTrialRunEvaluation {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        trial_id: plan.trial_id.clone(),
        plan_fingerprint,
        pair_id: first.pair_id.clone(),
        run_ids: vec![first.run_id.clone(), second.run_id.clone()],
        execution_order_packet_fingerprints: vec![
            first.worker_packet_fingerprint.clone(),
            second.worker_packet_fingerprint.clone(),
        ],
        verdict,
        frozen_identity_match,
        fresh_uncontaminated_sessions,
        distinct_arm_coverage,
        behavioral_evaluation,
        automatic_effect_claim: false,
        automatic_generalization: false,
    })
}

fn validate_receipt_shape(receipt: &BehavioralTrialRunReceipt) -> Result<(), String> {
    if receipt.schema_version != BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION {
        return Err(format!(
            "unsupported behavioral trial run receipt schema {}; expected {}",
            receipt.schema_version, BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION
        ));
    }
    for (value, field) in [
        (&receipt.trial_id, "trial_id"),
        (&receipt.pair_id, "pair_id"),
        (&receipt.run_id, "run_id"),
        (&receipt.plan_fingerprint, "plan_fingerprint"),
        (
            &receipt.worker_packet_fingerprint,
            "worker_packet_fingerprint",
        ),
        (&receipt.worker_ref, "worker_ref"),
        (&receipt.worker_identity, "worker_identity"),
        (&receipt.harness_identity, "harness_identity"),
        (&receipt.affordance_identity, "affordance_identity"),
        (&receipt.session_id, "session_id"),
        (&receipt.first_action_id, "first_action_id"),
    ] {
        validate_coordinate(value, field)?;
    }
    if !(1..=2).contains(&receipt.sequence_index) {
        return Err("sequence_index must be 1 or 2 for a paired behavioral replay".into());
    }
    validate_sha256(
        &receipt.worker_packet_file_sha256,
        "worker_packet_file_sha256",
    )?;
    validate_sha256(&receipt.sampling_config_sha256, "sampling_config_sha256")?;
    validate_sha256(
        &receipt.raw_worker_output_sha256,
        "raw_worker_output_sha256",
    )?;
    Ok(())
}

fn validate_receipt_against_plan(
    receipt: &BehavioralTrialRunReceipt,
    plan: &BehavioralTrialPlan,
    plan_fingerprint: &str,
    control_packet: &BehavioralWorkerPacket,
    treatment_packet: &BehavioralWorkerPacket,
) -> Result<(), String> {
    validate_receipt_shape(receipt)?;
    if receipt.trial_id != plan.trial_id {
        return Err(format!(
            "run {} trial_id does not match the registered plan",
            receipt.run_id
        ));
    }
    if receipt.plan_fingerprint != plan_fingerprint {
        return Err(format!(
            "run {} plan_fingerprint does not match the registered plan",
            receipt.run_id
        ));
    }

    let packet = if receipt.worker_packet_fingerprint == control_packet.worker_packet_fingerprint {
        control_packet
    } else if receipt.worker_packet_fingerprint == treatment_packet.worker_packet_fingerprint {
        treatment_packet
    } else {
        return Err(format!(
            "run {} names an unknown worker-packet fingerprint",
            receipt.run_id
        ));
    };

    let expected_file_sha256 = canonical_worker_packet_file_sha256(packet)?;
    if receipt.worker_packet_file_sha256 != expected_file_sha256 {
        return Err(format!(
            "run {} worker-packet file SHA256 does not match the canonical packet bytes",
            receipt.run_id
        ));
    }

    if !plan
        .allowed_first_actions
        .iter()
        .any(|action| action.id == receipt.first_action_id)
    {
        return Err(format!(
            "run {} first_action_id `{}` is outside the registered action vocabulary",
            receipt.run_id, receipt.first_action_id
        ));
    }
    Ok(())
}

fn observation(receipt: &BehavioralTrialRunReceipt) -> BehavioralTrialObservation {
    BehavioralTrialObservation {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        trial_id: receipt.trial_id.clone(),
        plan_fingerprint: receipt.plan_fingerprint.clone(),
        worker_packet_fingerprint: receipt.worker_packet_fingerprint.clone(),
        worker_ref: receipt.worker_ref.clone(),
        first_action_id: receipt.first_action_id.clone(),
    }
}

fn validate_coordinate(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_COORDINATE_BYTES
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(format!(
            "{field} must be a bounded non-empty canonical single-line coordinate"
        ));
    }
    Ok(())
}

fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    if value.len() != SHA256_HEX_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{field} must contain exactly 64 lowercase SHA-256 hex characters"
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(SHA256_HEX_BYTES);
    for byte in digest {
        output.push(hex_digit(byte >> 4));
        output.push(hex_digit(byte & 0x0f));
    }
    output
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => unreachable!("nibble is masked to four bits"),
    }
}

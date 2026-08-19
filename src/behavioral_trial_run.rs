use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::behavioral_trial::{
    BEHAVIORAL_TRIAL_SCHEMA_VERSION, BehavioralTrialArmKind, BehavioralTrialEvaluation,
    BehavioralTrialObservation, BehavioralTrialPair, BehavioralTrialPlan, BehavioralWorkerPacket,
    evaluate_behavioral_trial_pair, fingerprint_plan, materialize_worker_packet,
};

pub const BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION: u32 = 1;
pub const MAX_BEHAVIORAL_TRIAL_RUN_BYTES: usize = 512 * 1024;
const MAX_RAW_WORKER_PACKET_BYTES: usize = 256 * 1024;
const MAX_RAW_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_ID_BYTES: usize = 1024;
const MAX_RECEIPT_REF_BYTES: usize = 4096;
const SHA256_HEX_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralTrialExecutionOrigin {
    ExternalHarness,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialRunMetadata {
    pub schema_version: u32,
    pub execution_origin: BehavioralTrialExecutionOrigin,
    pub sequence_index: u8,
    pub worker_identity: String,
    pub harness_identity: String,
    pub affordance_identity: String,
    pub sampling_config_sha256: String,
    pub session_id: String,
    pub freshness_receipt: String,
    pub fresh_session: bool,
    pub prior_condition_exposure: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialRunReceipt {
    pub schema_version: u32,
    pub metadata: BehavioralTrialRunMetadata,
    pub worker_packet_sha256: String,
    pub raw_worker_packet: String,
    pub raw_output_sha256: String,
    pub raw_output: String,
    pub observation: BehavioralTrialObservation,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialRunPair {
    pub schema_version: u32,
    pub plan: Box<BehavioralTrialPlan>,
    pub runs: Vec<BehavioralTrialRunReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialRunPairEvaluation {
    pub schema_version: u32,
    pub trial: BehavioralTrialEvaluation,
    pub worker_identity: String,
    pub harness_identity: String,
    pub affordance_identity: String,
    pub sampling_config_sha256: String,
    pub control_sequence_index: u8,
    pub treatment_sequence_index: u8,
    pub control_session_id: String,
    pub treatment_session_id: String,
    pub control_freshness_receipt: String,
    pub treatment_freshness_receipt: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BehavioralTrialRunError {
    message: String,
}

impl BehavioralTrialRunError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BehavioralTrialRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for BehavioralTrialRunError {}

pub fn parse_behavioral_trial_run_metadata(
    bytes: &[u8],
) -> Result<BehavioralTrialRunMetadata, BehavioralTrialRunError> {
    enforce_bound(
        bytes,
        MAX_BEHAVIORAL_TRIAL_RUN_BYTES,
        "behavioral-trial run metadata",
    )?;
    let metadata: BehavioralTrialRunMetadata = serde_json::from_slice(bytes).map_err(|error| {
        BehavioralTrialRunError::new(format!(
            "invalid behavioral-trial run metadata JSON: {error}"
        ))
    })?;
    validate_metadata(&metadata)?;
    Ok(metadata)
}

pub fn parse_behavioral_trial_run_pair(
    bytes: &[u8],
) -> Result<BehavioralTrialRunPair, BehavioralTrialRunError> {
    enforce_bound(
        bytes,
        MAX_BEHAVIORAL_TRIAL_RUN_BYTES,
        "behavioral-trial run pair",
    )?;
    let pair: BehavioralTrialRunPair = serde_json::from_slice(bytes).map_err(|error| {
        BehavioralTrialRunError::new(format!("invalid behavioral-trial run pair JSON: {error}"))
    })?;
    if pair.schema_version != BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION {
        return Err(BehavioralTrialRunError::new(format!(
            "unsupported behavioral-trial run-pair schema {}; expected {BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION}",
            pair.schema_version
        )));
    }
    if pair.runs.len() != 2 {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial run pair requires exactly two run receipts",
        ));
    }
    Ok(pair)
}

pub fn build_behavioral_trial_run_receipt(
    plan: &BehavioralTrialPlan,
    metadata: BehavioralTrialRunMetadata,
    raw_worker_packet: &[u8],
    raw_output: &[u8],
) -> Result<BehavioralTrialRunReceipt, BehavioralTrialRunError> {
    validate_metadata(&metadata)?;
    enforce_bound(
        raw_worker_packet,
        MAX_RAW_WORKER_PACKET_BYTES,
        "raw behavioral worker packet",
    )?;
    enforce_bound(
        raw_output,
        MAX_RAW_OUTPUT_BYTES,
        "raw behavioral worker output",
    )?;

    let worker_packet: BehavioralWorkerPacket =
        serde_json::from_slice(raw_worker_packet).map_err(|error| {
            BehavioralTrialRunError::new(format!("invalid behavioral worker packet JSON: {error}"))
        })?;
    let arm = validate_worker_packet(plan, &worker_packet, raw_worker_packet)?;

    let observation: BehavioralTrialObservation =
        serde_json::from_slice(raw_output).map_err(|error| {
            BehavioralTrialRunError::new(format!("invalid behavioral worker output JSON: {error}"))
        })?;
    validate_observation_against_packet(plan, &worker_packet, &observation)?;

    let raw_worker_packet = std::str::from_utf8(raw_worker_packet)
        .map_err(|_| {
            BehavioralTrialRunError::new("raw behavioral worker packet must be UTF-8 JSON")
        })?
        .to_string();
    let raw_output = std::str::from_utf8(raw_output)
        .map_err(|_| {
            BehavioralTrialRunError::new("raw behavioral worker output must be UTF-8 JSON")
        })?
        .to_string();

    let receipt = BehavioralTrialRunReceipt {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        metadata,
        worker_packet_sha256: sha256_receipt(raw_worker_packet.as_bytes()),
        raw_worker_packet,
        raw_output_sha256: sha256_receipt(raw_output.as_bytes()),
        raw_output,
        observation,
    };
    validate_receipt(plan, &receipt)?;

    let expected = materialize_worker_packet(plan, arm).map_err(source_error)?;
    if expected.worker_packet_fingerprint != receipt.observation.worker_packet_fingerprint {
        return Err(BehavioralTrialRunError::new(
            "worker observation packet fingerprint changed after receipt construction",
        ));
    }
    Ok(receipt)
}

pub fn evaluate_behavioral_trial_run_pair(
    pair: &BehavioralTrialRunPair,
) -> Result<BehavioralTrialRunPairEvaluation, BehavioralTrialRunError> {
    if pair.schema_version != BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION {
        return Err(BehavioralTrialRunError::new(format!(
            "unsupported behavioral-trial run-pair schema {}; expected {BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION}",
            pair.schema_version
        )));
    }
    if pair.runs.len() != 2 {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial run pair requires exactly two run receipts",
        ));
    }

    let plan = pair.plan.as_ref();
    let first_arm = validate_receipt(plan, &pair.runs[0])?;
    let second_arm = validate_receipt(plan, &pair.runs[1])?;
    require_fresh_uncontaminated_run(&pair.runs[0].metadata)?;
    require_fresh_uncontaminated_run(&pair.runs[1].metadata)?;
    if first_arm == second_arm {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial run pair contains two receipts for the same arm",
        ));
    }

    let first = &pair.runs[0];
    let second = &pair.runs[1];
    require_same_pair_configuration(first, second)?;
    if first.metadata.session_id == second.metadata.session_id {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial pair requires distinct fresh session ids",
        ));
    }
    if first.metadata.freshness_receipt == second.metadata.freshness_receipt {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial pair requires distinct freshness evidence receipts",
        ));
    }
    if first.observation.worker_ref == second.observation.worker_ref {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial pair requires distinct worker run references",
        ));
    }
    let mut sequence = [
        first.metadata.sequence_index,
        second.metadata.sequence_index,
    ];
    sequence.sort_unstable();
    if sequence != [1, 2] {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial pair sequence indexes must be exactly 1 and 2",
        ));
    }

    let trial_pair = BehavioralTrialPair {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        plan: pair.plan.clone(),
        observations: vec![first.observation.clone(), second.observation.clone()],
    };
    let trial = evaluate_behavioral_trial_pair(&trial_pair).map_err(source_error)?;

    let (control, treatment) = if first_arm == BehavioralTrialArmKind::Control {
        (first, second)
    } else {
        (second, first)
    };

    Ok(BehavioralTrialRunPairEvaluation {
        schema_version: BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION,
        trial,
        worker_identity: first.metadata.worker_identity.clone(),
        harness_identity: first.metadata.harness_identity.clone(),
        affordance_identity: first.metadata.affordance_identity.clone(),
        sampling_config_sha256: first.metadata.sampling_config_sha256.clone(),
        control_sequence_index: control.metadata.sequence_index,
        treatment_sequence_index: treatment.metadata.sequence_index,
        control_session_id: control.metadata.session_id.clone(),
        treatment_session_id: treatment.metadata.session_id.clone(),
        control_freshness_receipt: control.metadata.freshness_receipt.clone(),
        treatment_freshness_receipt: treatment.metadata.freshness_receipt.clone(),
    })
}

fn validate_receipt(
    plan: &BehavioralTrialPlan,
    receipt: &BehavioralTrialRunReceipt,
) -> Result<BehavioralTrialArmKind, BehavioralTrialRunError> {
    if receipt.schema_version != BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION {
        return Err(BehavioralTrialRunError::new(format!(
            "unsupported behavioral-trial run schema {}; expected {BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION}",
            receipt.schema_version
        )));
    }
    validate_metadata(&receipt.metadata)?;
    enforce_bound(
        receipt.raw_worker_packet.as_bytes(),
        MAX_RAW_WORKER_PACKET_BYTES,
        "raw behavioral worker packet",
    )?;
    enforce_bound(
        receipt.raw_output.as_bytes(),
        MAX_RAW_OUTPUT_BYTES,
        "raw behavioral worker output",
    )?;
    require_sha256_match(
        &receipt.worker_packet_sha256,
        receipt.raw_worker_packet.as_bytes(),
        "worker_packet_sha256",
    )?;
    require_sha256_match(
        &receipt.raw_output_sha256,
        receipt.raw_output.as_bytes(),
        "raw_output_sha256",
    )?;

    let packet: BehavioralWorkerPacket =
        serde_json::from_str(&receipt.raw_worker_packet).map_err(|error| {
            BehavioralTrialRunError::new(format!(
                "invalid retained behavioral worker packet JSON: {error}"
            ))
        })?;
    let arm = validate_worker_packet(plan, &packet, receipt.raw_worker_packet.as_bytes())?;
    let observation: BehavioralTrialObservation = serde_json::from_str(&receipt.raw_output)
        .map_err(|error| {
            BehavioralTrialRunError::new(format!(
                "invalid retained behavioral worker output JSON: {error}"
            ))
        })?;
    if observation != receipt.observation {
        return Err(BehavioralTrialRunError::new(
            "retained behavioral observation disagrees with exact raw worker output",
        ));
    }
    validate_observation_against_packet(plan, &packet, &observation)?;
    Ok(arm)
}

fn validate_worker_packet(
    plan: &BehavioralTrialPlan,
    packet: &BehavioralWorkerPacket,
    raw_packet: &[u8],
) -> Result<BehavioralTrialArmKind, BehavioralTrialRunError> {
    let control =
        materialize_worker_packet(plan, BehavioralTrialArmKind::Control).map_err(source_error)?;
    let treatment =
        materialize_worker_packet(plan, BehavioralTrialArmKind::Treatment).map_err(source_error)?;
    let (arm, expected) = if packet == &control {
        (BehavioralTrialArmKind::Control, &control)
    } else if packet == &treatment {
        (BehavioralTrialArmKind::Treatment, &treatment)
    } else {
        return Err(BehavioralTrialRunError::new(
            "raw behavioral worker packet does not equal either frozen plan arm",
        ));
    };

    let mut expected_bytes = serde_json::to_vec_pretty(expected).map_err(|error| {
        BehavioralTrialRunError::new(format!(
            "failed to serialize the frozen behavioral worker packet: {error}"
        ))
    })?;
    expected_bytes.push(b'\n');
    if raw_packet != expected_bytes.as_slice() {
        return Err(BehavioralTrialRunError::new(
            "raw behavioral worker packet bytes do not match the exact materializer serialization",
        ));
    }
    Ok(arm)
}

fn validate_observation_against_packet(
    plan: &BehavioralTrialPlan,
    packet: &BehavioralWorkerPacket,
    observation: &BehavioralTrialObservation,
) -> Result<(), BehavioralTrialRunError> {
    let plan_fingerprint = fingerprint_plan(plan).map_err(source_error)?;
    if observation.schema_version != BEHAVIORAL_TRIAL_SCHEMA_VERSION {
        return Err(BehavioralTrialRunError::new(
            "behavioral worker output uses the wrong observation schema version",
        ));
    }
    if observation.trial_id != plan.trial_id
        || observation.trial_id != packet.trial_id
        || observation.plan_fingerprint != plan_fingerprint
        || observation.plan_fingerprint != packet.plan_fingerprint
    {
        return Err(BehavioralTrialRunError::new(
            "behavioral worker output does not bind to the exact frozen trial",
        ));
    }
    if observation.worker_packet_fingerprint != packet.worker_packet_fingerprint {
        return Err(BehavioralTrialRunError::new(
            "behavioral worker output does not bind to the exact worker packet",
        ));
    }
    validate_atom(&observation.worker_ref, "worker_ref", MAX_ID_BYTES)?;
    if !plan
        .allowed_first_actions
        .iter()
        .any(|action| action.id == observation.first_action_id)
    {
        return Err(BehavioralTrialRunError::new(format!(
            "behavioral worker output action `{}` is outside the frozen vocabulary",
            observation.first_action_id
        )));
    }
    Ok(())
}

fn validate_metadata(metadata: &BehavioralTrialRunMetadata) -> Result<(), BehavioralTrialRunError> {
    if metadata.schema_version != BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION {
        return Err(BehavioralTrialRunError::new(format!(
            "unsupported behavioral-trial run metadata schema {}; expected {BEHAVIORAL_TRIAL_RUN_SCHEMA_VERSION}",
            metadata.schema_version
        )));
    }
    if !(1..=2).contains(&metadata.sequence_index) {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial sequence_index must be 1 or 2",
        ));
    }
    validate_atom(&metadata.worker_identity, "worker_identity", MAX_ID_BYTES)?;
    validate_atom(&metadata.harness_identity, "harness_identity", MAX_ID_BYTES)?;
    validate_atom(
        &metadata.affordance_identity,
        "affordance_identity",
        MAX_ID_BYTES,
    )?;
    validate_sha256(&metadata.sampling_config_sha256, "sampling_config_sha256")?;
    validate_atom(&metadata.session_id, "session_id", MAX_ID_BYTES)?;
    validate_atom(
        &metadata.freshness_receipt,
        "freshness_receipt",
        MAX_RECEIPT_REF_BYTES,
    )?;
    Ok(())
}

fn require_fresh_uncontaminated_run(
    metadata: &BehavioralTrialRunMetadata,
) -> Result<(), BehavioralTrialRunError> {
    if !metadata.fresh_session {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial admitted pair requires fresh_session=true",
        ));
    }
    if metadata.prior_condition_exposure {
        return Err(BehavioralTrialRunError::new(
            "behavioral-trial admitted pair requires prior_condition_exposure=false",
        ));
    }
    Ok(())
}

fn require_same_pair_configuration(
    left: &BehavioralTrialRunReceipt,
    right: &BehavioralTrialRunReceipt,
) -> Result<(), BehavioralTrialRunError> {
    for (label, left_value, right_value) in [
        (
            "worker_identity",
            &left.metadata.worker_identity,
            &right.metadata.worker_identity,
        ),
        (
            "harness_identity",
            &left.metadata.harness_identity,
            &right.metadata.harness_identity,
        ),
        (
            "affordance_identity",
            &left.metadata.affordance_identity,
            &right.metadata.affordance_identity,
        ),
        (
            "sampling_config_sha256",
            &left.metadata.sampling_config_sha256,
            &right.metadata.sampling_config_sha256,
        ),
    ] {
        if left_value != right_value {
            return Err(BehavioralTrialRunError::new(format!(
                "behavioral-trial pair requires identical {label} across runs"
            )));
        }
    }
    Ok(())
}

fn sha256_receipt(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(SHA256_HEX_BYTES);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("sha256:{hex}")
}

fn require_sha256_match(
    supplied: &str,
    bytes: &[u8],
    field: &str,
) -> Result<(), BehavioralTrialRunError> {
    validate_sha256(supplied, field)?;
    if supplied != sha256_receipt(bytes) {
        return Err(BehavioralTrialRunError::new(format!(
            "{field} does not match the exact retained bytes"
        )));
    }
    Ok(())
}

fn validate_sha256(value: &str, field: &str) -> Result<(), BehavioralTrialRunError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(BehavioralTrialRunError::new(format!(
            "{field} must use sha256:<hex>"
        )));
    };
    if hex.len() != SHA256_HEX_BYTES
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(BehavioralTrialRunError::new(format!(
            "{field} must contain exactly 64 lowercase SHA-256 hex characters"
        )));
    }
    Ok(())
}

fn validate_atom(value: &str, field: &str, maximum: usize) -> Result<(), BehavioralTrialRunError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > maximum
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(BehavioralTrialRunError::new(format!(
            "{field} must be bounded non-empty single-line text"
        )));
    }
    Ok(())
}

fn enforce_bound(bytes: &[u8], maximum: usize, label: &str) -> Result<(), BehavioralTrialRunError> {
    if bytes.len() > maximum {
        return Err(BehavioralTrialRunError::new(format!(
            "{label} exceeds the {maximum}-byte limit"
        )));
    }
    Ok(())
}

fn source_error(error: impl fmt::Display) -> BehavioralTrialRunError {
    BehavioralTrialRunError::new(format!(
        "behavioral-trial source validation failed: {error}"
    ))
}

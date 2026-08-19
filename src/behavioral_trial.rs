use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const BEHAVIORAL_TRIAL_SCHEMA_VERSION: u32 = 1;
pub const MAX_BEHAVIORAL_TRIAL_BYTES: usize = 512 * 1024;
pub const CONTEXT_DIGEST_SCHEME: &str = "cultist-behavioral-context-sha256-v1";
pub const PLAN_FINGERPRINT_SCHEME: &str = "cultist-behavioral-trial-plan-sha256-v1";
pub const WORKER_PACKET_FINGERPRINT_SCHEME: &str = "cultist-behavioral-worker-packet-sha256-v1";
const MAX_TRIAL_ID_BYTES: usize = 1024;
const MAX_TASK_BYTES: usize = 16 * 1024;
const MAX_CONTEXT_BYTES: usize = 128 * 1024;
const MAX_CONTEXT_REF_BYTES: usize = 2048;
const MAX_ACTIONS: usize = 32;
const MAX_ACTION_ID_BYTES: usize = 256;
const MAX_ACTION_LABEL_BYTES: usize = 1024;
const MAX_WORKER_REF_BYTES: usize = 1024;
const SHA256_HEX_BYTES: usize = 64;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialPlan {
    pub schema_version: u32,
    pub trial_id: String,
    pub task_instruction: String,
    pub allowed_first_actions: Vec<BehavioralTrialAction>,
    pub control: BehavioralTrialArm,
    pub treatment: BehavioralTrialArm,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialAction {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialArm {
    pub context_ref: String,
    pub context: String,
    pub context_digest: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralTrialArmKind {
    Control,
    Treatment,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralWorkerPacket {
    pub schema_version: u32,
    pub trial_id: String,
    pub plan_fingerprint: String,
    pub worker_packet_fingerprint: String,
    pub task_instruction: String,
    pub context: String,
    pub context_digest: String,
    pub allowed_first_actions: Vec<BehavioralTrialAction>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialObservation {
    pub schema_version: u32,
    pub trial_id: String,
    pub plan_fingerprint: String,
    pub worker_packet_fingerprint: String,
    pub worker_ref: String,
    pub first_action_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialPair {
    pub schema_version: u32,
    pub plan: Box<BehavioralTrialPlan>,
    pub observations: Vec<BehavioralTrialObservation>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialEvaluation {
    pub schema_version: u32,
    pub trial_id: String,
    pub plan_fingerprint: String,
    pub control: BehavioralTrialArmObservation,
    pub treatment: BehavioralTrialArmObservation,
    pub same_first_action: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralTrialArmObservation {
    pub worker_packet_fingerprint: String,
    pub worker_ref: String,
    pub first_action_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BehavioralTrialError {
    message: String,
}

impl BehavioralTrialError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BehavioralTrialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for BehavioralTrialError {}

pub fn parse_behavioral_trial_plan(
    bytes: &[u8],
) -> Result<BehavioralTrialPlan, BehavioralTrialError> {
    enforce_input_bound(bytes)?;
    let plan: BehavioralTrialPlan = serde_json::from_slice(bytes).map_err(|error| {
        BehavioralTrialError::new(format!("invalid behavioral-trial plan JSON: {error}"))
    })?;
    validate_plan(&plan)?;
    Ok(plan)
}

pub fn parse_behavioral_trial_pair(
    bytes: &[u8],
) -> Result<BehavioralTrialPair, BehavioralTrialError> {
    enforce_input_bound(bytes)?;
    let pair: BehavioralTrialPair = serde_json::from_slice(bytes).map_err(|error| {
        BehavioralTrialError::new(format!("invalid behavioral-trial pair JSON: {error}"))
    })?;
    validate_pair_shape(&pair)?;
    Ok(pair)
}

pub fn context_digest(context: &str) -> String {
    hash_single(CONTEXT_DIGEST_SCHEME, context.as_bytes())
}

pub fn fingerprint_plan(plan: &BehavioralTrialPlan) -> Result<String, BehavioralTrialError> {
    validate_plan(plan)?;
    let mut components = Vec::<Vec<u8>>::new();
    components.push(plan.schema_version.to_be_bytes().to_vec());
    components.push(plan.trial_id.as_bytes().to_vec());
    components.push(plan.task_instruction.as_bytes().to_vec());
    components.push(
        (plan.allowed_first_actions.len() as u64)
            .to_be_bytes()
            .to_vec(),
    );
    for action in &plan.allowed_first_actions {
        components.push(action.id.as_bytes().to_vec());
        components.push(action.label.as_bytes().to_vec());
    }
    push_arm_components(&mut components, &plan.control);
    push_arm_components(&mut components, &plan.treatment);
    Ok(hash_components(PLAN_FINGERPRINT_SCHEME, &components))
}

pub fn materialize_worker_packet(
    plan: &BehavioralTrialPlan,
    arm: BehavioralTrialArmKind,
) -> Result<BehavioralWorkerPacket, BehavioralTrialError> {
    validate_plan(plan)?;
    let plan_fingerprint = fingerprint_plan(plan)?;
    let selected = match arm {
        BehavioralTrialArmKind::Control => &plan.control,
        BehavioralTrialArmKind::Treatment => &plan.treatment,
    };
    let packet_fingerprint = fingerprint_worker_packet_material(
        plan,
        &plan_fingerprint,
        &selected.context,
        &selected.context_digest,
    );

    Ok(BehavioralWorkerPacket {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        trial_id: plan.trial_id.clone(),
        plan_fingerprint,
        worker_packet_fingerprint: packet_fingerprint,
        task_instruction: plan.task_instruction.clone(),
        context: selected.context.clone(),
        context_digest: selected.context_digest.clone(),
        allowed_first_actions: plan.allowed_first_actions.clone(),
    })
}

pub fn evaluate_behavioral_trial_pair(
    pair: &BehavioralTrialPair,
) -> Result<BehavioralTrialEvaluation, BehavioralTrialError> {
    validate_pair_shape(pair)?;
    let plan = pair.plan.as_ref();
    let plan_fingerprint = fingerprint_plan(plan)?;
    let control_packet = materialize_worker_packet(plan, BehavioralTrialArmKind::Control)?;
    let treatment_packet = materialize_worker_packet(plan, BehavioralTrialArmKind::Treatment)?;

    let mut mapped = BTreeMap::<BehavioralTrialArmKindKey, &BehavioralTrialObservation>::new();
    for observation in &pair.observations {
        validate_observation(observation, plan, &plan_fingerprint)?;
        let key =
            if observation.worker_packet_fingerprint == control_packet.worker_packet_fingerprint {
                BehavioralTrialArmKindKey::Control
            } else if observation.worker_packet_fingerprint
                == treatment_packet.worker_packet_fingerprint
            {
                BehavioralTrialArmKindKey::Treatment
            } else {
                return Err(BehavioralTrialError::new(format!(
                    "observation from worker `{}` names an unknown worker-packet fingerprint",
                    observation.worker_ref
                )));
            };
        if mapped.insert(key, observation).is_some() {
            return Err(BehavioralTrialError::new(
                "paired behavioral trial contains two observations for the same arm",
            ));
        }
    }

    let control = mapped
        .get(&BehavioralTrialArmKindKey::Control)
        .ok_or_else(|| {
            BehavioralTrialError::new("paired behavioral trial is missing the control observation")
        })?;
    let treatment = mapped
        .get(&BehavioralTrialArmKindKey::Treatment)
        .ok_or_else(|| {
            BehavioralTrialError::new(
                "paired behavioral trial is missing the treatment observation",
            )
        })?;

    Ok(BehavioralTrialEvaluation {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        trial_id: plan.trial_id.clone(),
        plan_fingerprint,
        control: arm_observation(control),
        treatment: arm_observation(treatment),
        same_first_action: control.first_action_id == treatment.first_action_id,
    })
}

fn validate_plan(plan: &BehavioralTrialPlan) -> Result<(), BehavioralTrialError> {
    if plan.schema_version != BEHAVIORAL_TRIAL_SCHEMA_VERSION {
        return Err(BehavioralTrialError::new(format!(
            "unsupported behavioral-trial schema {}; expected {BEHAVIORAL_TRIAL_SCHEMA_VERSION}",
            plan.schema_version
        )));
    }
    validate_coordinate(&plan.trial_id, "trial_id", MAX_TRIAL_ID_BYTES)?;
    validate_text(&plan.task_instruction, "task_instruction", MAX_TASK_BYTES)?;
    if !(2..=MAX_ACTIONS).contains(&plan.allowed_first_actions.len()) {
        return Err(BehavioralTrialError::new(format!(
            "allowed_first_actions must contain 2..={MAX_ACTIONS} actions"
        )));
    }
    let mut action_ids = BTreeSet::<&str>::new();
    for action in &plan.allowed_first_actions {
        validate_token(&action.id, "action.id", MAX_ACTION_ID_BYTES)?;
        validate_text(&action.label, "action.label", MAX_ACTION_LABEL_BYTES)?;
        if !action_ids.insert(&action.id) {
            return Err(BehavioralTrialError::new(format!(
                "duplicate behavioral-trial action id `{}`",
                action.id
            )));
        }
    }
    validate_arm(&plan.control, "control")?;
    validate_arm(&plan.treatment, "treatment")?;
    if plan.control.context_digest == plan.treatment.context_digest {
        return Err(BehavioralTrialError::new(
            "control and treatment contexts must have different exact digests",
        ));
    }
    Ok(())
}

fn validate_arm(arm: &BehavioralTrialArm, label: &str) -> Result<(), BehavioralTrialError> {
    validate_coordinate(
        &arm.context_ref,
        &format!("{label}.context_ref"),
        MAX_CONTEXT_REF_BYTES,
    )?;
    validate_text(&arm.context, &format!("{label}.context"), MAX_CONTEXT_BYTES)?;
    validate_digest(
        &arm.context_digest,
        CONTEXT_DIGEST_SCHEME,
        &format!("{label}.context_digest"),
    )?;
    let expected = context_digest(&arm.context);
    if arm.context_digest != expected {
        return Err(BehavioralTrialError::new(format!(
            "{label}.context_digest does not match the exact context bytes"
        )));
    }
    Ok(())
}

fn validate_pair_shape(pair: &BehavioralTrialPair) -> Result<(), BehavioralTrialError> {
    if pair.schema_version != BEHAVIORAL_TRIAL_SCHEMA_VERSION {
        return Err(BehavioralTrialError::new(format!(
            "unsupported behavioral-trial pair schema {}; expected {BEHAVIORAL_TRIAL_SCHEMA_VERSION}",
            pair.schema_version
        )));
    }
    validate_plan(pair.plan.as_ref())?;
    if pair.observations.len() != 2 {
        return Err(BehavioralTrialError::new(
            "paired behavioral trial requires exactly two observations",
        ));
    }
    Ok(())
}

fn validate_observation(
    observation: &BehavioralTrialObservation,
    plan: &BehavioralTrialPlan,
    plan_fingerprint: &str,
) -> Result<(), BehavioralTrialError> {
    if observation.schema_version != BEHAVIORAL_TRIAL_SCHEMA_VERSION {
        return Err(BehavioralTrialError::new(format!(
            "unsupported behavioral-trial observation schema {}; expected {BEHAVIORAL_TRIAL_SCHEMA_VERSION}",
            observation.schema_version
        )));
    }
    if observation.trial_id != plan.trial_id {
        return Err(BehavioralTrialError::new(
            "observation trial_id does not match the registered plan",
        ));
    }
    validate_digest(
        &observation.plan_fingerprint,
        PLAN_FINGERPRINT_SCHEME,
        "observation.plan_fingerprint",
    )?;
    if observation.plan_fingerprint != plan_fingerprint {
        return Err(BehavioralTrialError::new(
            "observation plan_fingerprint does not match the current registered plan",
        ));
    }
    validate_digest(
        &observation.worker_packet_fingerprint,
        WORKER_PACKET_FINGERPRINT_SCHEME,
        "observation.worker_packet_fingerprint",
    )?;
    validate_coordinate(
        &observation.worker_ref,
        "observation.worker_ref",
        MAX_WORKER_REF_BYTES,
    )?;
    validate_token(
        &observation.first_action_id,
        "observation.first_action_id",
        MAX_ACTION_ID_BYTES,
    )?;
    if !plan
        .allowed_first_actions
        .iter()
        .any(|action| action.id == observation.first_action_id)
    {
        return Err(BehavioralTrialError::new(format!(
            "observation first_action_id `{}` is not in the registered action vocabulary",
            observation.first_action_id
        )));
    }
    Ok(())
}

fn arm_observation(observation: &BehavioralTrialObservation) -> BehavioralTrialArmObservation {
    BehavioralTrialArmObservation {
        worker_packet_fingerprint: observation.worker_packet_fingerprint.clone(),
        worker_ref: observation.worker_ref.clone(),
        first_action_id: observation.first_action_id.clone(),
    }
}

fn fingerprint_worker_packet_material(
    plan: &BehavioralTrialPlan,
    plan_fingerprint: &str,
    context: &str,
    context_digest: &str,
) -> String {
    let mut components = vec![
        plan.schema_version.to_be_bytes().to_vec(),
        plan.trial_id.as_bytes().to_vec(),
        plan_fingerprint.as_bytes().to_vec(),
        plan.task_instruction.as_bytes().to_vec(),
        context.as_bytes().to_vec(),
        context_digest.as_bytes().to_vec(),
        (plan.allowed_first_actions.len() as u64)
            .to_be_bytes()
            .to_vec(),
    ];
    for action in &plan.allowed_first_actions {
        components.push(action.id.as_bytes().to_vec());
        components.push(action.label.as_bytes().to_vec());
    }
    hash_components(WORKER_PACKET_FINGERPRINT_SCHEME, &components)
}

fn push_arm_components(components: &mut Vec<Vec<u8>>, arm: &BehavioralTrialArm) {
    components.push(arm.context_ref.as_bytes().to_vec());
    components.push(arm.context.as_bytes().to_vec());
    components.push(arm.context_digest.as_bytes().to_vec());
}

fn hash_single(scheme: &str, bytes: &[u8]) -> String {
    hash_components(scheme, &[bytes.to_vec()])
}

fn hash_components(scheme: &str, components: &[Vec<u8>]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scheme.as_bytes());
    hasher.update([0]);
    for component in components {
        hasher.update((component.len() as u64).to_be_bytes());
        hasher.update(component);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(SHA256_HEX_BYTES);
    for byte in digest {
        hex.push(hex_digit(byte >> 4));
        hex.push(hex_digit(byte & 0x0f));
    }
    format!("{scheme}:{hex}")
}

fn validate_digest(value: &str, scheme: &str, field: &str) -> Result<(), BehavioralTrialError> {
    let prefix = format!("{scheme}:");
    let Some(digest) = value.strip_prefix(&prefix) else {
        return Err(BehavioralTrialError::new(format!(
            "{field} uses an unsupported digest scheme"
        )));
    };
    if digest.len() != SHA256_HEX_BYTES
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(BehavioralTrialError::new(format!(
            "{field} must contain exactly 64 lowercase SHA-256 hex characters"
        )));
    }
    Ok(())
}

fn enforce_input_bound(bytes: &[u8]) -> Result<(), BehavioralTrialError> {
    if bytes.len() > MAX_BEHAVIORAL_TRIAL_BYTES {
        return Err(BehavioralTrialError::new(format!(
            "behavioral-trial input exceeds the {MAX_BEHAVIORAL_TRIAL_BYTES}-byte limit"
        )));
    }
    Ok(())
}

fn validate_coordinate(
    value: &str,
    field: &str,
    maximum: usize,
) -> Result<(), BehavioralTrialError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > maximum
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(BehavioralTrialError::new(format!(
            "{field} must be a bounded non-empty canonical single-line coordinate"
        )));
    }
    Ok(())
}

fn validate_token(value: &str, field: &str, maximum: usize) -> Result<(), BehavioralTrialError> {
    validate_coordinate(value, field, maximum)?;
    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/')
    }) {
        return Err(BehavioralTrialError::new(format!(
            "{field} must use lowercase ASCII token characters"
        )));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str, maximum: usize) -> Result<(), BehavioralTrialError> {
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(BehavioralTrialError::new(format!(
            "{field} must be bounded non-empty text"
        )));
    }
    Ok(())
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => unreachable!("nibble is masked to four bits"),
    }
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum BehavioralTrialArmKindKey {
    Control,
    Treatment,
}

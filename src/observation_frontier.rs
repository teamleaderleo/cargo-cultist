use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::discriminator_observation::{
    DiscriminatorObservation, DiscriminatorObservationBatch, ObservationCurrentness,
    classify_observation_currentness, validate_discriminator_observation_batch,
};

pub const OBSERVATION_FRONTIER_SCHEMA_VERSION: u32 = 2;
pub const MAX_OBSERVATION_FRONTIER_REQUEST_BYTES: usize = 512 * 1024;
const MAX_REQUIREMENTS: usize = 256;
const MAX_ID_BYTES: usize = 512;
const MAX_REFERENCE_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrontierRequest {
    pub schema_version: u32,
    pub requirements: Vec<ObservationRequirement>,
    pub observations: DiscriminatorObservationBatch,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRequirement {
    pub discriminator_id: String,
    pub subject_ref: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationFrontierStatus {
    Current,
    Unknown,
    Invalid,
    Missing,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrontierEvaluation {
    pub schema_version: u32,
    pub frontiers: Vec<ObservationFrontierReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrontierReceipt {
    pub discriminator_id: String,
    pub subject_ref: String,
    pub status: ObservationFrontierStatus,
    pub current: Vec<CurrentObservationReceipt>,
    pub unknown: Vec<NonCurrentObservationReceipt>,
    pub invalid: Vec<NonCurrentObservationReceipt>,
    pub other_subject: Vec<OtherSubjectObservationReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentObservationReceipt {
    pub observation_id: String,
    pub source_receipt: String,
    pub value_ref: String,
    pub applicability_ref: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonCurrentObservationReceipt {
    pub observation_id: String,
    pub source_receipt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_value_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_unknown_reason_ref: Option<String>,
    pub applicability_ref: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationCurrentnessKind {
    Current,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OtherSubjectObservationReceipt {
    pub observation_id: String,
    pub subject_ref: String,
    pub source_receipt: String,
    pub state: ObservationCurrentnessKind,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObservationFrontierError {
    message: String,
}

impl ObservationFrontierError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ObservationFrontierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ObservationFrontierError {}

pub fn parse_observation_frontier_request(
    bytes: &[u8],
) -> Result<ObservationFrontierRequest, ObservationFrontierError> {
    if bytes.len() > MAX_OBSERVATION_FRONTIER_REQUEST_BYTES {
        return Err(ObservationFrontierError::new(format!(
            "observation frontier request exceeds the {MAX_OBSERVATION_FRONTIER_REQUEST_BYTES}-byte limit"
        )));
    }
    let request: ObservationFrontierRequest = serde_json::from_slice(bytes).map_err(|error| {
        ObservationFrontierError::new(format!("invalid observation frontier JSON: {error}"))
    })?;
    validate_request(&request)?;
    Ok(request)
}

pub fn evaluate_observation_frontiers(
    request: &ObservationFrontierRequest,
) -> Result<ObservationFrontierEvaluation, ObservationFrontierError> {
    validate_request(request)?;

    let mut frontiers = request
        .requirements
        .iter()
        .map(|requirement| evaluate_requirement(requirement, &request.observations.observations))
        .collect::<Vec<_>>();
    frontiers.sort_by(|left, right| {
        left.discriminator_id
            .cmp(&right.discriminator_id)
            .then_with(|| left.subject_ref.cmp(&right.subject_ref))
    });

    Ok(ObservationFrontierEvaluation {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        frontiers,
    })
}

fn evaluate_requirement(
    requirement: &ObservationRequirement,
    observations: &[DiscriminatorObservation],
) -> ObservationFrontierReceipt {
    let mut current = Vec::new();
    let mut unknown = Vec::new();
    let mut invalid = Vec::new();
    let mut other_subject = Vec::new();

    for observation in observations
        .iter()
        .filter(|observation| observation.discriminator_id == requirement.discriminator_id)
    {
        let currentness = classify_observation_currentness(observation);
        if observation.subject_ref != requirement.subject_ref {
            other_subject.push(OtherSubjectObservationReceipt {
                observation_id: observation.observation_id.clone(),
                subject_ref: observation.subject_ref.clone(),
                source_receipt: observation.source_receipt.clone(),
                state: currentness_kind(currentness),
            });
            continue;
        }

        match currentness {
            ObservationCurrentness::Current {
                value_ref,
                applicability_ref,
            } => current.push(CurrentObservationReceipt {
                observation_id: observation.observation_id.clone(),
                source_receipt: observation.source_receipt.clone(),
                value_ref: value_ref.to_string(),
                applicability_ref: applicability_ref.to_string(),
            }),
            ObservationCurrentness::Unknown {
                known_value_ref,
                value_unknown_reason_ref,
                applicability_ref,
            } => unknown.push(non_current_receipt(
                observation,
                known_value_ref,
                value_unknown_reason_ref,
                applicability_ref,
            )),
            ObservationCurrentness::Invalid {
                known_value_ref,
                value_unknown_reason_ref,
                applicability_ref,
            } => invalid.push(non_current_receipt(
                observation,
                known_value_ref,
                value_unknown_reason_ref,
                applicability_ref,
            )),
        }
    }

    current.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    unknown.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    invalid.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    other_subject.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));

    let status = if !current.is_empty() {
        ObservationFrontierStatus::Current
    } else if !unknown.is_empty() {
        ObservationFrontierStatus::Unknown
    } else if !invalid.is_empty() {
        ObservationFrontierStatus::Invalid
    } else {
        ObservationFrontierStatus::Missing
    };

    ObservationFrontierReceipt {
        discriminator_id: requirement.discriminator_id.clone(),
        subject_ref: requirement.subject_ref.clone(),
        status,
        current,
        unknown,
        invalid,
        other_subject,
    }
}

fn currentness_kind(currentness: ObservationCurrentness<'_>) -> ObservationCurrentnessKind {
    match currentness {
        ObservationCurrentness::Current { .. } => ObservationCurrentnessKind::Current,
        ObservationCurrentness::Unknown { .. } => ObservationCurrentnessKind::Unknown,
        ObservationCurrentness::Invalid { .. } => ObservationCurrentnessKind::Invalid,
    }
}

fn non_current_receipt(
    observation: &DiscriminatorObservation,
    known_value_ref: Option<&str>,
    value_unknown_reason_ref: Option<&str>,
    applicability_ref: &str,
) -> NonCurrentObservationReceipt {
    NonCurrentObservationReceipt {
        observation_id: observation.observation_id.clone(),
        source_receipt: observation.source_receipt.clone(),
        known_value_ref: known_value_ref.map(str::to_string),
        value_unknown_reason_ref: value_unknown_reason_ref.map(str::to_string),
        applicability_ref: applicability_ref.to_string(),
    }
}

fn validate_request(request: &ObservationFrontierRequest) -> Result<(), ObservationFrontierError> {
    if request.schema_version != OBSERVATION_FRONTIER_SCHEMA_VERSION {
        return Err(ObservationFrontierError::new(format!(
            "unsupported observation frontier schema {}; expected {OBSERVATION_FRONTIER_SCHEMA_VERSION}",
            request.schema_version
        )));
    }
    if request.requirements.is_empty() || request.requirements.len() > MAX_REQUIREMENTS {
        return Err(ObservationFrontierError::new(
            "observation frontier requirements must be bounded and non-empty",
        ));
    }
    validate_discriminator_observation_batch(&request.observations).map_err(|error| {
        ObservationFrontierError::new(format!("observation batch validation failed: {error}"))
    })?;

    let mut seen = BTreeSet::new();
    for requirement in &request.requirements {
        validate_atom(
            &requirement.discriminator_id,
            "requirement discriminator_id",
            MAX_ID_BYTES,
        )?;
        validate_atom(
            &requirement.subject_ref,
            "requirement subject_ref",
            MAX_REFERENCE_BYTES,
        )?;
        if !seen.insert(requirement.clone()) {
            return Err(ObservationFrontierError::new(format!(
                "duplicate observation requirement {} @ {}",
                requirement.discriminator_id, requirement.subject_ref
            )));
        }
    }
    Ok(())
}

fn validate_atom(
    value: &str,
    field: &str,
    max_bytes: usize,
) -> Result<(), ObservationFrontierError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > max_bytes
        || value.contains('\0')
        || value.contains(['\n', '\r'])
    {
        return Err(ObservationFrontierError::new(format!(
            "{field} must be bounded canonical single-line text"
        )));
    }
    Ok(())
}

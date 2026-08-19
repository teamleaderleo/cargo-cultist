use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES: usize = 256 * 1024;
const MAX_OBSERVATIONS: usize = 1024;
const MAX_ID_BYTES: usize = 512;
const MAX_REFERENCE_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscriminatorObservationBatch {
    pub schema_version: u32,
    pub observations: Vec<DiscriminatorObservation>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscriminatorObservation {
    pub observation_id: String,
    pub discriminator_id: String,
    pub subject_ref: String,
    pub source_receipt: String,
    pub value_state: DiscriminatorValueState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability_ref: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiscriminatorValueState {
    Known { value_ref: String },
    Unknown { reason_ref: String },
    Invalid { reason_ref: String },
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscriminatorEnumeration {
    pub schema_version: u32,
    pub discriminators: Vec<EnumeratedDiscriminator>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnumeratedDiscriminator {
    pub discriminator_id: String,
    pub known_partitions: Vec<KnownPartition>,
    pub unknown: Vec<NonCurrentObservationReceipt>,
    pub invalid: Vec<NonCurrentObservationReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnownPartition {
    pub value_ref: String,
    pub observations: Vec<CurrentObservationReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentObservationReceipt {
    pub observation_id: String,
    pub subject_ref: String,
    pub source_receipt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability_ref: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonCurrentObservationReceipt {
    pub observation_id: String,
    pub subject_ref: String,
    pub source_receipt: String,
    pub reason_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability_ref: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DiscriminatorObservationError {
    message: String,
}

impl DiscriminatorObservationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DiscriminatorObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for DiscriminatorObservationError {}

pub fn parse_discriminator_observation_batch(
    bytes: &[u8],
) -> Result<DiscriminatorObservationBatch, DiscriminatorObservationError> {
    if bytes.len() > MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES {
        return Err(DiscriminatorObservationError::new(format!(
            "discriminator observation batch exceeds the {MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES}-byte limit"
        )));
    }
    let batch: DiscriminatorObservationBatch = serde_json::from_slice(bytes).map_err(|error| {
        DiscriminatorObservationError::new(format!(
            "invalid discriminator observation JSON: {error}"
        ))
    })?;
    validate_discriminator_observation_batch(&batch)?;
    Ok(batch)
}

pub fn validate_discriminator_observation_batch(
    batch: &DiscriminatorObservationBatch,
) -> Result<(), DiscriminatorObservationError> {
    if batch.schema_version != DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION {
        return Err(DiscriminatorObservationError::new(format!(
            "unsupported discriminator observation schema {}; expected {DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION}",
            batch.schema_version
        )));
    }
    if batch.observations.is_empty() || batch.observations.len() > MAX_OBSERVATIONS {
        return Err(DiscriminatorObservationError::new(
            "discriminator observation batch must contain a bounded non-empty observation set",
        ));
    }

    let mut seen = BTreeMap::<String, DiscriminatorObservation>::new();
    for observation in &batch.observations {
        validate_observation(observation)?;
        if let Some(existing) = seen.get(&observation.observation_id) {
            let message = if existing == observation {
                format!(
                    "duplicate discriminator observation id {}",
                    observation.observation_id
                )
            } else {
                format!(
                    "conflicting discriminator observation id {}",
                    observation.observation_id
                )
            };
            return Err(DiscriminatorObservationError::new(message));
        }
        seen.insert(observation.observation_id.clone(), observation.clone());
    }
    Ok(())
}

pub fn enumerate_discriminator_partitions(
    batch: &DiscriminatorObservationBatch,
) -> Result<DiscriminatorEnumeration, DiscriminatorObservationError> {
    validate_discriminator_observation_batch(batch)?;

    #[derive(Default)]
    struct Builder {
        known: BTreeMap<String, Vec<CurrentObservationReceipt>>,
        unknown: Vec<NonCurrentObservationReceipt>,
        invalid: Vec<NonCurrentObservationReceipt>,
    }

    let mut builders = BTreeMap::<String, Builder>::new();
    for observation in &batch.observations {
        let builder = builders
            .entry(observation.discriminator_id.clone())
            .or_default();
        match &observation.value_state {
            DiscriminatorValueState::Known { value_ref } => {
                builder
                    .known
                    .entry(value_ref.clone())
                    .or_default()
                    .push(current_receipt(observation));
            }
            DiscriminatorValueState::Unknown { reason_ref } => {
                builder
                    .unknown
                    .push(non_current_receipt(observation, reason_ref));
            }
            DiscriminatorValueState::Invalid { reason_ref } => {
                builder
                    .invalid
                    .push(non_current_receipt(observation, reason_ref));
            }
        }
    }

    let discriminators = builders
        .into_iter()
        .map(|(discriminator_id, mut builder)| {
            let known_partitions = builder
                .known
                .into_iter()
                .map(|(value_ref, mut observations)| {
                    observations
                        .sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
                    KnownPartition {
                        value_ref,
                        observations,
                    }
                })
                .collect();
            builder
                .unknown
                .sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
            builder
                .invalid
                .sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
            EnumeratedDiscriminator {
                discriminator_id,
                known_partitions,
                unknown: builder.unknown,
                invalid: builder.invalid,
            }
        })
        .collect();

    Ok(DiscriminatorEnumeration {
        schema_version: DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
        discriminators,
    })
}

fn current_receipt(observation: &DiscriminatorObservation) -> CurrentObservationReceipt {
    CurrentObservationReceipt {
        observation_id: observation.observation_id.clone(),
        subject_ref: observation.subject_ref.clone(),
        source_receipt: observation.source_receipt.clone(),
        applicability_ref: observation.applicability_ref.clone(),
    }
}

fn non_current_receipt(
    observation: &DiscriminatorObservation,
    reason_ref: &str,
) -> NonCurrentObservationReceipt {
    NonCurrentObservationReceipt {
        observation_id: observation.observation_id.clone(),
        subject_ref: observation.subject_ref.clone(),
        source_receipt: observation.source_receipt.clone(),
        reason_ref: reason_ref.to_string(),
        applicability_ref: observation.applicability_ref.clone(),
    }
}

fn validate_observation(
    observation: &DiscriminatorObservation,
) -> Result<(), DiscriminatorObservationError> {
    validate_atom(&observation.observation_id, "observation_id", MAX_ID_BYTES)?;
    validate_atom(
        &observation.discriminator_id,
        "discriminator_id",
        MAX_ID_BYTES,
    )?;
    validate_atom(&observation.subject_ref, "subject_ref", MAX_REFERENCE_BYTES)?;
    validate_atom(
        &observation.source_receipt,
        "source_receipt",
        MAX_REFERENCE_BYTES,
    )?;
    if let Some(applicability_ref) = &observation.applicability_ref {
        validate_atom(applicability_ref, "applicability_ref", MAX_REFERENCE_BYTES)?;
    }
    match &observation.value_state {
        DiscriminatorValueState::Known { value_ref } => {
            validate_atom(value_ref, "value_ref", MAX_REFERENCE_BYTES)
        }
        DiscriminatorValueState::Unknown { reason_ref }
        | DiscriminatorValueState::Invalid { reason_ref } => {
            validate_atom(reason_ref, "reason_ref", MAX_REFERENCE_BYTES)
        }
    }
}

fn validate_atom(
    value: &str,
    field: &str,
    max_bytes: usize,
) -> Result<(), DiscriminatorObservationError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > max_bytes
        || value.contains('\0')
        || value.contains(['\n', '\r'])
    {
        return Err(DiscriminatorObservationError::new(format!(
            "{field} must be bounded canonical single-line text"
        )));
    }
    Ok(())
}

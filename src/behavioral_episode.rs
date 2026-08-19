use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::behavioral_receipt::{BehavioralReceipt, validate_receipt};

pub const BEHAVIORAL_EPISODE_SCHEMA_VERSION: u32 = 1;
pub const MAX_BEHAVIORAL_EPISODE_BATCH_BYTES: usize = 256 * 1024;
const MAX_EPISODES: usize = 512;
const MAX_EPISODE_ID_BYTES: usize = 1024;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralEpisode {
    pub episode_id: String,
    pub receipt: BehavioralReceipt,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehavioralEpisodeBatch {
    pub schema_version: u32,
    pub episodes: Vec<BehavioralEpisode>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BehavioralEpisodeError {
    message: String,
}

impl BehavioralEpisodeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BehavioralEpisodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for BehavioralEpisodeError {}

pub fn parse_behavioral_episode_batch(
    bytes: &[u8],
) -> Result<BehavioralEpisodeBatch, BehavioralEpisodeError> {
    if bytes.len() > MAX_BEHAVIORAL_EPISODE_BATCH_BYTES {
        return Err(BehavioralEpisodeError::new(format!(
            "behavioral episode batch exceeds the {MAX_BEHAVIORAL_EPISODE_BATCH_BYTES}-byte limit"
        )));
    }

    let batch: BehavioralEpisodeBatch = serde_json::from_slice(bytes).map_err(|error| {
        BehavioralEpisodeError::new(format!("invalid behavioral episode JSON: {error}"))
    })?;
    validate_behavioral_episode_batch(&batch)?;
    Ok(batch)
}

pub fn validate_behavioral_episode_batch(
    batch: &BehavioralEpisodeBatch,
) -> Result<(), BehavioralEpisodeError> {
    if batch.schema_version != BEHAVIORAL_EPISODE_SCHEMA_VERSION {
        return Err(BehavioralEpisodeError::new(format!(
            "unsupported behavioral episode schema {}; expected {BEHAVIORAL_EPISODE_SCHEMA_VERSION}",
            batch.schema_version
        )));
    }

    if batch.episodes.is_empty() || batch.episodes.len() > MAX_EPISODES {
        return Err(BehavioralEpisodeError::new(format!(
            "behavioral episode batch must contain 1..={MAX_EPISODES} episodes"
        )));
    }

    let mut by_id = BTreeMap::<&str, &BehavioralReceipt>::new();
    for episode in &batch.episodes {
        validate_episode_id(&episode.episode_id)?;
        validate_receipt(&episode.receipt).map_err(|error| {
            BehavioralEpisodeError::new(format!(
                "episode `{}` contains an invalid behavioral receipt: {error}",
                episode.episode_id
            ))
        })?;

        if let Some(existing) = by_id.insert(&episode.episode_id, &episode.receipt) {
            let kind = if existing == &episode.receipt {
                "duplicate"
            } else {
                "conflicting duplicate"
            };
            return Err(BehavioralEpisodeError::new(format!(
                "{kind} behavioral episode_id `{}`",
                episode.episode_id
            )));
        }
    }

    Ok(())
}

fn validate_episode_id(value: &str) -> Result<(), BehavioralEpisodeError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_EPISODE_ID_BYTES
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(BehavioralEpisodeError::new(
            "episode_id must be a bounded non-empty canonical observation identity",
        ));
    }
    Ok(())
}

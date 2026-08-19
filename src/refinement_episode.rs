use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const REFINEMENT_EPISODE_SCHEMA_VERSION: u32 = 1;
pub const MAX_REFINEMENT_EPISODE_BATCH_BYTES: usize = 256 * 1024;
const MAX_EPISODES: usize = 128;
const MAX_CANDIDATES: usize = 64;
const MAX_REFERENCES: usize = 256;
const MAX_ID_BYTES: usize = 512;
const MAX_TEXT_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementEpisodeBatch {
    pub schema_version: u32,
    pub episodes: Vec<RefinementEpisode>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementEpisode {
    pub id: String,
    pub family: String,
    pub hypothesis_before: ResearchHypothesis,
    pub counterexample_refs: Vec<String>,
    pub admitted_discriminators: Vec<DiscriminatorRef>,
    pub candidate_refinements: Vec<CandidateRefinement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_transition: Option<String>,
    pub source_receipts: Vec<String>,
    #[serde(default)]
    pub behavioral_episode_ids: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchHypothesis {
    pub id: String,
    pub statement: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscriminatorRef {
    pub id: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateRefinement {
    pub id: String,
    pub hypothesis_after: ResearchHypothesis,
    pub discriminator_refs: Vec<String>,
    pub replay_result: ReplayResult,
    pub status: RefinementStatus,
    pub source_receipts: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayResult {
    pub expected_cases_retained: usize,
    pub counterexamples_resolved: usize,
    pub expected_cases_lost: usize,
    pub counterexamples_remaining: usize,
    pub held_out_status: HeldOutStatus,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HeldOutStatus {
    NotRun,
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefinementStatus {
    Retained,
    Weakened,
    Split,
    RejectedNoImprovement,
    RejectedOverfit,
    RejectedLostExpectedCase,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefinementEpisodeError {
    message: String,
}

impl RefinementEpisodeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RefinementEpisodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RefinementEpisodeError {}

pub fn parse_refinement_episode_batch(
    bytes: &[u8],
) -> Result<RefinementEpisodeBatch, RefinementEpisodeError> {
    if bytes.len() > MAX_REFINEMENT_EPISODE_BATCH_BYTES {
        return Err(RefinementEpisodeError::new(format!(
            "refinement episode batch exceeds the {MAX_REFINEMENT_EPISODE_BATCH_BYTES}-byte limit"
        )));
    }
    let batch: RefinementEpisodeBatch = serde_json::from_slice(bytes).map_err(|error| {
        RefinementEpisodeError::new(format!("invalid refinement episode JSON: {error}"))
    })?;
    validate_refinement_episode_batch(&batch)?;
    Ok(batch)
}

pub fn validate_refinement_episode_batch(
    batch: &RefinementEpisodeBatch,
) -> Result<(), RefinementEpisodeError> {
    if batch.schema_version != REFINEMENT_EPISODE_SCHEMA_VERSION {
        return Err(RefinementEpisodeError::new(format!(
            "unsupported refinement episode schema {}; expected {REFINEMENT_EPISODE_SCHEMA_VERSION}",
            batch.schema_version
        )));
    }
    if batch.episodes.is_empty() || batch.episodes.len() > MAX_EPISODES {
        return Err(RefinementEpisodeError::new(
            "refinement episode batch must contain a bounded non-empty episode set",
        ));
    }

    let mut episode_ids = BTreeSet::new();
    for episode in &batch.episodes {
        validate_episode(episode)?;
        if !episode_ids.insert(episode.id.clone()) {
            return Err(RefinementEpisodeError::new(format!(
                "duplicate refinement episode id {}",
                episode.id
            )));
        }
    }
    Ok(())
}

fn validate_episode(episode: &RefinementEpisode) -> Result<(), RefinementEpisodeError> {
    validate_atom(&episode.id, "episode id", MAX_ID_BYTES)?;
    validate_atom(&episode.family, "episode family", MAX_ID_BYTES)?;
    validate_hypothesis(&episode.hypothesis_before, "hypothesis_before")?;
    validate_reference_set(&episode.counterexample_refs, "counterexample_refs", false)?;
    validate_reference_set(&episode.source_receipts, "source_receipts", false)?;
    validate_reference_set(
        &episode.behavioral_episode_ids,
        "behavioral_episode_ids",
        true,
    )?;

    if episode.admitted_discriminators.is_empty()
        || episode.admitted_discriminators.len() > MAX_REFERENCES
    {
        return Err(RefinementEpisodeError::new(
            "admitted_discriminators must be bounded and non-empty",
        ));
    }
    let mut discriminator_ids = BTreeSet::new();
    for discriminator in &episode.admitted_discriminators {
        validate_atom(&discriminator.id, "discriminator id", MAX_ID_BYTES)?;
        validate_atom(
            &discriminator.source_ref,
            "discriminator source_ref",
            MAX_TEXT_BYTES,
        )?;
        if !discriminator_ids.insert(discriminator.id.clone()) {
            return Err(RefinementEpisodeError::new(format!(
                "duplicate admitted discriminator {}",
                discriminator.id
            )));
        }
    }

    if episode.candidate_refinements.is_empty()
        || episode.candidate_refinements.len() > MAX_CANDIDATES
    {
        return Err(RefinementEpisodeError::new(
            "candidate_refinements must be bounded and non-empty",
        ));
    }
    let mut candidate_ids = BTreeSet::new();
    for candidate in &episode.candidate_refinements {
        validate_candidate(candidate, &discriminator_ids)?;
        if !candidate_ids.insert(candidate.id.clone()) {
            return Err(RefinementEpisodeError::new(format!(
                "duplicate candidate refinement id {}",
                candidate.id
            )));
        }
    }

    if let Some(selected) = &episode.selected_transition {
        validate_atom(selected, "selected_transition", MAX_ID_BYTES)?;
        let candidate = episode
            .candidate_refinements
            .iter()
            .find(|candidate| candidate.id == *selected)
            .ok_or_else(|| {
                RefinementEpisodeError::new(format!(
                    "selected transition {selected} is absent from candidate_refinements"
                ))
            })?;
        if !matches!(
            candidate.status,
            RefinementStatus::Retained | RefinementStatus::Weakened | RefinementStatus::Split
        ) {
            return Err(RefinementEpisodeError::new(format!(
                "selected transition {selected} has rejected status {:?}",
                candidate.status
            )));
        }
    }

    Ok(())
}

fn validate_candidate(
    candidate: &CandidateRefinement,
    admitted_discriminators: &BTreeSet<String>,
) -> Result<(), RefinementEpisodeError> {
    validate_atom(&candidate.id, "candidate id", MAX_ID_BYTES)?;
    validate_hypothesis(&candidate.hypothesis_after, "hypothesis_after")?;
    validate_reference_set(
        &candidate.discriminator_refs,
        "candidate discriminator_refs",
        false,
    )?;
    validate_reference_set(
        &candidate.source_receipts,
        "candidate source_receipts",
        false,
    )?;

    for discriminator in &candidate.discriminator_refs {
        if !admitted_discriminators.contains(discriminator) {
            return Err(RefinementEpisodeError::new(format!(
                "candidate {} references unadmitted discriminator {discriminator}",
                candidate.id
            )));
        }
    }

    match candidate.status {
        RefinementStatus::Retained | RefinementStatus::Weakened | RefinementStatus::Split => {
            if candidate.replay_result.expected_cases_lost != 0 {
                return Err(RefinementEpisodeError::new(format!(
                    "kept candidate {} loses expected replay cases",
                    candidate.id
                )));
            }
            if candidate.replay_result.counterexamples_resolved == 0 {
                return Err(RefinementEpisodeError::new(format!(
                    "kept candidate {} resolves no counterexample",
                    candidate.id
                )));
            }
            if candidate.replay_result.held_out_status == HeldOutStatus::Failed {
                return Err(RefinementEpisodeError::new(format!(
                    "kept candidate {} has failed held-out replay",
                    candidate.id
                )));
            }
        }
        RefinementStatus::RejectedNoImprovement => {
            if candidate.replay_result.counterexamples_resolved != 0
                || candidate.replay_result.expected_cases_lost != 0
            {
                return Err(RefinementEpisodeError::new(format!(
                    "rejected_no_improvement candidate {} must preserve the baseline replay",
                    candidate.id
                )));
            }
        }
        RefinementStatus::RejectedLostExpectedCase => {
            if candidate.replay_result.expected_cases_lost == 0 {
                return Err(RefinementEpisodeError::new(format!(
                    "rejected_lost_expected_case candidate {} must lose at least one expected case",
                    candidate.id
                )));
            }
        }
        RefinementStatus::RejectedOverfit => {}
    }

    Ok(())
}

fn validate_hypothesis(
    hypothesis: &ResearchHypothesis,
    field: &str,
) -> Result<(), RefinementEpisodeError> {
    validate_atom(&hypothesis.id, &format!("{field} id"), MAX_ID_BYTES)?;
    validate_text(
        &hypothesis.statement,
        &format!("{field} statement"),
        MAX_TEXT_BYTES,
    )
}

fn validate_reference_set(
    values: &[String],
    field: &str,
    allow_empty: bool,
) -> Result<(), RefinementEpisodeError> {
    if (!allow_empty && values.is_empty()) || values.len() > MAX_REFERENCES {
        return Err(RefinementEpisodeError::new(format!(
            "{field} must be within the admitted reference bound"
        )));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        validate_atom(value, field, MAX_TEXT_BYTES)?;
        if !seen.insert(value.clone()) {
            return Err(RefinementEpisodeError::new(format!(
                "{field} contains duplicate reference {value}"
            )));
        }
    }
    Ok(())
}

fn validate_atom(value: &str, field: &str, max_bytes: usize) -> Result<(), RefinementEpisodeError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > max_bytes
        || value.contains('\0')
        || value.contains(['\n', '\r'])
    {
        return Err(RefinementEpisodeError::new(format!(
            "{field} must be bounded canonical single-line text"
        )));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str, max_bytes: usize) -> Result<(), RefinementEpisodeError> {
    if value.is_empty() || value.trim() != value || value.len() > max_bytes || value.contains('\0')
    {
        return Err(RefinementEpisodeError::new(format!(
            "{field} must be bounded non-empty text"
        )));
    }
    Ok(())
}

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::closure_episode::{
    ClosureEpisodeDisposition, IssueClosureEpisode, IssueClosureEvaluation,
    evaluate_closure_episode,
};
use crate::review_memory::{
    ReviewMemoryEvaluation, ReviewMemoryQuery, ReviewThreadDisposition, evaluate_review_memory,
};

pub const PRIOR_EPISODE_FRONT_SCHEMA_VERSION: u32 = 1;
pub const MAX_PRIOR_EPISODE_FRONT_QUERY_BYTES: usize = 512 * 1024;
const MAX_INPUTS: usize = 64;
const MAX_ID_BYTES: usize = 1024;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PriorEpisodeFrontQuery {
    pub schema_version: u32,
    #[serde(default)]
    pub inputs: Vec<PriorEpisodeInput>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PriorEpisodeInput {
    ReviewMemory {
        id: String,
        query: ReviewMemoryQuery,
    },
    IssueClosure {
        id: String,
        episode: IssueClosureEpisode,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorEpisodeNextAction {
    ReuseExistingReviewThread,
    RecomputeAndRefreshReviewThread,
    AcquireMissingReviewCoordinate,
    InspectPriorFailureAndRereport,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PriorEpisodeFrontItem {
    Review {
        id: String,
        next: PriorEpisodeNextAction,
        evaluation: ReviewMemoryEvaluation,
    },
    IssueClosure {
        id: String,
        next: PriorEpisodeNextAction,
        evaluation: IssueClosureEvaluation,
        source_refs: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorEpisodeQuietReason {
    NoPriorReviewLineage,
    NoCurrentReviewLineage,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PriorEpisodeQuietReceipt {
    pub id: String,
    pub reason: PriorEpisodeQuietReason,
    pub evaluation: ReviewMemoryEvaluation,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PriorEpisodeFront {
    pub schema_version: u32,
    pub items: Vec<PriorEpisodeFrontItem>,
    pub quiet: Vec<PriorEpisodeQuietReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PriorEpisodeFrontError {
    message: String,
}

impl PriorEpisodeFrontError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PriorEpisodeFrontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PriorEpisodeFrontError {}

pub fn parse_prior_episode_front_query(
    bytes: &[u8],
) -> Result<PriorEpisodeFrontQuery, PriorEpisodeFrontError> {
    if bytes.len() > MAX_PRIOR_EPISODE_FRONT_QUERY_BYTES {
        return Err(PriorEpisodeFrontError::new(format!(
            "prior-episode-front query exceeds the {MAX_PRIOR_EPISODE_FRONT_QUERY_BYTES}-byte limit"
        )));
    }
    let query: PriorEpisodeFrontQuery = serde_json::from_slice(bytes).map_err(|error| {
        PriorEpisodeFrontError::new(format!("invalid prior-episode-front JSON: {error}"))
    })?;
    validate_query(&query)?;
    Ok(query)
}

pub fn evaluate_prior_episode_front(
    query: &PriorEpisodeFrontQuery,
) -> Result<PriorEpisodeFront, PriorEpisodeFrontError> {
    validate_query(query)?;

    let mut items = Vec::new();
    let mut quiet = Vec::new();

    for input in &query.inputs {
        match input {
            PriorEpisodeInput::ReviewMemory { id, query } => {
                let evaluation = evaluate_review_memory(query).map_err(|error| {
                    PriorEpisodeFrontError::new(format!(
                        "prior-episode input `{id}` has invalid review-memory evidence: {error}"
                    ))
                })?;
                project_review(id, evaluation, &mut items, &mut quiet);
            }
            PriorEpisodeInput::IssueClosure { id, episode } => {
                let evaluation = evaluate_closure_episode(episode).map_err(|error| {
                    PriorEpisodeFrontError::new(format!(
                        "prior-episode input `{id}` has invalid issue-closure evidence: {error}"
                    ))
                })?;
                if evaluation.disposition != ClosureEpisodeDisposition::InspectPriorFailure {
                    return Err(PriorEpisodeFrontError::new(format!(
                        "prior-episode input `{id}` returned unsupported issue-closure disposition"
                    )));
                }
                items.push(PriorEpisodeFrontItem::IssueClosure {
                    id: id.clone(),
                    next: PriorEpisodeNextAction::InspectPriorFailureAndRereport,
                    evaluation,
                    source_refs: closure_source_refs(episode),
                });
            }
        }
    }

    Ok(PriorEpisodeFront {
        schema_version: PRIOR_EPISODE_FRONT_SCHEMA_VERSION,
        items,
        quiet,
    })
}

fn project_review(
    id: &str,
    evaluation: ReviewMemoryEvaluation,
    items: &mut Vec<PriorEpisodeFrontItem>,
    quiet: &mut Vec<PriorEpisodeQuietReceipt>,
) {
    let has_prior_lineage = !evaluation.matches.is_empty();

    match evaluation.disposition {
        ReviewThreadDisposition::ReuseCurrentThread => {
            items.push(PriorEpisodeFrontItem::Review {
                id: id.to_string(),
                next: PriorEpisodeNextAction::ReuseExistingReviewThread,
                evaluation,
            });
        }
        ReviewThreadDisposition::RefreshExistingThread => {
            items.push(PriorEpisodeFrontItem::Review {
                id: id.to_string(),
                next: PriorEpisodeNextAction::RecomputeAndRefreshReviewThread,
                evaluation,
            });
        }
        ReviewThreadDisposition::NeedContext if has_prior_lineage => {
            items.push(PriorEpisodeFrontItem::Review {
                id: id.to_string(),
                next: PriorEpisodeNextAction::AcquireMissingReviewCoordinate,
                evaluation,
            });
        }
        ReviewThreadDisposition::NeedContext => {
            quiet.push(PriorEpisodeQuietReceipt {
                id: id.to_string(),
                reason: PriorEpisodeQuietReason::NoPriorReviewLineage,
                evaluation,
            });
        }
        ReviewThreadDisposition::NewThread => {
            quiet.push(PriorEpisodeQuietReceipt {
                id: id.to_string(),
                reason: if has_prior_lineage {
                    PriorEpisodeQuietReason::NoCurrentReviewLineage
                } else {
                    PriorEpisodeQuietReason::NoPriorReviewLineage
                },
                evaluation,
            });
        }
    }
}

fn closure_source_refs(episode: &IssueClosureEpisode) -> Vec<String> {
    let mut refs = vec![
        episode.closure.source_ref.clone(),
        episode.re_report.source_ref.clone(),
    ];
    if let Some(challenge) = &episode.duplicate_challenge {
        refs.push(challenge.suggestion_source_ref.clone());
        refs.push(challenge.rejection_source_ref.clone());
    }
    refs
}

fn validate_query(query: &PriorEpisodeFrontQuery) -> Result<(), PriorEpisodeFrontError> {
    if query.schema_version != PRIOR_EPISODE_FRONT_SCHEMA_VERSION {
        return Err(PriorEpisodeFrontError::new(format!(
            "unsupported prior-episode-front schema {}; expected {PRIOR_EPISODE_FRONT_SCHEMA_VERSION}",
            query.schema_version
        )));
    }
    if query.inputs.len() > MAX_INPUTS {
        return Err(PriorEpisodeFrontError::new(format!(
            "prior-episode-front query may contain at most {MAX_INPUTS} inputs"
        )));
    }

    let mut by_id = BTreeMap::<&str, &PriorEpisodeInput>::new();
    for input in &query.inputs {
        let id = input.id();
        validate_id(id)?;
        if let Some(existing) = by_id.insert(id, input) {
            let kind = if existing == input {
                "duplicate"
            } else {
                "conflicting duplicate"
            };
            return Err(PriorEpisodeFrontError::new(format!(
                "{kind} prior-episode input id `{id}`"
            )));
        }
    }
    Ok(())
}

impl PriorEpisodeInput {
    fn id(&self) -> &str {
        match self {
            Self::ReviewMemory { id, .. } | Self::IssueClosure { id, .. } => id,
        }
    }
}

fn validate_id(value: &str) -> Result<(), PriorEpisodeFrontError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_ID_BYTES
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(PriorEpisodeFrontError::new(
            "prior-episode input id must be a bounded non-empty canonical coordinate",
        ));
    }
    Ok(())
}

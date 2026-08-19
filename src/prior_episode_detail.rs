use std::error::Error;
use std::fmt;

use serde::Serialize;

use crate::lesson_promotion::EnforcementKind;
use crate::prior_episode_front::{
    PRIOR_EPISODE_FRONT_SCHEMA_VERSION, PriorEpisodeFrontItem, PriorEpisodeFrontQuery,
    PriorEpisodeInput, PriorEpisodeNextAction, evaluate_prior_episode_front,
};
use crate::project_memory::ArtifactRef;

pub const PRIOR_EPISODE_DETAIL_SCHEMA_VERSION: u32 = 1;
pub const MAX_PRIOR_EPISODE_DETAIL_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PriorEpisodeDetail {
    AcceptedGuard {
        schema_version: u32,
        id: String,
        next: PriorEpisodeNextAction,
        candidate_discriminator_id: String,
        candidate_value_ref: String,
        operational_marker: String,
        guard: ArtifactRef,
        guard_marker: String,
        guard_source_evidence: String,
        enforcement_kind: EnforcementKind,
        enforcement_path: String,
        scope_ref: String,
        same_class_repairs: Vec<ArtifactRef>,
        automatic_policy_authority: bool,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PriorEpisodeDetailError {
    message: String,
}

impl PriorEpisodeDetailError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PriorEpisodeDetailError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PriorEpisodeDetailError {}

pub fn project_prior_episode_detail(
    input: &PriorEpisodeInput,
) -> Result<PriorEpisodeDetail, PriorEpisodeDetailError> {
    let query = PriorEpisodeFrontQuery {
        schema_version: PRIOR_EPISODE_FRONT_SCHEMA_VERSION,
        inputs: vec![input.clone()],
    };
    let front = evaluate_prior_episode_front(&query).map_err(|error| {
        PriorEpisodeDetailError::new(format!(
            "selected prior episode is not actionable: {error}"
        ))
    })?;

    if front.items.len() != 1 || !front.quiet.is_empty() {
        return Err(PriorEpisodeDetailError::new(
            "selected prior episode did not produce exactly one actionable front item",
        ));
    }

    let detail = match (input, &front.items[0]) {
        (
            PriorEpisodeInput::LessonPromotion { id, claim, .. },
            PriorEpisodeFrontItem::LessonPromotion {
                id: front_id,
                next,
                evaluation,
            },
        ) => {
            if id != front_id {
                return Err(PriorEpisodeDetailError::new(
                    "selected prior-episode id changed during front projection",
                ));
            }
            if *next != PriorEpisodeNextAction::UseAcceptedGuard {
                return Err(PriorEpisodeDetailError::new(
                    "lesson-promotion detail requires next=use_accepted_guard",
                ));
            }

            PriorEpisodeDetail::AcceptedGuard {
                schema_version: PRIOR_EPISODE_DETAIL_SCHEMA_VERSION,
                id: id.clone(),
                next: *next,
                candidate_discriminator_id: evaluation.candidate_discriminator_id.clone(),
                candidate_value_ref: evaluation.candidate_value_ref.clone(),
                operational_marker: claim.repair_marker.clone(),
                guard: evaluation.guard,
                guard_marker: claim.guard.marker.clone(),
                guard_source_evidence: claim.guard.source_evidence.clone(),
                enforcement_kind: evaluation.enforcement_kind,
                enforcement_path: evaluation.enforcement_path.clone(),
                scope_ref: evaluation.scope_ref.clone(),
                same_class_repairs: evaluation.same_class_repairs.clone(),
                automatic_policy_authority: evaluation.automatic_policy_authority,
            }
        }
        _ => {
            return Err(PriorEpisodeDetailError::new(
                "prior-episode detail v1 supports lesson_promotion inputs only",
            ));
        }
    };

    let serialized = serde_json::to_vec(&detail).map_err(|error| {
        PriorEpisodeDetailError::new(format!("cannot serialize selected prior detail: {error}"))
    })?;
    if serialized.len() > MAX_PRIOR_EPISODE_DETAIL_BYTES {
        return Err(PriorEpisodeDetailError::new(format!(
            "selected prior detail exceeds the {MAX_PRIOR_EPISODE_DETAIL_BYTES}-byte limit"
        )));
    }

    Ok(detail)
}

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::observation_frontier::{ObservationFrontierReceipt, ObservationFrontierStatus};
use crate::refinement_candidate_readiness::{
    CandidateEvidenceStatus, RefinementCandidateReadinessRequest,
    evaluate_refinement_candidate_readiness,
};
use crate::refinement_episode::RefinementStatus;

pub const REFINEMENT_INVESTIGATION_DEMAND_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefinementInvestigationDispositionStatus {
    Satisfied,
    RequirementMappingNeeded,
    ObservationAcquisitionNeeded,
    ReplayRejected,
    Unselected,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementInvestigationDisposition {
    pub episode_id: String,
    pub candidate_id: String,
    pub is_selected_transition: bool,
    pub replay_status: RefinementStatus,
    pub evidence_status: CandidateEvidenceStatus,
    pub disposition: RefinementInvestigationDispositionStatus,
    pub missing_requirement_mappings: Vec<String>,
    pub acquisition_frontiers: Vec<ObservationFrontierReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementInvestigationDemandEvaluation {
    pub schema_version: u32,
    pub candidates: Vec<RefinementInvestigationDisposition>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefinementInvestigationDemandError {
    message: String,
}

impl RefinementInvestigationDemandError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RefinementInvestigationDemandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RefinementInvestigationDemandError {}

pub fn evaluate_refinement_investigation_demand(
    request: &RefinementCandidateReadinessRequest,
) -> Result<RefinementInvestigationDemandEvaluation, RefinementInvestigationDemandError> {
    let readiness = evaluate_refinement_candidate_readiness(request).map_err(|error| {
        RefinementInvestigationDemandError::new(format!(
            "candidate readiness evaluation failed: {error}"
        ))
    })?;

    let mut candidates = Vec::with_capacity(readiness.candidates.len());
    for candidate in readiness.candidates {
        let replay_rejected = matches!(
            candidate.replay_status,
            RefinementStatus::RejectedNoImprovement
                | RefinementStatus::RejectedOverfit
                | RefinementStatus::RejectedLostExpectedCase
        );

        let (disposition, acquisition_frontiers) = if replay_rejected {
            (
                RefinementInvestigationDispositionStatus::ReplayRejected,
                Vec::new(),
            )
        } else if !candidate.is_selected_transition {
            (
                RefinementInvestigationDispositionStatus::Unselected,
                Vec::new(),
            )
        } else if candidate.evidence_status == CandidateEvidenceStatus::Current {
            (
                RefinementInvestigationDispositionStatus::Satisfied,
                Vec::new(),
            )
        } else if !candidate.missing_requirement_mappings.is_empty() {
            (
                RefinementInvestigationDispositionStatus::RequirementMappingNeeded,
                Vec::new(),
            )
        } else {
            let acquisition_frontiers = candidate
                .requirement_frontiers
                .iter()
                .filter(|frontier| frontier.status != ObservationFrontierStatus::Current)
                .cloned()
                .collect::<Vec<_>>();
            if acquisition_frontiers.is_empty() {
                return Err(RefinementInvestigationDemandError::new(format!(
                    "selected blocked candidate {} / {} has no missing mapping or noncurrent frontier",
                    candidate.episode_id, candidate.candidate_id
                )));
            }
            (
                RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded,
                acquisition_frontiers,
            )
        };

        candidates.push(RefinementInvestigationDisposition {
            episode_id: candidate.episode_id,
            candidate_id: candidate.candidate_id,
            is_selected_transition: candidate.is_selected_transition,
            replay_status: candidate.replay_status,
            evidence_status: candidate.evidence_status,
            disposition,
            missing_requirement_mappings: candidate.missing_requirement_mappings,
            acquisition_frontiers,
        });
    }

    Ok(RefinementInvestigationDemandEvaluation {
        schema_version: REFINEMENT_INVESTIGATION_DEMAND_SCHEMA_VERSION,
        candidates,
    })
}

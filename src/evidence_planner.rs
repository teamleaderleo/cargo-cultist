use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};
use crate::durable_obligation::{
    DiscriminatorKey, DurableObligation, DurableObligationStatus, evaluate_obligation,
};

pub const EVIDENCE_PLANNER_SCHEMA_VERSION: u32 = 1;
const MAX_PROBES: usize = 128;
const MAX_ID_BYTES: usize = 256;
const MAX_TEXT_BYTES: usize = 4096;
const MAX_COST_UNIT: u32 = 1_000_000;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbePlanRequest {
    pub schema_version: u32,
    pub obligation: DurableObligation,
    pub context: EvaluationContext,
    pub probes: Vec<EvidenceProbe>,
    pub allow_effectful: bool,
    pub policy: ProbeSelectionPolicy,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceProbe {
    pub id: String,
    pub produces: DiscriminatorKey,
    pub requirements: EvidenceRequirements,
    pub effect: ProbeEffect,
    pub cost: ProbeCost,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeEffect {
    ReadOnly,
    ExternalRead,
    Effectful,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeCost {
    pub git_subprocesses: u32,
    pub rust_files_parsed: u32,
    pub remote_requests: u32,
    pub effectful_executions: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeSelectionPolicy {
    Conservative,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeCandidateStatus {
    Eligible,
    Incapable,
    IncompatibleClearingCondition,
    InvalidCoordinate,
    MissingContext,
    EffectAuthorityRequired,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeCandidateReceipt {
    pub id: String,
    pub status: ProbeCandidateStatus,
    pub effect: ProbeEffect,
    pub cost: ProbeCost,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePlanStatus {
    Selected,
    Blocked,
    Unresolved,
    StaleObligation,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedProbe {
    pub id: String,
    pub produces: DiscriminatorKey,
    pub requirements: EvidenceRequirements,
    pub effect: ProbeEffect,
    pub cost: ProbeCost,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidencePlan {
    pub schema_version: u32,
    pub obligation_id: String,
    pub obligation_status: DurableObligationStatus,
    pub status: EvidencePlanStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<SelectedProbe>,
    pub candidates: Vec<ProbeCandidateReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EvidencePlannerError {
    message: String,
}

impl EvidencePlannerError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for EvidencePlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for EvidencePlannerError {}

pub fn plan_evidence(request: &ProbePlanRequest) -> Result<EvidencePlan, EvidencePlannerError> {
    validate_request(request)?;

    let obligation_evaluation = evaluate_obligation(&request.obligation, &[], &request.context)
        .map_err(|error| {
            EvidencePlannerError::new(format!("obligation evaluation failed: {error}"))
        })?;

    if obligation_evaluation.status == DurableObligationStatus::ReopenRequired {
        return Ok(EvidencePlan {
            schema_version: EVIDENCE_PLANNER_SCHEMA_VERSION,
            obligation_id: request.obligation.id.clone(),
            obligation_status: obligation_evaluation.status,
            status: EvidencePlanStatus::StaleObligation,
            selected: None,
            candidates: Vec::new(),
        });
    }

    let mut candidates = request
        .probes
        .iter()
        .map(|probe| {
            evaluate_candidate(
                &request.obligation,
                &request.context,
                request.allow_effectful,
                probe,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    candidates.sort_by(|left, right| left.receipt.id.cmp(&right.receipt.id));

    let selected_probe = candidates
        .iter()
        .filter(|candidate| candidate.receipt.status == ProbeCandidateStatus::Eligible)
        .map(|candidate| candidate.probe)
        .min_by(|left, right| compare_probe_cost(left, right, request.policy));

    let status = if selected_probe.is_some() {
        EvidencePlanStatus::Selected
    } else if obligation_evaluation.status == DurableObligationStatus::Unknown
        || candidates.iter().any(|candidate| {
            matches!(
                candidate.receipt.status,
                ProbeCandidateStatus::MissingContext
                    | ProbeCandidateStatus::EffectAuthorityRequired
            )
        })
    {
        EvidencePlanStatus::Blocked
    } else {
        EvidencePlanStatus::Unresolved
    };

    let selected = selected_probe.map(|probe| SelectedProbe {
        id: probe.id.clone(),
        produces: probe.produces.clone(),
        requirements: probe.requirements.clone(),
        effect: probe.effect,
        cost: probe.cost,
    });

    Ok(EvidencePlan {
        schema_version: EVIDENCE_PLANNER_SCHEMA_VERSION,
        obligation_id: request.obligation.id.clone(),
        obligation_status: obligation_evaluation.status,
        status,
        selected,
        candidates: candidates
            .into_iter()
            .map(|candidate| candidate.receipt)
            .collect(),
    })
}

struct CandidateEvaluation<'a> {
    probe: &'a EvidenceProbe,
    receipt: ProbeCandidateReceipt,
}

fn evaluate_candidate<'a>(
    obligation: &DurableObligation,
    context: &EvaluationContext,
    allow_effectful: bool,
    probe: &'a EvidenceProbe,
) -> Result<CandidateEvaluation<'a>, EvidencePlannerError> {
    let status = if probe.produces != obligation.missing_discriminator {
        ProbeCandidateStatus::Incapable
    } else if !obligation.clearing_conditions.iter().any(|condition| {
        condition.discriminator == probe.produces && condition.requirements == probe.requirements
    }) {
        ProbeCandidateStatus::IncompatibleClearingCondition
    } else {
        let applicability = evaluate_query(&ApplicabilityQuery {
            schema_version: APPLICABILITY_SCHEMA_VERSION,
            requirements: probe.requirements.clone(),
            context: context.clone(),
        })
        .map_err(|error| {
            EvidencePlannerError::new(format!(
                "probe {} applicability failed: {error}",
                probe.id
            ))
        })?;

        match applicability.status {
            ApplicabilityStatus::Invalid => ProbeCandidateStatus::InvalidCoordinate,
            ApplicabilityStatus::Unknown => ProbeCandidateStatus::MissingContext,
            ApplicabilityStatus::Applies
                if probe.effect == ProbeEffect::Effectful && !allow_effectful =>
            {
                ProbeCandidateStatus::EffectAuthorityRequired
            }
            ApplicabilityStatus::Applies => ProbeCandidateStatus::Eligible,
        }
    };

    Ok(CandidateEvaluation {
        probe,
        receipt: ProbeCandidateReceipt {
            id: probe.id.clone(),
            status,
            effect: probe.effect,
            cost: probe.cost,
        },
    })
}

fn compare_probe_cost(
    left: &EvidenceProbe,
    right: &EvidenceProbe,
    policy: ProbeSelectionPolicy,
) -> Ordering {
    match policy {
        ProbeSelectionPolicy::Conservative => conservative_cost_key(left)
            .cmp(&conservative_cost_key(right))
            .then_with(|| left.id.cmp(&right.id)),
    }
}

fn conservative_cost_key(probe: &EvidenceProbe) -> (u8, u32, u32, u32, u32) {
    (
        effect_rank(probe.effect),
        probe.cost.remote_requests,
        probe.cost.git_subprocesses,
        probe.cost.rust_files_parsed,
        probe.cost.effectful_executions,
    )
}

fn effect_rank(effect: ProbeEffect) -> u8 {
    match effect {
        ProbeEffect::ReadOnly => 0,
        ProbeEffect::ExternalRead => 1,
        ProbeEffect::Effectful => 2,
    }
}

fn validate_request(request: &ProbePlanRequest) -> Result<(), EvidencePlannerError> {
    if request.schema_version != EVIDENCE_PLANNER_SCHEMA_VERSION {
        return Err(EvidencePlannerError::new(format!(
            "unsupported evidence planner schema {}; expected {EVIDENCE_PLANNER_SCHEMA_VERSION}",
            request.schema_version
        )));
    }
    if request.probes.len() > MAX_PROBES {
        return Err(EvidencePlannerError::new(
            "probe inventory exceeds the admitted boundary",
        ));
    }

    let mut ids = BTreeSet::new();
    for probe in &request.probes {
        validate_probe(probe)?;
        if !ids.insert(probe.id.clone()) {
            return Err(EvidencePlannerError::new(format!(
                "duplicate probe id {}",
                probe.id
            )));
        }
    }
    Ok(())
}

fn validate_probe(probe: &EvidenceProbe) -> Result<(), EvidencePlannerError> {
    validate_id(&probe.id, "probe id")?;
    validate_id(&probe.produces.kind, "probe discriminator kind")?;
    validate_text(&probe.produces.target, "probe discriminator target")?;
    require_coordinates(&probe.requirements)?;
    validate_cost(probe)?;
    Ok(())
}

fn validate_cost(probe: &EvidenceProbe) -> Result<(), EvidencePlannerError> {
    let costs = [
        probe.cost.git_subprocesses,
        probe.cost.rust_files_parsed,
        probe.cost.remote_requests,
        probe.cost.effectful_executions,
    ];
    if costs.into_iter().any(|cost| cost > MAX_COST_UNIT) {
        return Err(EvidencePlannerError::new(format!(
            "probe {} cost exceeds the admitted boundary",
            probe.id
        )));
    }

    match probe.effect {
        ProbeEffect::ReadOnly
            if probe.cost.remote_requests != 0 || probe.cost.effectful_executions != 0 =>
        {
            Err(EvidencePlannerError::new(format!(
                "read-only probe {} cannot forecast remote requests or effectful executions",
                probe.id
            )))
        }
        ProbeEffect::ExternalRead if probe.cost.remote_requests == 0 => {
            Err(EvidencePlannerError::new(format!(
                "external-read probe {} must forecast at least one remote request",
                probe.id
            )))
        }
        ProbeEffect::ExternalRead if probe.cost.effectful_executions != 0 => {
            Err(EvidencePlannerError::new(format!(
                "external-read probe {} cannot forecast effectful executions",
                probe.id
            )))
        }
        ProbeEffect::Effectful if probe.cost.effectful_executions == 0 => {
            Err(EvidencePlannerError::new(format!(
                "effectful probe {} must forecast at least one effectful execution",
                probe.id
            )))
        }
        _ => Ok(()),
    }
}

fn require_coordinates(requirements: &EvidenceRequirements) -> Result<(), EvidencePlannerError> {
    if requirements.repository.is_none()
        && requirements.revision.is_none()
        && requirements.work.is_none()
        && requirements.scope.is_none()
    {
        return Err(EvidencePlannerError::new(
            "probe requirements must carry at least one applicability coordinate",
        ));
    }
    Ok(())
}

fn validate_id(value: &str, field: &str) -> Result<(), EvidencePlannerError> {
    if value.is_empty() || value.trim() != value || value.len() > MAX_ID_BYTES || value.contains('\0') {
        return Err(EvidencePlannerError::new(format!(
            "{field} must be a bounded canonical identifier"
        )));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str) -> Result<(), EvidencePlannerError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_TEXT_BYTES
        || value.contains('\0')
    {
        return Err(EvidencePlannerError::new(format!(
            "{field} must be bounded non-empty text"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durable_obligation::{
        ClearingCondition, ClearingEvidenceReceipt, DURABLE_OBLIGATION_SCHEMA_VERSION,
    };

    fn requirements(revision: Option<&str>) -> EvidenceRequirements {
        EvidenceRequirements {
            repository: Some("owner/repo".to_string()),
            revision: revision.map(str::to_string),
            work: None,
            scope: None,
        }
    }

    fn discriminator(kind: &str) -> DiscriminatorKey {
        DiscriminatorKey {
            kind: kind.to_string(),
            target: "cargo test target_t".to_string(),
        }
    }

    fn obligation() -> DurableObligation {
        DurableObligation {
            schema_version: DURABLE_OBLIGATION_SCHEMA_VERSION,
            id: "U17".to_string(),
            question: "Does target T pass at this exact head?".to_string(),
            subject: requirements(Some("head-a")),
            established_evidence: vec!["E-history".to_string()],
            missing_discriminator: discriminator("target_test_result"),
            clearing_conditions: vec![ClearingCondition {
                discriminator: discriminator("target_test_result"),
                requirements: requirements(Some("head-a")),
            }],
        }
    }

    fn context(revision: Option<&str>) -> EvaluationContext {
        EvaluationContext {
            repository: Some("owner/repo".to_string()),
            revision: revision.map(str::to_string),
            work: None,
            path: None,
        }
    }

    fn probe(
        id: &str,
        kind: &str,
        revision: Option<&str>,
        effect: ProbeEffect,
        cost: ProbeCost,
    ) -> EvidenceProbe {
        EvidenceProbe {
            id: id.to_string(),
            produces: discriminator(kind),
            requirements: requirements(revision),
            effect,
            cost,
        }
    }

    fn request(probes: Vec<EvidenceProbe>, allow_effectful: bool) -> ProbePlanRequest {
        ProbePlanRequest {
            schema_version: EVIDENCE_PLANNER_SCHEMA_VERSION,
            obligation: obligation(),
            context: context(Some("head-a")),
            probes,
            allow_effectful,
            policy: ProbeSelectionPolicy::Conservative,
        }
    }

    #[test]
    fn selects_capable_exact_head_probe_and_skips_cheaper_incapable_work() {
        let plan = plan_evidence(&request(
            vec![
                probe(
                    "history",
                    "historical_companion",
                    Some("head-a"),
                    ProbeEffect::ReadOnly,
                    ProbeCost {
                        git_subprocesses: 1,
                        ..ProbeCost::default()
                    },
                ),
                probe(
                    "stale-test",
                    "target_test_result",
                    Some("old-head"),
                    ProbeEffect::Effectful,
                    ProbeCost {
                        effectful_executions: 1,
                        ..ProbeCost::default()
                    },
                ),
                probe(
                    "exact-test",
                    "target_test_result",
                    Some("head-a"),
                    ProbeEffect::Effectful,
                    ProbeCost {
                        effectful_executions: 1,
                        ..ProbeCost::default()
                    },
                ),
            ],
            true,
        ))
        .unwrap();

        assert_eq!(plan.status, EvidencePlanStatus::Selected);
        assert_eq!(plan.selected.as_ref().unwrap().id, "exact-test");
        assert_eq!(plan.candidates[0].id, "exact-test");
        assert_eq!(plan.candidates[0].status, ProbeCandidateStatus::Eligible);
        assert_eq!(plan.candidates[1].status, ProbeCandidateStatus::Incapable);
        assert_eq!(
            plan.candidates[2].status,
            ProbeCandidateStatus::IncompatibleClearingCondition
        );
    }

    #[test]
    fn effectful_capability_can_be_selected_without_granting_execution_authority() {
        let plan = plan_evidence(&request(
            vec![probe(
                "exact-test",
                "target_test_result",
                Some("head-a"),
                ProbeEffect::Effectful,
                ProbeCost {
                    effectful_executions: 1,
                    ..ProbeCost::default()
                },
            )],
            false,
        ))
        .unwrap();

        assert_eq!(plan.status, EvidencePlanStatus::Blocked);
        assert!(plan.selected.is_none());
        assert_eq!(
            plan.candidates[0].status,
            ProbeCandidateStatus::EffectAuthorityRequired
        );
    }

    #[test]
    fn conservative_policy_prefers_read_only_capability_before_effectful_probe() {
        let mut record = obligation();
        record.missing_discriminator = discriminator("provider_current");
        record.clearing_conditions[0].discriminator = discriminator("provider_current");

        let mut request = request(
            vec![
                probe(
                    "remote-current",
                    "provider_current",
                    Some("head-a"),
                    ProbeEffect::ExternalRead,
                    ProbeCost {
                        remote_requests: 1,
                        ..ProbeCost::default()
                    },
                ),
                probe(
                    "effectful-current",
                    "provider_current",
                    Some("head-a"),
                    ProbeEffect::Effectful,
                    ProbeCost {
                        effectful_executions: 1,
                        ..ProbeCost::default()
                    },
                ),
            ],
            true,
        );
        request.obligation = record;

        let plan = plan_evidence(&request).unwrap();
        assert_eq!(plan.selected.as_ref().unwrap().id, "remote-current");
    }

    #[test]
    fn no_capable_probe_leaves_an_explicit_unresolved_frontier() {
        let plan = plan_evidence(&request(
            vec![probe(
                "history",
                "historical_companion",
                Some("head-a"),
                ProbeEffect::ReadOnly,
                ProbeCost {
                    git_subprocesses: 1,
                    ..ProbeCost::default()
                },
            )],
            true,
        ))
        .unwrap();

        assert_eq!(plan.status, EvidencePlanStatus::Unresolved);
        assert!(plan.selected.is_none());
        assert_eq!(plan.candidates[0].status, ProbeCandidateStatus::Incapable);
    }

    #[test]
    fn missing_current_coordinate_blocks_planning_instead_of_guessing() {
        let mut request = request(Vec::new(), true);
        request.context = context(None);

        let plan = plan_evidence(&request).unwrap();
        assert_eq!(plan.status, EvidencePlanStatus::Blocked);
        assert_eq!(plan.obligation_status, DurableObligationStatus::Unknown);
    }

    #[test]
    fn moved_obligation_subject_requires_reopen_before_new_probe_selection() {
        let mut request = request(Vec::new(), true);
        request.context = context(Some("head-b"));

        let plan = plan_evidence(&request).unwrap();
        assert_eq!(plan.status, EvidencePlanStatus::StaleObligation);
        assert_eq!(
            plan.obligation_status,
            DurableObligationStatus::ReopenRequired
        );
    }

    #[test]
    fn selected_probe_can_produce_a_receipt_that_clears_the_same_obligation() {
        let request = request(
            vec![probe(
                "exact-test",
                "target_test_result",
                Some("head-a"),
                ProbeEffect::Effectful,
                ProbeCost {
                    effectful_executions: 1,
                    ..ProbeCost::default()
                },
            )],
            true,
        );
        let plan = plan_evidence(&request).unwrap();
        let selected = plan.selected.unwrap();

        let receipt = ClearingEvidenceReceipt {
            id: "executed-test-receipt".to_string(),
            discriminator: selected.produces,
            requirements: selected.requirements,
        };
        let evaluation = evaluate_obligation(&request.obligation, &[receipt], &request.context)
            .unwrap();
        assert_eq!(evaluation.status, DurableObligationStatus::Cleared);
    }

    #[test]
    fn malformed_effect_forecast_fails_explicitly() {
        let error = plan_evidence(&request(
            vec![probe(
                "bad-test",
                "target_test_result",
                Some("head-a"),
                ProbeEffect::Effectful,
                ProbeCost::default(),
            )],
            true,
        ))
        .unwrap_err();

        assert!(error.to_string().contains("effectful executions"));
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};

pub const JUSTIFICATION_SCHEMA_VERSION: u32 = 1;
const MAX_NODES: usize = 1024;
const MAX_EDGES: usize = 4096;
const MAX_ID_BYTES: usize = 256;
const MAX_TEXT_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JustificationGraph {
    pub schema_version: u32,
    pub evidence: Vec<EvidenceNode>,
    pub claims: Vec<ClaimNode>,
    pub obligations: Vec<ObligationNode>,
    pub edges: Vec<JustificationEdge>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceNode {
    pub id: String,
    pub requirements: EvidenceRequirements,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimNode {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObligationNode {
    pub id: String,
    pub question: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum JustificationTarget {
    Claim(String),
    Obligation(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JustificationRelation {
    Support,
    Counterexample,
    Limit,
    Dependency,
    Clearing,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JustificationEdge {
    pub evidence_id: String,
    pub target: JustificationTarget,
    pub relation: JustificationRelation,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JustificationEvaluation {
    pub schema_version: u32,
    pub evidence: Vec<EvidenceEvaluation>,
    pub claims: Vec<ClaimEvaluation>,
    pub obligations: Vec<ObligationEvaluation>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceEvaluation {
    pub id: String,
    pub applicability: ApplicabilityStatus,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimJustificationStatus {
    Supported,
    Unknown,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RelationReceipt {
    pub applies: Vec<String>,
    pub invalid: Vec<String>,
    pub unknown: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimEvaluation {
    pub id: String,
    pub status: ClaimJustificationStatus,
    pub support: RelationReceipt,
    pub counterexamples: RelationReceipt,
    pub limits: RelationReceipt,
    pub dependencies: RelationReceipt,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationStatus {
    Open,
    Cleared,
    Unknown,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObligationEvaluation {
    pub id: String,
    pub status: ObligationStatus,
    pub clearing: RelationReceipt,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceTransition {
    pub id: String,
    pub before: ApplicabilityStatus,
    pub after: ApplicabilityStatus,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReevaluationReceipt {
    pub schema_version: u32,
    pub evidence_transitions: Vec<EvidenceTransition>,
    pub affected_targets: Vec<JustificationTarget>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JustificationError {
    message: String,
}

impl JustificationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for JustificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for JustificationError {}

pub fn evaluate_graph(
    graph: &JustificationGraph,
    context: &EvaluationContext,
) -> Result<JustificationEvaluation, JustificationError> {
    validate_graph(graph)?;
    let evidence_status = evaluate_evidence(graph, context)?;

    let mut claim_receipts: BTreeMap<String, ClaimReceipts> = graph
        .claims
        .iter()
        .map(|claim| (claim.id.clone(), ClaimReceipts::default()))
        .collect();
    let mut obligation_receipts: BTreeMap<String, RelationReceipt> = graph
        .obligations
        .iter()
        .map(|obligation| (obligation.id.clone(), RelationReceipt::default()))
        .collect();

    for edge in &graph.edges {
        let status = evidence_status[&edge.evidence_id];
        match (&edge.target, edge.relation) {
            (JustificationTarget::Claim(id), JustificationRelation::Support) => {
                claim_receipts
                    .get_mut(id)
                    .expect("validated claim target")
                    .support
                    .push(edge.evidence_id.clone(), status);
            }
            (JustificationTarget::Claim(id), JustificationRelation::Counterexample) => {
                claim_receipts
                    .get_mut(id)
                    .expect("validated claim target")
                    .counterexamples
                    .push(edge.evidence_id.clone(), status);
            }
            (JustificationTarget::Claim(id), JustificationRelation::Limit) => {
                claim_receipts
                    .get_mut(id)
                    .expect("validated claim target")
                    .limits
                    .push(edge.evidence_id.clone(), status);
            }
            (JustificationTarget::Claim(id), JustificationRelation::Dependency) => {
                claim_receipts
                    .get_mut(id)
                    .expect("validated claim target")
                    .dependencies
                    .push(edge.evidence_id.clone(), status);
            }
            (JustificationTarget::Obligation(id), JustificationRelation::Clearing) => {
                obligation_receipts
                    .get_mut(id)
                    .expect("validated obligation target")
                    .push(edge.evidence_id.clone(), status);
            }
            _ => unreachable!("validated relation/target pairing"),
        }
    }

    let evidence = evidence_status
        .iter()
        .map(|(id, applicability)| EvidenceEvaluation {
            id: id.clone(),
            applicability: *applicability,
        })
        .collect();

    let claims = graph
        .claims
        .iter()
        .map(|claim| {
            let mut receipts = claim_receipts
                .remove(&claim.id)
                .expect("claim accumulator exists");
            receipts.sort();

            let dependencies_block = !receipts.dependencies.invalid.is_empty()
                || !receipts.dependencies.unknown.is_empty();
            let status = if !dependencies_block && !receipts.support.applies.is_empty() {
                ClaimJustificationStatus::Supported
            } else {
                ClaimJustificationStatus::Unknown
            };

            ClaimEvaluation {
                id: claim.id.clone(),
                status,
                support: receipts.support,
                counterexamples: receipts.counterexamples,
                limits: receipts.limits,
                dependencies: receipts.dependencies,
            }
        })
        .collect();

    let obligations = graph
        .obligations
        .iter()
        .map(|obligation| {
            let mut clearing = obligation_receipts
                .remove(&obligation.id)
                .expect("obligation accumulator exists");
            clearing.sort();
            let status = if !clearing.applies.is_empty() {
                ObligationStatus::Cleared
            } else if !clearing.unknown.is_empty() {
                ObligationStatus::Unknown
            } else {
                ObligationStatus::Open
            };

            ObligationEvaluation {
                id: obligation.id.clone(),
                status,
                clearing,
            }
        })
        .collect();

    Ok(JustificationEvaluation {
        schema_version: JUSTIFICATION_SCHEMA_VERSION,
        evidence,
        claims,
        obligations,
    })
}

pub fn reevaluate_graph(
    graph: &JustificationGraph,
    before: &EvaluationContext,
    after: &EvaluationContext,
) -> Result<ReevaluationReceipt, JustificationError> {
    validate_graph(graph)?;
    let before_status = evaluate_evidence(graph, before)?;
    let after_status = evaluate_evidence(graph, after)?;

    let mut changed = BTreeSet::new();
    let mut evidence_transitions = Vec::new();
    for (id, before) in &before_status {
        let after = after_status[id];
        if *before != after {
            changed.insert(id.clone());
            evidence_transitions.push(EvidenceTransition {
                id: id.clone(),
                before: *before,
                after,
            });
        }
    }

    let affected_targets = graph
        .edges
        .iter()
        .filter(|edge| changed.contains(&edge.evidence_id))
        .map(|edge| edge.target.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(ReevaluationReceipt {
        schema_version: JUSTIFICATION_SCHEMA_VERSION,
        evidence_transitions,
        affected_targets,
    })
}

fn evaluate_evidence(
    graph: &JustificationGraph,
    context: &EvaluationContext,
) -> Result<BTreeMap<String, ApplicabilityStatus>, JustificationError> {
    graph
        .evidence
        .iter()
        .map(|evidence| {
            let query = ApplicabilityQuery {
                schema_version: APPLICABILITY_SCHEMA_VERSION,
                requirements: evidence.requirements.clone(),
                context: context.clone(),
            };
            let evaluation = evaluate_query(&query).map_err(|error| {
                JustificationError::new(format!(
                    "evidence {} applicability failed: {error}",
                    evidence.id
                ))
            })?;
            Ok((evidence.id.clone(), evaluation.status))
        })
        .collect()
}

fn validate_graph(graph: &JustificationGraph) -> Result<(), JustificationError> {
    if graph.schema_version != JUSTIFICATION_SCHEMA_VERSION {
        return Err(JustificationError::new(format!(
            "unsupported justification schema {}; expected {JUSTIFICATION_SCHEMA_VERSION}",
            graph.schema_version
        )));
    }

    let total_nodes = graph.evidence.len() + graph.claims.len() + graph.obligations.len();
    if total_nodes > MAX_NODES || graph.edges.len() > MAX_EDGES {
        return Err(JustificationError::new(
            "justification graph exceeds the admitted node/edge boundary",
        ));
    }

    let mut evidence_ids = BTreeSet::new();
    for evidence in &graph.evidence {
        validate_id(&evidence.id, "evidence id")?;
        if !evidence_ids.insert(evidence.id.clone()) {
            return Err(JustificationError::new(format!(
                "duplicate evidence id {}",
                evidence.id
            )));
        }
        if requirements_are_empty(&evidence.requirements) {
            return Err(JustificationError::new(format!(
                "evidence {} must carry at least one applicability requirement",
                evidence.id
            )));
        }
    }

    let mut claim_ids = BTreeSet::new();
    for claim in &graph.claims {
        validate_id(&claim.id, "claim id")?;
        validate_text(&claim.message, "claim message")?;
        if !claim_ids.insert(claim.id.clone()) {
            return Err(JustificationError::new(format!(
                "duplicate claim id {}",
                claim.id
            )));
        }
    }

    let mut obligation_ids = BTreeSet::new();
    for obligation in &graph.obligations {
        validate_id(&obligation.id, "obligation id")?;
        validate_text(&obligation.question, "obligation question")?;
        if !obligation_ids.insert(obligation.id.clone()) {
            return Err(JustificationError::new(format!(
                "duplicate obligation id {}",
                obligation.id
            )));
        }
    }

    let mut edge_keys = BTreeSet::new();
    let mut supported_claims = BTreeSet::new();
    for edge in &graph.edges {
        if !evidence_ids.contains(&edge.evidence_id) {
            return Err(JustificationError::new(format!(
                "edge references unknown evidence {}",
                edge.evidence_id
            )));
        }

        match (&edge.target, edge.relation) {
            (JustificationTarget::Claim(id), JustificationRelation::Clearing) => {
                return Err(JustificationError::new(format!(
                    "clearing relation cannot target claim {id}"
                )));
            }
            (JustificationTarget::Claim(id), relation) => {
                if !claim_ids.contains(id) {
                    return Err(JustificationError::new(format!(
                        "edge references unknown claim {id}"
                    )));
                }
                if relation == JustificationRelation::Support {
                    supported_claims.insert(id.clone());
                }
            }
            (JustificationTarget::Obligation(id), JustificationRelation::Clearing) => {
                if !obligation_ids.contains(id) {
                    return Err(JustificationError::new(format!(
                        "edge references unknown obligation {id}"
                    )));
                }
            }
            (JustificationTarget::Obligation(id), relation) => {
                return Err(JustificationError::new(format!(
                    "{relation:?} relation cannot target obligation {id}"
                )));
            }
        }

        let key = (edge.evidence_id.clone(), edge.target.clone(), edge.relation);
        if !edge_keys.insert(key) {
            return Err(JustificationError::new("duplicate justification edge"));
        }
    }

    if let Some(id) = claim_ids.difference(&supported_claims).next() {
        return Err(JustificationError::new(format!(
            "claim {id} has no support edge"
        )));
    }

    Ok(())
}

fn requirements_are_empty(requirements: &EvidenceRequirements) -> bool {
    requirements.repository.is_none()
        && requirements.revision.is_none()
        && requirements.work.is_none()
        && requirements.scope.is_none()
}

fn validate_id(id: &str, field: &str) -> Result<(), JustificationError> {
    if id.is_empty() || id.trim() != id || id.len() > MAX_ID_BYTES || id.contains('\0') {
        return Err(JustificationError::new(format!(
            "{field} must be a bounded canonical identifier"
        )));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str) -> Result<(), JustificationError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_TEXT_BYTES
        || value.contains('\0')
    {
        return Err(JustificationError::new(format!(
            "{field} must be bounded non-empty text"
        )));
    }
    Ok(())
}

impl RelationReceipt {
    fn push(&mut self, evidence_id: String, status: ApplicabilityStatus) {
        match status {
            ApplicabilityStatus::Applies => self.applies.push(evidence_id),
            ApplicabilityStatus::Invalid => self.invalid.push(evidence_id),
            ApplicabilityStatus::Unknown => self.unknown.push(evidence_id),
        }
    }

    fn sort(&mut self) {
        self.applies.sort();
        self.invalid.sort();
        self.unknown.sort();
    }
}

#[derive(Debug, Default)]
struct ClaimReceipts {
    support: RelationReceipt,
    counterexamples: RelationReceipt,
    limits: RelationReceipt,
    dependencies: RelationReceipt,
}

impl ClaimReceipts {
    fn sort(&mut self) {
        self.support.sort();
        self.counterexamples.sort();
        self.limits.sort();
        self.dependencies.sort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirements(repository: Option<&str>, revision: Option<&str>) -> EvidenceRequirements {
        EvidenceRequirements {
            repository: repository.map(str::to_string),
            revision: revision.map(str::to_string),
            work: None,
            scope: None,
        }
    }

    fn evidence(id: &str, requirements: EvidenceRequirements) -> EvidenceNode {
        EvidenceNode {
            id: id.to_string(),
            requirements,
        }
    }

    fn claim(id: &str) -> ClaimNode {
        ClaimNode {
            id: id.to_string(),
            message: format!("claim {id}"),
        }
    }

    fn support(evidence_id: &str, claim_id: &str) -> JustificationEdge {
        JustificationEdge {
            evidence_id: evidence_id.to_string(),
            target: JustificationTarget::Claim(claim_id.to_string()),
            relation: JustificationRelation::Support,
        }
    }

    fn context(repository: &str, revision: Option<&str>) -> EvaluationContext {
        EvaluationContext {
            repository: Some(repository.to_string()),
            revision: revision.map(str::to_string),
            work: None,
            path: None,
        }
    }

    #[test]
    fn independent_support_survives_one_invalidated_receipt() {
        let graph = JustificationGraph {
            schema_version: JUSTIFICATION_SCHEMA_VERSION,
            evidence: vec![
                evidence("E1", requirements(None, Some("old-head"))),
                evidence("E2", requirements(Some("owner/repo"), None)),
            ],
            claims: vec![claim("C1")],
            obligations: Vec::new(),
            edges: vec![support("E1", "C1"), support("E2", "C1")],
        };

        let evaluation = evaluate_graph(&graph, &context("owner/repo", Some("new-head"))).unwrap();
        assert_eq!(
            evaluation.claims[0].status,
            ClaimJustificationStatus::Supported
        );
        assert_eq!(evaluation.claims[0].support.applies, vec!["E2"]);
        assert_eq!(evaluation.claims[0].support.invalid, vec!["E1"]);
    }

    #[test]
    fn sole_invalidated_support_returns_claim_to_unknown() {
        let graph = JustificationGraph {
            schema_version: JUSTIFICATION_SCHEMA_VERSION,
            evidence: vec![evidence("E1", requirements(None, Some("old-head")))],
            claims: vec![claim("C1")],
            obligations: Vec::new(),
            edges: vec![support("E1", "C1")],
        };

        let evaluation = evaluate_graph(&graph, &context("owner/repo", Some("new-head"))).unwrap();
        assert_eq!(
            evaluation.claims[0].status,
            ClaimJustificationStatus::Unknown
        );
        assert_eq!(evaluation.claims[0].support.invalid, vec!["E1"]);
    }

    #[test]
    fn counterexamples_and_limits_remain_visible_without_collapsing_support() {
        let graph = JustificationGraph {
            schema_version: JUSTIFICATION_SCHEMA_VERSION,
            evidence: vec![
                evidence("E1", requirements(Some("owner/repo"), None)),
                evidence("E2", requirements(Some("owner/repo"), None)),
                evidence("E3", requirements(Some("owner/repo"), None)),
            ],
            claims: vec![claim("C1")],
            obligations: Vec::new(),
            edges: vec![
                support("E1", "C1"),
                JustificationEdge {
                    evidence_id: "E2".to_string(),
                    target: JustificationTarget::Claim("C1".to_string()),
                    relation: JustificationRelation::Counterexample,
                },
                JustificationEdge {
                    evidence_id: "E3".to_string(),
                    target: JustificationTarget::Claim("C1".to_string()),
                    relation: JustificationRelation::Limit,
                },
            ],
        };

        let evaluation = evaluate_graph(&graph, &context("owner/repo", Some("head"))).unwrap();
        let claim = &evaluation.claims[0];
        assert_eq!(claim.status, ClaimJustificationStatus::Supported);
        assert_eq!(claim.counterexamples.applies, vec!["E2"]);
        assert_eq!(claim.limits.applies, vec!["E3"]);
    }

    #[test]
    fn open_obligation_can_exist_before_clearing_evidence_arrives() {
        let graph = JustificationGraph {
            schema_version: JUSTIFICATION_SCHEMA_VERSION,
            evidence: Vec::new(),
            claims: Vec::new(),
            obligations: vec![ObligationNode {
                id: "U0".to_string(),
                question: "which exact evidence clears this?".to_string(),
            }],
            edges: Vec::new(),
        };

        let evaluation = evaluate_graph(&graph, &context("owner/repo", Some("head"))).unwrap();
        assert_eq!(evaluation.obligations[0].status, ObligationStatus::Open);
        assert!(evaluation.obligations[0].clearing.applies.is_empty());
    }

    #[test]
    fn exact_clearing_evidence_resolves_obligation_and_reopens_after_head_moves() {
        let graph = JustificationGraph {
            schema_version: JUSTIFICATION_SCHEMA_VERSION,
            evidence: vec![evidence("E1", requirements(None, Some("exact-head")))],
            claims: Vec::new(),
            obligations: vec![ObligationNode {
                id: "U1".to_string(),
                question: "run exact target execution?".to_string(),
            }],
            edges: vec![JustificationEdge {
                evidence_id: "E1".to_string(),
                target: JustificationTarget::Obligation("U1".to_string()),
                relation: JustificationRelation::Clearing,
            }],
        };

        let cleared = evaluate_graph(&graph, &context("owner/repo", Some("exact-head"))).unwrap();
        assert_eq!(cleared.obligations[0].status, ObligationStatus::Cleared);

        let reopened = evaluate_graph(&graph, &context("owner/repo", Some("new-head"))).unwrap();
        assert_eq!(reopened.obligations[0].status, ObligationStatus::Open);
        assert_eq!(reopened.obligations[0].clearing.invalid, vec!["E1"]);
    }

    #[test]
    fn reevaluation_names_only_targets_downstream_of_changed_evidence() {
        let graph = JustificationGraph {
            schema_version: JUSTIFICATION_SCHEMA_VERSION,
            evidence: vec![
                evidence("E1", requirements(None, Some("old-head"))),
                evidence("E2", requirements(Some("owner/repo"), None)),
            ],
            claims: vec![claim("C1"), claim("C2")],
            obligations: Vec::new(),
            edges: vec![support("E1", "C1"), support("E2", "C2")],
        };

        let receipt = reevaluate_graph(
            &graph,
            &context("owner/repo", Some("old-head")),
            &context("owner/repo", Some("new-head")),
        )
        .unwrap();

        assert_eq!(receipt.evidence_transitions.len(), 1);
        assert_eq!(receipt.evidence_transitions[0].id, "E1");
        assert_eq!(
            receipt.affected_targets,
            vec![JustificationTarget::Claim("C1".to_string())]
        );
    }

    #[test]
    fn dependency_unknown_prevents_supported_projection() {
        let graph = JustificationGraph {
            schema_version: JUSTIFICATION_SCHEMA_VERSION,
            evidence: vec![
                evidence("E1", requirements(Some("owner/repo"), None)),
                evidence("E2", requirements(None, Some("exact-head"))),
            ],
            claims: vec![claim("C1")],
            obligations: Vec::new(),
            edges: vec![
                support("E1", "C1"),
                JustificationEdge {
                    evidence_id: "E2".to_string(),
                    target: JustificationTarget::Claim("C1".to_string()),
                    relation: JustificationRelation::Dependency,
                },
            ],
        };

        let evaluation = evaluate_graph(&graph, &context("owner/repo", None)).unwrap();
        assert_eq!(
            evaluation.claims[0].status,
            ClaimJustificationStatus::Unknown
        );
        assert_eq!(evaluation.claims[0].dependencies.unknown, vec!["E2"]);
    }

    #[test]
    fn typed_targets_keep_v0_graph_acyclic_and_reject_relation_mismatch() {
        let graph = JustificationGraph {
            schema_version: JUSTIFICATION_SCHEMA_VERSION,
            evidence: vec![evidence("E1", requirements(Some("owner/repo"), None))],
            claims: Vec::new(),
            obligations: vec![ObligationNode {
                id: "U1".to_string(),
                question: "resolve?".to_string(),
            }],
            edges: vec![JustificationEdge {
                evidence_id: "E1".to_string(),
                target: JustificationTarget::Obligation("U1".to_string()),
                relation: JustificationRelation::Support,
            }],
        };

        let error = evaluate_graph(&graph, &context("owner/repo", Some("head"))).unwrap_err();
        assert!(error.to_string().contains("cannot target obligation"));
    }
}

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};
use crate::justification::RelationReceipt;

pub const DURABLE_OBLIGATION_SCHEMA_VERSION: u32 = 1;
const MAX_ID_BYTES: usize = 256;
const MAX_TEXT_BYTES: usize = 4096;
const MAX_ESTABLISHED_EVIDENCE: usize = 256;
const MAX_CLEARING_CONDITIONS: usize = 64;
const MAX_RECEIPTS: usize = 512;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DurableObligation {
    pub schema_version: u32,
    pub id: String,
    pub question: String,
    pub subject: EvidenceRequirements,
    pub established_evidence: Vec<String>,
    pub missing_discriminator: DiscriminatorKey,
    pub clearing_conditions: Vec<ClearingCondition>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscriminatorKey {
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClearingCondition {
    pub discriminator: DiscriminatorKey,
    pub requirements: EvidenceRequirements,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClearingEvidenceReceipt {
    pub id: String,
    pub discriminator: DiscriminatorKey,
    pub requirements: EvidenceRequirements,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DurableObligationStatus {
    Open,
    Cleared,
    Unknown,
    ReopenRequired,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DurableObligationEvaluation {
    pub schema_version: u32,
    pub id: String,
    pub status: DurableObligationStatus,
    pub subject_applicability: ApplicabilityStatus,
    pub clearing: RelationReceipt,
    pub unmatched_receipts: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DurableObligationError {
    message: String,
}

impl DurableObligationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DurableObligationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for DurableObligationError {}

pub fn evaluate_obligation(
    obligation: &DurableObligation,
    receipts: &[ClearingEvidenceReceipt],
    context: &EvaluationContext,
) -> Result<DurableObligationEvaluation, DurableObligationError> {
    validate_obligation(obligation)?;
    validate_receipts(receipts)?;

    let subject_applicability = evaluate_requirements(&obligation.subject, context, "subject")?;
    let mut clearing = RelationReceipt::default();
    let mut unmatched_receipts = Vec::new();

    for receipt in receipts {
        let Some(condition) = obligation.clearing_conditions.iter().find(|condition| {
            condition.discriminator == receipt.discriminator
                && condition.requirements == receipt.requirements
        }) else {
            unmatched_receipts.push(receipt.id.clone());
            continue;
        };

        debug_assert_eq!(condition.requirements, receipt.requirements);
        let applicability = evaluate_requirements(
            &receipt.requirements,
            context,
            &format!("clearing receipt {}", receipt.id),
        )?;
        push_receipt(&mut clearing, receipt.id.clone(), applicability);
    }

    clearing.applies.sort();
    clearing.invalid.sort();
    clearing.unknown.sort();
    unmatched_receipts.sort();

    let status = match subject_applicability {
        ApplicabilityStatus::Invalid => DurableObligationStatus::ReopenRequired,
        ApplicabilityStatus::Unknown => DurableObligationStatus::Unknown,
        ApplicabilityStatus::Applies if !clearing.applies.is_empty() => {
            DurableObligationStatus::Cleared
        }
        ApplicabilityStatus::Applies if !clearing.unknown.is_empty() => {
            DurableObligationStatus::Unknown
        }
        ApplicabilityStatus::Applies => DurableObligationStatus::Open,
    };

    Ok(DurableObligationEvaluation {
        schema_version: DURABLE_OBLIGATION_SCHEMA_VERSION,
        id: obligation.id.clone(),
        status,
        subject_applicability,
        clearing,
        unmatched_receipts,
    })
}

fn evaluate_requirements(
    requirements: &EvidenceRequirements,
    context: &EvaluationContext,
    label: &str,
) -> Result<ApplicabilityStatus, DurableObligationError> {
    let evaluation = evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: requirements.clone(),
        context: context.clone(),
    })
    .map_err(|error| {
        DurableObligationError::new(format!("{label} applicability failed: {error}"))
    })?;
    Ok(evaluation.status)
}

fn validate_obligation(obligation: &DurableObligation) -> Result<(), DurableObligationError> {
    if obligation.schema_version != DURABLE_OBLIGATION_SCHEMA_VERSION {
        return Err(DurableObligationError::new(format!(
            "unsupported durable obligation schema {}; expected {DURABLE_OBLIGATION_SCHEMA_VERSION}",
            obligation.schema_version
        )));
    }

    validate_id(&obligation.id, "obligation id")?;
    validate_text(&obligation.question, "obligation question")?;
    validate_discriminator(&obligation.missing_discriminator)?;
    require_coordinates(&obligation.subject, "obligation subject")?;

    if obligation.established_evidence.len() > MAX_ESTABLISHED_EVIDENCE {
        return Err(DurableObligationError::new(
            "established evidence exceeds the admitted boundary",
        ));
    }
    let mut established = BTreeSet::new();
    for id in &obligation.established_evidence {
        validate_id(id, "established evidence id")?;
        if !established.insert(id.clone()) {
            return Err(DurableObligationError::new(format!(
                "duplicate established evidence id {id}"
            )));
        }
    }

    if obligation.clearing_conditions.is_empty()
        || obligation.clearing_conditions.len() > MAX_CLEARING_CONDITIONS
    {
        return Err(DurableObligationError::new(
            "durable obligation must declare a bounded non-empty clearing condition set",
        ));
    }

    let mut conditions = BTreeSet::new();
    for condition in &obligation.clearing_conditions {
        validate_discriminator(&condition.discriminator)?;
        require_coordinates(&condition.requirements, "clearing condition requirements")?;
        let key = (
            condition.discriminator.clone(),
            serde_json::to_string(&condition.requirements).map_err(|error| {
                DurableObligationError::new(format!(
                    "failed to canonicalize clearing requirements: {error}"
                ))
            })?,
        );
        if !conditions.insert(key) {
            return Err(DurableObligationError::new(
                "duplicate durable obligation clearing condition",
            ));
        }
    }

    if !obligation
        .clearing_conditions
        .iter()
        .any(|condition| condition.discriminator == obligation.missing_discriminator)
    {
        return Err(DurableObligationError::new(
            "at least one clearing condition must answer the missing discriminator",
        ));
    }

    Ok(())
}

fn validate_receipts(receipts: &[ClearingEvidenceReceipt]) -> Result<(), DurableObligationError> {
    if receipts.len() > MAX_RECEIPTS {
        return Err(DurableObligationError::new(
            "clearing receipts exceed the admitted boundary",
        ));
    }

    let mut ids = BTreeSet::new();
    for receipt in receipts {
        validate_id(&receipt.id, "clearing receipt id")?;
        validate_discriminator(&receipt.discriminator)?;
        require_coordinates(&receipt.requirements, "clearing receipt requirements")?;
        if !ids.insert(receipt.id.clone()) {
            return Err(DurableObligationError::new(format!(
                "duplicate clearing receipt id {}",
                receipt.id
            )));
        }
    }
    Ok(())
}

fn require_coordinates(
    requirements: &EvidenceRequirements,
    label: &str,
) -> Result<(), DurableObligationError> {
    if requirements.repository.is_none()
        && requirements.revision.is_none()
        && requirements.work.is_none()
        && requirements.scope.is_none()
    {
        return Err(DurableObligationError::new(format!(
            "{label} must carry at least one applicability coordinate"
        )));
    }
    Ok(())
}

fn validate_discriminator(discriminator: &DiscriminatorKey) -> Result<(), DurableObligationError> {
    validate_id(&discriminator.kind, "discriminator kind")?;
    validate_text(&discriminator.target, "discriminator target")
}

fn validate_id(value: &str, field: &str) -> Result<(), DurableObligationError> {
    if value.is_empty() || value.trim() != value || value.len() > MAX_ID_BYTES || value.contains('\0') {
        return Err(DurableObligationError::new(format!(
            "{field} must be a bounded canonical identifier"
        )));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str) -> Result<(), DurableObligationError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_TEXT_BYTES
        || value.contains('\0')
    {
        return Err(DurableObligationError::new(format!(
            "{field} must be bounded non-empty text"
        )));
    }
    Ok(())
}

fn push_receipt(receipt: &mut RelationReceipt, id: String, status: ApplicabilityStatus) {
    match status {
        ApplicabilityStatus::Applies => receipt.applies.push(id),
        ApplicabilityStatus::Invalid => receipt.invalid.push(id),
        ApplicabilityStatus::Unknown => receipt.unknown.push(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirements(repository: &str, revision: Option<&str>) -> EvidenceRequirements {
        EvidenceRequirements {
            repository: Some(repository.to_string()),
            revision: revision.map(str::to_string),
            work: None,
            scope: None,
        }
    }

    fn discriminator() -> DiscriminatorKey {
        DiscriminatorKey {
            kind: "target_test_result".to_string(),
            target: "cargo test target_t".to_string(),
        }
    }

    fn obligation() -> DurableObligation {
        DurableObligation {
            schema_version: DURABLE_OBLIGATION_SCHEMA_VERSION,
            id: "U17".to_string(),
            question: "Does target T pass at this exact head?".to_string(),
            subject: requirements("owner/repo", Some("head-a")),
            established_evidence: vec!["E-history".to_string(), "E-guidance".to_string()],
            missing_discriminator: discriminator(),
            clearing_conditions: vec![ClearingCondition {
                discriminator: discriminator(),
                requirements: requirements("owner/repo", Some("head-a")),
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

    fn receipt(revision: &str) -> ClearingEvidenceReceipt {
        ClearingEvidenceReceipt {
            id: format!("test-{revision}"),
            discriminator: discriminator(),
            requirements: requirements("owner/repo", Some(revision)),
        }
    }

    #[test]
    fn durable_unknown_can_exist_before_any_clearing_evidence_arrives() {
        let evaluation = evaluate_obligation(&obligation(), &[], &context(Some("head-a"))).unwrap();
        assert_eq!(evaluation.status, DurableObligationStatus::Open);
        assert!(evaluation.clearing.applies.is_empty());
    }

    #[test]
    fn exact_matching_receipt_clears_the_obligation() {
        let evaluation = evaluate_obligation(
            &obligation(),
            &[receipt("head-a")],
            &context(Some("head-a")),
        )
        .unwrap();
        assert_eq!(evaluation.status, DurableObligationStatus::Cleared);
        assert_eq!(evaluation.clearing.applies, vec!["test-head-a"]);
    }

    #[test]
    fn moved_subject_coordinate_requires_reopen() {
        let evaluation = evaluate_obligation(
            &obligation(),
            &[receipt("head-a")],
            &context(Some("head-b")),
        )
        .unwrap();
        assert_eq!(
            evaluation.status,
            DurableObligationStatus::ReopenRequired
        );
        assert_eq!(evaluation.subject_applicability, ApplicabilityStatus::Invalid);
        assert_eq!(evaluation.clearing.invalid, vec!["test-head-a"]);
    }

    #[test]
    fn missing_current_coordinate_remains_unknown() {
        let evaluation = evaluate_obligation(&obligation(), &[], &context(None)).unwrap();
        assert_eq!(evaluation.status, DurableObligationStatus::Unknown);
        assert_eq!(evaluation.subject_applicability, ApplicabilityStatus::Unknown);
    }

    #[test]
    fn semantically_adjacent_receipt_does_not_clear() {
        let mut wrong = receipt("head-a");
        wrong.discriminator.kind = "target_test_listing".to_string();
        let evaluation = evaluate_obligation(&obligation(), &[wrong], &context(Some("head-a"))).unwrap();
        assert_eq!(evaluation.status, DurableObligationStatus::Open);
        assert_eq!(evaluation.unmatched_receipts, vec!["test-head-a"]);
    }

    #[test]
    fn durable_record_round_trips_for_fresh_worker_handoff() {
        let record = obligation();
        let encoded = serde_json::to_string(&record).unwrap();
        let decoded: DurableObligation = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, record);
        assert_eq!(decoded.established_evidence, vec!["E-history", "E-guidance"]);
    }

    #[test]
    fn clearing_condition_must_answer_declared_missing_discriminator() {
        let mut record = obligation();
        record.clearing_conditions[0].discriminator.kind = "provider_current".to_string();
        let error = evaluate_obligation(&record, &[], &context(Some("head-a"))).unwrap_err();
        assert!(error.to_string().contains("answer the missing discriminator"));
    }
}

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::applicability::{
    ApplicabilityDimension, ApplicabilityQuery, DimensionStatus, EvaluationContext,
    EvidenceApplicability, EvidenceRequirements, PathScope, evaluate_query,
};

pub const REVIEW_MEMORY_SCHEMA_VERSION: u32 = 1;
pub const MAX_REVIEW_MEMORY_QUERY_BYTES: usize = 256 * 1024;
const MAX_RECORDS: usize = 256;
const MAX_ID_BYTES: usize = 1024;
const MAX_REF_BYTES: usize = 2048;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewMemoryQuery {
    pub schema_version: u32,
    pub current: CurrentConcern,
    #[serde(default)]
    pub records: Vec<ReviewMemoryRecord>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentConcern {
    pub concern_key: String,
    pub context: EvaluationContext,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewMemoryRecord {
    pub event_id: String,
    pub concern_key: String,
    pub source_ref: String,
    pub subject: ReviewSubject,
    pub outcome: ReviewOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_ref: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSubject {
    pub repository: String,
    pub work: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<PathScope>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOutcome {
    Open,
    PatchChanged,
    RejectedWithEvidence,
    Dismissed,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewThreadDisposition {
    ReuseCurrentThread,
    RefreshExistingThread,
    NeedContext,
    NewThread,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMemoryMatchKind {
    Current,
    PriorHead,
    ContextMissing,
    Unrelated,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewMemoryMatch {
    pub event_id: String,
    pub source_ref: String,
    pub outcome: ReviewOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_ref: Option<String>,
    pub match_kind: ReviewMemoryMatchKind,
    pub applicability: EvidenceApplicability,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewMemoryEvaluation {
    pub schema_version: u32,
    pub concern_key: String,
    pub disposition: ReviewThreadDisposition,
    pub matches: Vec<ReviewMemoryMatch>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReviewMemoryError {
    message: String,
}

impl ReviewMemoryError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ReviewMemoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ReviewMemoryError {}

pub fn parse_review_memory_query(bytes: &[u8]) -> Result<ReviewMemoryQuery, ReviewMemoryError> {
    if bytes.len() > MAX_REVIEW_MEMORY_QUERY_BYTES {
        return Err(ReviewMemoryError::new(format!(
            "review-memory query exceeds the {MAX_REVIEW_MEMORY_QUERY_BYTES}-byte limit"
        )));
    }
    let query: ReviewMemoryQuery = serde_json::from_slice(bytes)
        .map_err(|error| ReviewMemoryError::new(format!("invalid review-memory JSON: {error}")))?;
    validate_query(&query)?;
    Ok(query)
}

pub fn evaluate_review_memory(
    query: &ReviewMemoryQuery,
) -> Result<ReviewMemoryEvaluation, ReviewMemoryError> {
    validate_query(query)?;

    let mut matches = Vec::new();
    for record in query
        .records
        .iter()
        .filter(|record| record.concern_key == query.current.concern_key)
    {
        let applicability = evaluate_query(&ApplicabilityQuery {
            schema_version: crate::applicability::APPLICABILITY_SCHEMA_VERSION,
            requirements: record.subject.requirements(),
            context: query.current.context.clone(),
        })
        .map_err(|error| {
            ReviewMemoryError::new(format!(
                "review event `{}` has invalid applicability input: {error}",
                record.event_id
            ))
        })?;
        let match_kind = classify_match(&applicability);
        matches.push(ReviewMemoryMatch {
            event_id: record.event_id.clone(),
            source_ref: record.source_ref.clone(),
            outcome: record.outcome,
            resolution_ref: record.resolution_ref.clone(),
            match_kind,
            applicability,
        });
    }

    matches.sort_by(|left, right| left.event_id.cmp(&right.event_id));

    let current_identity_missing = query.current.context.repository.is_none()
        || query.current.context.revision.is_none()
        || query.current.context.work.is_none();

    let disposition = if current_identity_missing {
        ReviewThreadDisposition::NeedContext
    } else if matches
        .iter()
        .any(|entry| entry.match_kind == ReviewMemoryMatchKind::Current)
    {
        ReviewThreadDisposition::ReuseCurrentThread
    } else if matches
        .iter()
        .any(|entry| entry.match_kind == ReviewMemoryMatchKind::PriorHead)
    {
        ReviewThreadDisposition::RefreshExistingThread
    } else if matches
        .iter()
        .any(|entry| entry.match_kind == ReviewMemoryMatchKind::ContextMissing)
    {
        ReviewThreadDisposition::NeedContext
    } else {
        ReviewThreadDisposition::NewThread
    };

    Ok(ReviewMemoryEvaluation {
        schema_version: REVIEW_MEMORY_SCHEMA_VERSION,
        concern_key: query.current.concern_key.clone(),
        disposition,
        matches,
    })
}

impl ReviewSubject {
    fn requirements(&self) -> EvidenceRequirements {
        EvidenceRequirements {
            repository: Some(self.repository.clone()),
            revision: Some(self.revision.clone()),
            work: Some(self.work.clone()),
            scope: self.scope.clone(),
        }
    }
}

fn classify_match(applicability: &EvidenceApplicability) -> ReviewMemoryMatchKind {
    let mut revision_status = None;
    let mut other_mismatch = false;
    let mut other_missing = false;

    for dimension in &applicability.dimensions {
        if dimension.dimension == ApplicabilityDimension::Revision {
            revision_status = Some(dimension.status);
            continue;
        }
        match dimension.status {
            DimensionStatus::Matched => {}
            DimensionStatus::Mismatched => other_mismatch = true,
            DimensionStatus::Missing => other_missing = true,
        }
    }

    if other_mismatch {
        return ReviewMemoryMatchKind::Unrelated;
    }
    if other_missing {
        return ReviewMemoryMatchKind::ContextMissing;
    }

    match revision_status {
        Some(DimensionStatus::Matched) => ReviewMemoryMatchKind::Current,
        Some(DimensionStatus::Mismatched) => ReviewMemoryMatchKind::PriorHead,
        Some(DimensionStatus::Missing) | None => ReviewMemoryMatchKind::ContextMissing,
    }
}

fn validate_query(query: &ReviewMemoryQuery) -> Result<(), ReviewMemoryError> {
    if query.schema_version != REVIEW_MEMORY_SCHEMA_VERSION {
        return Err(ReviewMemoryError::new(format!(
            "unsupported review-memory schema {}; expected {REVIEW_MEMORY_SCHEMA_VERSION}",
            query.schema_version
        )));
    }
    if query.records.len() > MAX_RECORDS {
        return Err(ReviewMemoryError::new(format!(
            "review-memory query may contain at most {MAX_RECORDS} records"
        )));
    }

    validate_id(&query.current.concern_key, "current.concern_key")?;
    validate_context(&query.current.context)?;

    let mut by_event_id = BTreeMap::<&str, &ReviewMemoryRecord>::new();
    for record in &query.records {
        validate_record(record)?;
        if let Some(existing) = by_event_id.insert(&record.event_id, record) {
            let kind = if existing == record {
                "duplicate"
            } else {
                "conflicting duplicate"
            };
            return Err(ReviewMemoryError::new(format!(
                "{kind} review event_id `{}`",
                record.event_id
            )));
        }
    }
    Ok(())
}

fn validate_record(record: &ReviewMemoryRecord) -> Result<(), ReviewMemoryError> {
    validate_id(&record.event_id, "event_id")?;
    validate_id(&record.concern_key, "concern_key")?;
    validate_ref(&record.source_ref, "source_ref")?;
    validate_coordinate(&record.subject.repository, "subject.repository")?;
    validate_coordinate(&record.subject.work, "subject.work")?;
    validate_git_revision(&record.subject.revision, "subject.revision")?;
    if let Some(scope) = &record.subject.scope {
        validate_path(&scope.path, "subject.scope.path")?;
    }

    match (record.outcome, record.resolution_ref.as_deref()) {
        (ReviewOutcome::Open, None) => {}
        (ReviewOutcome::Open, Some(_)) => {
            return Err(ReviewMemoryError::new(
                "open review outcome must not carry resolution_ref",
            ));
        }
        (
            ReviewOutcome::PatchChanged
            | ReviewOutcome::RejectedWithEvidence
            | ReviewOutcome::Dismissed,
            Some(reference),
        ) => validate_ref(reference, "resolution_ref")?,
        (
            ReviewOutcome::PatchChanged
            | ReviewOutcome::RejectedWithEvidence
            | ReviewOutcome::Dismissed,
            None,
        ) => {
            return Err(ReviewMemoryError::new(
                "resolved review outcome requires resolution_ref",
            ));
        }
    }
    Ok(())
}

fn validate_context(context: &EvaluationContext) -> Result<(), ReviewMemoryError> {
    if let Some(repository) = &context.repository {
        validate_coordinate(repository, "current.context.repository")?;
    }
    if let Some(revision) = &context.revision {
        validate_git_revision(revision, "current.context.revision")?;
    }
    if let Some(work) = &context.work {
        validate_coordinate(work, "current.context.work")?;
    }
    if let Some(path) = &context.path {
        validate_path(path, "current.context.path")?;
    }
    Ok(())
}

fn validate_id(value: &str, field: &str) -> Result<(), ReviewMemoryError> {
    validate_bounded_single_line(value, field, MAX_ID_BYTES)
}

fn validate_ref(value: &str, field: &str) -> Result<(), ReviewMemoryError> {
    validate_bounded_single_line(value, field, MAX_REF_BYTES)
}

fn validate_coordinate(value: &str, field: &str) -> Result<(), ReviewMemoryError> {
    validate_bounded_single_line(value, field, MAX_ID_BYTES)
}

fn validate_bounded_single_line(
    value: &str,
    field: &str,
    maximum: usize,
) -> Result<(), ReviewMemoryError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > maximum
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(ReviewMemoryError::new(format!(
            "{field} must be a bounded non-empty canonical single-line value"
        )));
    }
    Ok(())
}

fn validate_git_revision(value: &str, field: &str) -> Result<(), ReviewMemoryError> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ReviewMemoryError::new(format!(
            "{field} must be an exact 40-character lowercase Git object id"
        )));
    }
    Ok(())
}

fn validate_path(path: &str, field: &str) -> Result<(), ReviewMemoryError> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(ReviewMemoryError::new(format!(
            "{field} must be a canonical repository-relative path"
        )));
    }
    Ok(())
}

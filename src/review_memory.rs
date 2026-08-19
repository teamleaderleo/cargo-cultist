use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::applicability::{
    ApplicabilityDimension, ApplicabilityQuery, ApplicabilityStatus, DimensionStatus,
    EvaluationContext, EvidenceApplicability, EvidenceRequirements, PathScope, evaluate_query,
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

    let disposition = if matches
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::applicability::{PathScopeMode, ApplicabilityStatus};

    const HEAD_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEAD_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEAD_C: &str = "cccccccccccccccccccccccccccccccccccccccc";

    fn record(
        event_id: &str,
        revision: &str,
        work: &str,
        scope: Option<PathScope>,
        outcome: ReviewOutcome,
        resolution_ref: Option<&str>,
    ) -> ReviewMemoryRecord {
        ReviewMemoryRecord {
            event_id: event_id.to_string(),
            concern_key: "review:unused-result:src/lib.rs".to_string(),
            source_ref: "github:review-comment/100".to_string(),
            subject: ReviewSubject {
                repository: "owner/repo".to_string(),
                work: work.to_string(),
                revision: revision.to_string(),
                scope,
            },
            outcome,
            resolution_ref: resolution_ref.map(str::to_string),
        }
    }

    fn query(
        records: Vec<ReviewMemoryRecord>,
        revision: Option<&str>,
        work: Option<&str>,
        path: Option<&str>,
    ) -> ReviewMemoryQuery {
        ReviewMemoryQuery {
            schema_version: REVIEW_MEMORY_SCHEMA_VERSION,
            current: CurrentConcern {
                concern_key: "review:unused-result:src/lib.rs".to_string(),
                context: EvaluationContext {
                    repository: Some("owner/repo".to_string()),
                    revision: revision.map(str::to_string),
                    work: work.map(str::to_string),
                    path: path.map(str::to_string),
                },
            },
            records,
        }
    }

    #[test]
    fn same_exact_head_reuses_current_thread() {
        let evaluation = evaluate_review_memory(&query(
            vec![record(
                "event:1",
                HEAD_A,
                "#7",
                None,
                ReviewOutcome::Open,
                None,
            )],
            Some(HEAD_A),
            Some("#7"),
            None,
        ))
        .unwrap();

        assert_eq!(
            evaluation.disposition,
            ReviewThreadDisposition::ReuseCurrentThread
        );
        assert_eq!(evaluation.matches[0].match_kind, ReviewMemoryMatchKind::Current);
        assert_eq!(
            evaluation.matches[0].applicability.status,
            ApplicabilityStatus::Applies
        );
    }

    #[test]
    fn moved_head_refreshes_thread_and_invalidates_old_resolution() {
        let evaluation = evaluate_review_memory(&query(
            vec![record(
                "event:1",
                HEAD_A,
                "#7",
                None,
                ReviewOutcome::RejectedWithEvidence,
                Some("github:review-comment/101"),
            )],
            Some(HEAD_B),
            Some("#7"),
            None,
        ))
        .unwrap();

        assert_eq!(
            evaluation.disposition,
            ReviewThreadDisposition::RefreshExistingThread
        );
        assert_eq!(
            evaluation.matches[0].match_kind,
            ReviewMemoryMatchKind::PriorHead
        );
        assert_eq!(
            evaluation.matches[0].applicability.status,
            ApplicabilityStatus::Invalid
        );
        assert_eq!(
            evaluation.matches[0].outcome,
            ReviewOutcome::RejectedWithEvidence
        );
    }

    #[test]
    fn missing_current_head_requires_context() {
        let evaluation = evaluate_review_memory(&query(
            vec![record(
                "event:1",
                HEAD_A,
                "#7",
                None,
                ReviewOutcome::Open,
                None,
            )],
            None,
            Some("#7"),
            None,
        ))
        .unwrap();

        assert_eq!(
            evaluation.disposition,
            ReviewThreadDisposition::NeedContext
        );
        assert_eq!(
            evaluation.matches[0].match_kind,
            ReviewMemoryMatchKind::ContextMissing
        );
        assert_eq!(
            evaluation.matches[0].applicability.status,
            ApplicabilityStatus::Unknown
        );
    }

    #[test]
    fn same_concern_key_on_different_work_starts_new_thread() {
        let evaluation = evaluate_review_memory(&query(
            vec![record(
                "event:1",
                HEAD_A,
                "#7",
                None,
                ReviewOutcome::Open,
                None,
            )],
            Some(HEAD_A),
            Some("#8"),
            None,
        ))
        .unwrap();

        assert_eq!(evaluation.disposition, ReviewThreadDisposition::NewThread);
        assert_eq!(
            evaluation.matches[0].match_kind,
            ReviewMemoryMatchKind::Unrelated
        );
    }

    #[test]
    fn same_concern_key_on_different_scope_starts_new_thread() {
        let scope = PathScope {
            mode: PathScopeMode::Exact,
            path: "src/lib.rs".to_string(),
        };
        let evaluation = evaluate_review_memory(&query(
            vec![record(
                "event:1",
                HEAD_A,
                "#7",
                Some(scope),
                ReviewOutcome::Open,
                None,
            )],
            Some(HEAD_A),
            Some("#7"),
            Some("src/other.rs"),
        ))
        .unwrap();

        assert_eq!(evaluation.disposition, ReviewThreadDisposition::NewThread);
        assert_eq!(
            evaluation.matches[0].match_kind,
            ReviewMemoryMatchKind::Unrelated
        );
    }

    #[test]
    fn multiple_prior_events_are_retained_without_latest_inference() {
        let evaluation = evaluate_review_memory(&query(
            vec![
                record(
                    "event:b",
                    HEAD_B,
                    "#7",
                    None,
                    ReviewOutcome::Dismissed,
                    Some("github:review-comment/102"),
                ),
                record(
                    "event:a",
                    HEAD_A,
                    "#7",
                    None,
                    ReviewOutcome::PatchChanged,
                    Some("github:commit/change"),
                ),
            ],
            Some(HEAD_C),
            Some("#7"),
            None,
        ))
        .unwrap();

        assert_eq!(
            evaluation.disposition,
            ReviewThreadDisposition::RefreshExistingThread
        );
        assert_eq!(
            evaluation
                .matches
                .iter()
                .map(|entry| entry.event_id.as_str())
                .collect::<Vec<_>>(),
            vec!["event:a", "event:b"]
        );
        assert!(evaluation
            .matches
            .iter()
            .all(|entry| entry.match_kind == ReviewMemoryMatchKind::PriorHead));
    }

    #[test]
    fn duplicate_and_conflicting_event_ids_reject() {
        let first = record(
            "event:1",
            HEAD_A,
            "#7",
            None,
            ReviewOutcome::Open,
            None,
        );
        let duplicate = first.clone();
        let mut conflict = first.clone();
        conflict.source_ref = "github:review-comment/999".to_string();

        let duplicate_error = evaluate_review_memory(&query(
            vec![first.clone(), duplicate],
            Some(HEAD_A),
            Some("#7"),
            None,
        ))
        .unwrap_err();
        assert!(duplicate_error.to_string().contains("duplicate review event_id"));

        let conflict_error = evaluate_review_memory(&query(
            vec![first, conflict],
            Some(HEAD_A),
            Some("#7"),
            None,
        ))
        .unwrap_err();
        assert!(
            conflict_error
                .to_string()
                .contains("conflicting duplicate review event_id")
        );
    }

    #[test]
    fn reviewed_revision_must_be_exact_lowercase_sha() {
        let bad = record(
            "event:1",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "#7",
            None,
            ReviewOutcome::Open,
            None,
        );
        let error = evaluate_review_memory(&query(
            vec![bad],
            Some(HEAD_A),
            Some("#7"),
            None,
        ))
        .unwrap_err();
        assert!(error.to_string().contains("exact 40-character lowercase"));
    }

    #[test]
    fn resolution_reference_matches_outcome_state() {
        let open_with_resolution = record(
            "event:1",
            HEAD_A,
            "#7",
            None,
            ReviewOutcome::Open,
            Some("github:review-comment/101"),
        );
        let resolved_without_reference = record(
            "event:2",
            HEAD_A,
            "#7",
            None,
            ReviewOutcome::Dismissed,
            None,
        );

        let open_error = evaluate_review_memory(&query(
            vec![open_with_resolution],
            Some(HEAD_A),
            Some("#7"),
            None,
        ))
        .unwrap_err();
        assert!(open_error.to_string().contains("must not carry resolution_ref"));

        let resolved_error = evaluate_review_memory(&query(
            vec![resolved_without_reference],
            Some(HEAD_A),
            Some("#7"),
            None,
        ))
        .unwrap_err();
        assert!(resolved_error.to_string().contains("requires resolution_ref"));
    }

    #[test]
    fn pr_agent_2184_fixture_refreshes_identity_without_inheriting_resolution() {
        let mut prior = record(
            "github:issue/2184:reported-review-event",
            HEAD_A,
            "fixture:pr-reported-by-2184",
            None,
            ReviewOutcome::Dismissed,
            Some("fixture:resolved-pr-agent-thread"),
        );
        prior.concern_key = "fixture:pr-agent:same-suggestion".to_string();
        prior.source_ref = "github:issue/2184".to_string();

        let evaluation = evaluate_review_memory(&ReviewMemoryQuery {
            schema_version: REVIEW_MEMORY_SCHEMA_VERSION,
            current: CurrentConcern {
                concern_key: "fixture:pr-agent:same-suggestion".to_string(),
                context: EvaluationContext {
                    repository: Some("owner/repo".to_string()),
                    revision: Some(HEAD_B.to_string()),
                    work: Some("fixture:pr-reported-by-2184".to_string()),
                    path: None,
                },
            },
            records: vec![prior],
        })
        .unwrap();

        assert_eq!(
            evaluation.disposition,
            ReviewThreadDisposition::RefreshExistingThread
        );
        assert_eq!(
            evaluation.matches[0].applicability.status,
            ApplicabilityStatus::Invalid
        );
        assert_eq!(evaluation.matches[0].source_ref, "github:issue/2184");
    }
}

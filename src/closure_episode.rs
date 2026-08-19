use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const CLOSURE_EPISODE_SCHEMA_VERSION: u32 = 1;
pub const MAX_CLOSURE_EPISODE_BYTES: usize = 256 * 1024;
const MAX_COORDINATE_BYTES: usize = 2048;
const MAX_TITLE_BYTES: usize = 1024;
const MAX_EVIDENCE_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IssueClosureEpisode {
    pub schema_version: u32,
    pub repository: String,
    pub prior: IssueSnapshot,
    pub later: IssueSnapshot,
    pub closure: ClosureReceipt,
    pub re_report: ReReportReceipt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_challenge: Option<DuplicateChallengeReceipt>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IssueSnapshot {
    pub number: u64,
    pub title: String,
    pub state: IssueState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_reason: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_by: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureReceipt {
    pub issue: u64,
    pub comment_id: u64,
    pub source_ref: String,
    pub actor: String,
    pub kind: ClosureKind,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosureKind {
    AdministrativeInactive,
    Other,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReReportReceipt {
    pub from_issue: u64,
    pub to_issue: u64,
    pub relation: ReReportRelation,
    pub source_ref: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReReportRelation {
    ReReportOf,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DuplicateChallengeReceipt {
    pub suggestion_comment_id: u64,
    pub suggestion_source_ref: String,
    pub suggestion_actor: String,
    pub suggestion_evidence: String,
    pub rejection_comment_id: u64,
    pub rejection_source_ref: String,
    pub rejection_actor: String,
    pub rejection_evidence: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClearanceStatus {
    Unknown,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosureEpisodeDisposition {
    InspectPriorFailure,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IssueClosureEvaluation {
    pub schema_version: u32,
    pub repository: String,
    pub prior_issue: u64,
    pub later_issue: u64,
    pub prior_state: IssueState,
    pub later_state: IssueState,
    pub closure_kind: ClosureKind,
    pub re_report_observed: bool,
    pub clearance: ClearanceStatus,
    pub disposition: ClosureEpisodeDisposition,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClosureEpisodeError {
    message: String,
}

impl ClosureEpisodeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ClosureEpisodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ClosureEpisodeError {}

pub fn parse_closure_episode(bytes: &[u8]) -> Result<IssueClosureEpisode, ClosureEpisodeError> {
    if bytes.len() > MAX_CLOSURE_EPISODE_BYTES {
        return Err(ClosureEpisodeError::new(format!(
            "closure episode exceeds the {MAX_CLOSURE_EPISODE_BYTES}-byte limit"
        )));
    }
    let episode: IssueClosureEpisode = serde_json::from_slice(bytes)
        .map_err(|error| ClosureEpisodeError::new(format!("invalid closure-episode JSON: {error}")))?;
    validate_episode(&episode)?;
    Ok(episode)
}

pub fn evaluate_closure_episode(
    episode: &IssueClosureEpisode,
) -> Result<IssueClosureEvaluation, ClosureEpisodeError> {
    validate_episode(episode)?;
    Ok(IssueClosureEvaluation {
        schema_version: CLOSURE_EPISODE_SCHEMA_VERSION,
        repository: episode.repository.clone(),
        prior_issue: episode.prior.number,
        later_issue: episode.later.number,
        prior_state: episode.prior.state,
        later_state: episode.later.state,
        closure_kind: episode.closure.kind,
        re_report_observed: true,
        clearance: ClearanceStatus::Unknown,
        disposition: ClosureEpisodeDisposition::InspectPriorFailure,
    })
}

fn validate_episode(episode: &IssueClosureEpisode) -> Result<(), ClosureEpisodeError> {
    if episode.schema_version != CLOSURE_EPISODE_SCHEMA_VERSION {
        return Err(ClosureEpisodeError::new(format!(
            "unsupported closure-episode schema {}; expected {CLOSURE_EPISODE_SCHEMA_VERSION}",
            episode.schema_version
        )));
    }
    validate_repository(&episode.repository)?;
    validate_issue(&episode.prior, "prior")?;
    validate_issue(&episode.later, "later")?;
    if episode.prior.number == episode.later.number {
        return Err(ClosureEpisodeError::new(
            "prior and later issue identities must differ",
        ));
    }
    if episode.prior.state != IssueState::Closed {
        return Err(ClosureEpisodeError::new(
            "closure episode requires the prior issue to be closed",
        ));
    }
    validate_closure(&episode.repository, &episode.prior, &episode.closure)?;
    validate_re_report(&episode.prior, &episode.later, &episode.re_report)?;
    if let Some(challenge) = &episode.duplicate_challenge {
        validate_duplicate_challenge(challenge)?;
    }
    Ok(())
}

fn validate_issue(issue: &IssueSnapshot, label: &str) -> Result<(), ClosureEpisodeError> {
    if issue.number == 0 {
        return Err(ClosureEpisodeError::new(format!(
            "{label} issue number must be positive"
        )));
    }
    validate_single_line(&issue.title, &format!("{label} title"), MAX_TITLE_BYTES)?;
    if let Some(reason) = &issue.state_reason {
        validate_single_line(reason, &format!("{label} state_reason"), MAX_COORDINATE_BYTES)?;
    }
    validate_timestamp(&issue.created_at, &format!("{label} created_at"))?;
    if let Some(closed_by) = &issue.closed_by {
        validate_single_line(
            closed_by,
            &format!("{label} closed_by"),
            MAX_COORDINATE_BYTES,
        )?;
    }
    match (issue.state, issue.closed_at.as_deref()) {
        (IssueState::Open, None) => {
            if issue.closed_by.is_some() {
                return Err(ClosureEpisodeError::new(format!(
                    "open {label} issue may not carry closed_by"
                )));
            }
        }
        (IssueState::Open, Some(_)) => {
            return Err(ClosureEpisodeError::new(format!(
                "open {label} issue may not carry closed_at"
            )));
        }
        (IssueState::Closed, Some(closed_at)) => {
            validate_timestamp(closed_at, &format!("{label} closed_at"))?;
        }
        (IssueState::Closed, None) => {
            return Err(ClosureEpisodeError::new(format!(
                "closed {label} issue requires closed_at"
            )));
        }
    }
    Ok(())
}

fn validate_closure(
    repository: &str,
    prior: &IssueSnapshot,
    closure: &ClosureReceipt,
) -> Result<(), ClosureEpisodeError> {
    if closure.issue != prior.number {
        return Err(ClosureEpisodeError::new(
            "closure receipt issue does not match prior issue",
        ));
    }
    if closure.comment_id == 0 {
        return Err(ClosureEpisodeError::new(
            "closure comment id must be positive",
        ));
    }
    validate_single_line(&closure.source_ref, "closure source_ref", MAX_COORDINATE_BYTES)?;
    validate_single_line(&closure.actor, "closure actor", MAX_COORDINATE_BYTES)?;
    validate_evidence(&closure.evidence, "closure evidence")?;
    if closure.kind == ClosureKind::AdministrativeInactive {
        if closure.actor != "github-actions[bot]" {
            return Err(ClosureEpisodeError::new(
                "administrative_inactive closure requires github-actions[bot] actor",
            ));
        }
        let expected = administrative_inactive_evidence(repository);
        if closure.evidence != expected {
            return Err(ClosureEpisodeError::new(
                "administrative_inactive closure evidence is not the admitted exact GitHub bot form",
            ));
        }
    }
    Ok(())
}

fn validate_re_report(
    prior: &IssueSnapshot,
    later: &IssueSnapshot,
    receipt: &ReReportReceipt,
) -> Result<(), ClosureEpisodeError> {
    if receipt.from_issue != later.number || receipt.to_issue != prior.number {
        return Err(ClosureEpisodeError::new(
            "re-report receipt endpoints do not match later/prior issues",
        ));
    }
    validate_single_line(
        &receipt.source_ref,
        "re-report source_ref",
        MAX_COORDINATE_BYTES,
    )?;
    validate_evidence(&receipt.evidence, "re-report evidence")?;
    let Some(target) = re_report_target(&receipt.evidence) else {
        return Err(ClosureEpisodeError::new(
            "re-report evidence is not an admitted exact re-report form",
        ));
    };
    if target != prior.number {
        return Err(ClosureEpisodeError::new(format!(
            "re-report evidence names issue #{target}, not prior issue #{}",
            prior.number
        )));
    }
    Ok(())
}

fn validate_duplicate_challenge(
    challenge: &DuplicateChallengeReceipt,
) -> Result<(), ClosureEpisodeError> {
    if challenge.suggestion_comment_id == 0 || challenge.rejection_comment_id == 0 {
        return Err(ClosureEpisodeError::new(
            "duplicate-challenge comment ids must be positive",
        ));
    }
    if challenge.suggestion_comment_id == challenge.rejection_comment_id {
        return Err(ClosureEpisodeError::new(
            "duplicate suggestion and rejection must be different comments",
        ));
    }
    validate_single_line(
        &challenge.suggestion_source_ref,
        "duplicate suggestion source_ref",
        MAX_COORDINATE_BYTES,
    )?;
    validate_single_line(
        &challenge.suggestion_actor,
        "duplicate suggestion actor",
        MAX_COORDINATE_BYTES,
    )?;
    validate_evidence(&challenge.suggestion_evidence, "duplicate suggestion evidence")?;
    validate_single_line(
        &challenge.rejection_source_ref,
        "duplicate rejection source_ref",
        MAX_COORDINATE_BYTES,
    )?;
    validate_single_line(
        &challenge.rejection_actor,
        "duplicate rejection actor",
        MAX_COORDINATE_BYTES,
    )?;
    validate_evidence(&challenge.rejection_evidence, "duplicate rejection evidence")?;
    Ok(())
}

fn administrative_inactive_evidence(repository: &str) -> String {
    format!(
        "Closing for now — inactive for too long. Please [open a new issue](https://github.com/{repository}/issues/new/choose) if this is still relevant."
    )
}

fn re_report_target(evidence: &str) -> Option<u64> {
    let prefix = "**Re-reporting** the bug from #";
    let rest = evidence.strip_prefix(prefix)?;
    let bytes = rest.as_bytes();
    if bytes.is_empty() || bytes[0] == b'0' || !bytes[0].is_ascii_digit() {
        return None;
    }
    let mut value = 0u64;
    let mut cursor = 0usize;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        value = value
            .checked_mul(10)?
            .checked_add((bytes[cursor] - b'0') as u64)?;
        cursor += 1;
    }
    if cursor < bytes.len()
        && !matches!(bytes[cursor], b' ' | b'(' | b'.' | b',' | b':' | b';' | b'-')
    {
        return None;
    }
    Some(value)
}

fn validate_repository(value: &str) -> Result<(), ClosureEpisodeError> {
    if value.is_empty()
        || value.len() > MAX_COORDINATE_BYTES
        || value.contains('\n')
        || value.contains('\r')
    {
        return Err(ClosureEpisodeError::new(
            "repository must be a bounded canonical owner/name coordinate",
        ));
    }
    let Some((owner, repository)) = value.split_once('/') else {
        return Err(ClosureEpisodeError::new(
            "repository must be a bounded canonical owner/name coordinate",
        ));
    };
    if owner.is_empty()
        || repository.is_empty()
        || repository.contains('/')
        || !owner.bytes().all(valid_repository_char)
        || !repository.bytes().all(valid_repository_char)
    {
        return Err(ClosureEpisodeError::new(
            "repository must be a bounded canonical owner/name coordinate",
        ));
    }
    Ok(())
}

fn valid_repository_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

fn validate_timestamp(value: &str, field: &str) -> Result<(), ClosureEpisodeError> {
    validate_single_line(value, field, 64)
}

fn validate_single_line(
    value: &str,
    field: &str,
    maximum: usize,
) -> Result<(), ClosureEpisodeError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > maximum
        || value.contains('\0')
        || value.contains('\n')
        || value.contains('\r')
    {
        return Err(ClosureEpisodeError::new(format!(
            "{field} must be a bounded non-empty single-line value"
        )));
    }
    Ok(())
}

fn validate_evidence(value: &str, field: &str) -> Result<(), ClosureEpisodeError> {
    if value.is_empty() || value.len() > MAX_EVIDENCE_BYTES || value.contains('\0') {
        return Err(ClosureEpisodeError::new(format!(
            "{field} must be bounded non-empty evidence"
        )));
    }
    Ok(())
}

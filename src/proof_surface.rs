use serde::{Deserialize, Serialize};

use crate::project_memory::{ArtifactRef, ProjectArtifact, ProjectMemoryPacket};

pub const PROOF_SURFACE_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROOF_SURFACE_BYTES: usize = 128 * 1024;
const MAX_ID_BYTES: usize = 256;
const MAX_EVIDENCE_BYTES: usize = 8 * 1024;
const MAX_MARKER_BYTES: usize = 1024;
const MAX_EVENT_BODY_BYTES: usize = 8 * 1024;
const MAX_EVENT_URL_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofArtifactKind {
    IssueConversationComment,
    PullRequestReview,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProofArtifact {
    pub url: String,
    pub id: u64,
    pub body: String,
    pub review_state: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofSurfaceClaim {
    pub schema_version: u32,
    pub repository: String,
    pub candidate_id: String,
    pub subject: ArtifactRef,
    pub behavior_evidence: String,
    pub requirement_evidence: String,
    pub behavior_marker: String,
    pub requirement_marker: String,
    pub required_artifact_kind: ProofArtifactKind,
    pub provider_event: ProviderProofArtifact,
    pub provider_body_marker: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofSurfaceStatus {
    BehaviorEvidenceMissing,
    RequirementEvidenceMissing,
    ProviderEventBodyMissing,
    ProducedArtifactUnclassifiable,
    ProofSurfaceMatched,
    ObservedProofSurfaceMismatch,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ProofSurfaceEvaluation {
    pub schema_version: u32,
    pub repository: String,
    pub candidate_id: String,
    pub subject: ArtifactRef,
    pub status: ProofSurfaceStatus,
    pub behavior_passed: bool,
    pub required_artifact_kind: ProofArtifactKind,
    pub produced_artifact_kind: Option<ProofArtifactKind>,
    pub proof_valid: bool,
    pub automatic_behavior_failure: bool,
    pub automatic_acceptance: bool,
}

pub fn parse_proof_surface_claim(bytes: &[u8]) -> Result<ProofSurfaceClaim, String> {
    if bytes.len() > MAX_PROOF_SURFACE_BYTES {
        return Err(format!(
            "proof-surface claim exceeds {} bytes",
            MAX_PROOF_SURFACE_BYTES
        ));
    }
    let claim: ProofSurfaceClaim = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid proof-surface JSON: {error}"))?;
    claim.validate_shape()?;
    Ok(claim)
}

pub fn evaluate_proof_surface(
    memory: &ProjectMemoryPacket,
    claim: &ProofSurfaceClaim,
) -> Result<ProofSurfaceEvaluation, String> {
    memory.validate()?;
    claim.validate_shape()?;
    if memory.repository != claim.repository {
        return Err(format!(
            "proof-surface repository `{}` disagrees with project-memory repository `{}`",
            claim.repository, memory.repository
        ));
    }

    let subject = artifact(memory, claim.subject)?;
    validate_exact_source_excerpt(subject, &claim.behavior_evidence, claim.subject)?;
    validate_exact_source_excerpt(subject, &claim.requirement_evidence, claim.subject)?;

    let behavior_passed = claim.behavior_evidence.contains(&claim.behavior_marker);
    let requirement_present = claim.requirement_evidence.contains(&claim.requirement_marker);
    let provider_body_present = claim.provider_event.body.contains(&claim.provider_body_marker);
    let produced_artifact_kind = classify_provider_artifact(claim);
    let proof_valid = produced_artifact_kind == Some(claim.required_artifact_kind);

    let status = if !behavior_passed {
        ProofSurfaceStatus::BehaviorEvidenceMissing
    } else if !requirement_present {
        ProofSurfaceStatus::RequirementEvidenceMissing
    } else if !provider_body_present {
        ProofSurfaceStatus::ProviderEventBodyMissing
    } else if produced_artifact_kind.is_none() {
        ProofSurfaceStatus::ProducedArtifactUnclassifiable
    } else if proof_valid {
        ProofSurfaceStatus::ProofSurfaceMatched
    } else {
        ProofSurfaceStatus::ObservedProofSurfaceMismatch
    };

    Ok(ProofSurfaceEvaluation {
        schema_version: PROOF_SURFACE_SCHEMA_VERSION,
        repository: claim.repository.clone(),
        candidate_id: claim.candidate_id.clone(),
        subject: claim.subject,
        status,
        behavior_passed,
        required_artifact_kind: claim.required_artifact_kind,
        produced_artifact_kind,
        proof_valid,
        automatic_behavior_failure: false,
        automatic_acceptance: false,
    })
}

impl ProofSurfaceClaim {
    fn validate_shape(&self) -> Result<(), String> {
        if self.schema_version != PROOF_SURFACE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported proof-surface schema version {}",
                self.schema_version
            ));
        }
        validate_repository(&self.repository)?;
        validate_single_line(&self.candidate_id, "candidate_id", MAX_ID_BYTES)?;
        if self.subject.number == 0 {
            return Err("proof-surface subject number must be positive".to_string());
        }
        validate_evidence(&self.behavior_evidence, "behavior_evidence")?;
        validate_evidence(&self.requirement_evidence, "requirement_evidence")?;
        validate_marker(&self.behavior_marker, "behavior_marker")?;
        validate_marker(&self.requirement_marker, "requirement_marker")?;
        validate_marker(&self.provider_body_marker, "provider_body_marker")?;
        let canonical_requirement = canonical_requirement_marker(self.required_artifact_kind);
        if self.requirement_marker != canonical_requirement {
            return Err(format!(
                "requirement_marker for {:?} must be `{canonical_requirement}`",
                self.required_artifact_kind
            ));
        }
        validate_provider_event(&self.provider_event)?;
        Ok(())
    }
}

fn classify_provider_artifact(claim: &ProofSurfaceClaim) -> Option<ProofArtifactKind> {
    let prefix = format!(
        "https://github.com/{}/pull/{}",
        claim.repository, claim.subject.number
    );
    if !claim.provider_event.url.starts_with(&prefix) {
        return None;
    }

    let review_suffix = format!("#pullrequestreview-{}", claim.provider_event.id);
    if claim.provider_event.url == format!("{prefix}{review_suffix}")
        && claim.provider_event.review_state.as_deref() == Some("COMMENTED")
    {
        return Some(ProofArtifactKind::PullRequestReview);
    }

    let issue_comment_suffix = format!("#issuecomment-{}", claim.provider_event.id);
    if claim.provider_event.url == format!("{prefix}{issue_comment_suffix}")
        && claim.provider_event.review_state.is_none()
    {
        return Some(ProofArtifactKind::IssueConversationComment);
    }

    None
}

fn canonical_requirement_marker(kind: ProofArtifactKind) -> &'static str {
    match kind {
        ProofArtifactKind::IssueConversationComment => "ordinary conversation comment",
        ProofArtifactKind::PullRequestReview => "PR review COMMENT",
    }
}

fn artifact(
    memory: &ProjectMemoryPacket,
    reference: ArtifactRef,
) -> Result<&ProjectArtifact, String> {
    memory
        .artifacts
        .iter()
        .find(|artifact| artifact.reference == reference)
        .ok_or_else(|| {
            format!(
                "proof-surface artifact {} is absent from project memory",
                display_ref(reference)
            )
        })
}

fn validate_exact_source_excerpt(
    artifact: &ProjectArtifact,
    evidence: &str,
    reference: ArtifactRef,
) -> Result<(), String> {
    if !artifact.evidence_text.contains(evidence) {
        return Err(format!(
            "proof-surface evidence for {} is absent from retained project-memory text",
            display_ref(reference)
        ));
    }
    Ok(())
}

fn validate_provider_event(event: &ProviderProofArtifact) -> Result<(), String> {
    if event.id == 0 {
        return Err("provider event id must be positive".to_string());
    }
    if event.url.is_empty()
        || event.url.len() > MAX_EVENT_URL_BYTES
        || event.url.contains('\0')
        || event.url.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
    {
        return Err("provider event URL is empty, malformed, or too long".to_string());
    }
    if event.body.is_empty() || event.body.len() > MAX_EVENT_BODY_BYTES || event.body.contains('\0') {
        return Err("provider event body is empty, malformed, or too long".to_string());
    }
    if let Some(review_state) = &event.review_state {
        validate_single_line(review_state, "review_state", 64)?;
    }
    Ok(())
}

fn validate_evidence(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_EVIDENCE_BYTES || value.contains('\0') {
        return Err(format!("{label} is empty, malformed, or too long"));
    }
    Ok(())
}

fn validate_marker(value: &str, label: &str) -> Result<(), String> {
    validate_single_line(value, label, MAX_MARKER_BYTES)
}

fn validate_single_line(value: &str, label: &str, maximum: usize) -> Result<(), String> {
    if value.is_empty()
        || value.len() > maximum
        || value.contains('\0')
        || value.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
    {
        return Err(format!("{label} is empty, malformed, or too long"));
    }
    Ok(())
}

fn validate_repository(value: &str) -> Result<(), String> {
    let Some((owner, repository)) = value.split_once('/') else {
        return Err("repository must be canonical owner/name".to_string());
    };
    if owner.is_empty()
        || repository.is_empty()
        || repository.contains('/')
        || !owner.bytes().all(valid_repository_char)
        || !repository.bytes().all(valid_repository_char)
    {
        return Err("repository must be canonical owner/name".to_string());
    }
    Ok(())
}

fn valid_repository_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

fn display_ref(reference: ArtifactRef) -> String {
    format!("{:?}#{}", reference.kind, reference.number)
}

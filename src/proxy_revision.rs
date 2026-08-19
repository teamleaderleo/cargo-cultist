use serde::{Deserialize, Serialize};

use crate::project_memory::{ArtifactKind, ArtifactRef, ProjectArtifact, ProjectMemoryPacket};

pub const PROXY_REVISION_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROXY_REVISION_BYTES: usize = 128 * 1024;
const MAX_ID_BYTES: usize = 256;
const MAX_MARKER_BYTES: usize = 1024;
const MAX_SOURCE_EVIDENCE_BYTES: usize = 8 * 1024;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyRevisionClaim {
    pub schema_version: u32,
    pub repository: String,
    pub candidate_id: String,
    pub semantic_axis_id: String,
    pub prior_value_ref: String,
    pub replacement_value_ref: String,
    pub predecessor: ArtifactRef,
    pub successor: ArtifactRef,
    pub shared_path: String,
    pub predecessor_source_evidence: String,
    pub successor_counterexample_evidence: String,
    pub successor_replacement_evidence: String,
    pub proxy_rule_marker: String,
    pub counterexample_marker: String,
    pub replacement_rule_marker: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyRevisionStatus {
    PredecessorUnmerged,
    SuccessorUnmerged,
    SuccessorDoesNotNamePredecessor,
    NoSharedImplementationPath,
    PriorProxyRuleMissing,
    CounterexampleMissing,
    ReplacementRuleMissing,
    ObservedProxyRevision,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ProxyRevisionEvaluation {
    pub schema_version: u32,
    pub repository: String,
    pub candidate_id: String,
    pub status: ProxyRevisionStatus,
    pub semantic_axis_id: String,
    pub prior_value_ref: String,
    pub replacement_value_ref: String,
    pub predecessor: ArtifactRef,
    pub successor: ArtifactRef,
    pub shared_path: String,
    pub automatic_generalization_authority: bool,
}

pub fn parse_proxy_revision_claim(bytes: &[u8]) -> Result<ProxyRevisionClaim, String> {
    if bytes.len() > MAX_PROXY_REVISION_BYTES {
        return Err(format!(
            "proxy-revision claim exceeds {} bytes",
            MAX_PROXY_REVISION_BYTES
        ));
    }
    let claim: ProxyRevisionClaim = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid proxy-revision JSON: {error}"))?;
    claim.validate_shape()?;
    Ok(claim)
}

pub fn evaluate_proxy_revision(
    memory: &ProjectMemoryPacket,
    claim: &ProxyRevisionClaim,
) -> Result<ProxyRevisionEvaluation, String> {
    memory.validate()?;
    claim.validate_shape()?;
    if memory.repository != claim.repository {
        return Err(format!(
            "proxy-revision repository `{}` disagrees with project-memory repository `{}`",
            claim.repository, memory.repository
        ));
    }

    let predecessor = artifact(memory, claim.predecessor)?;
    let successor = artifact(memory, claim.successor)?;
    validate_exact_source_excerpt(
        predecessor,
        &claim.predecessor_source_evidence,
        claim.predecessor,
    )?;
    validate_exact_source_excerpt(
        successor,
        &claim.successor_counterexample_evidence,
        claim.successor,
    )?;
    validate_exact_source_excerpt(
        successor,
        &claim.successor_replacement_evidence,
        claim.successor,
    )?;

    let predecessor_merged = merged(predecessor);
    let successor_merged = merged(successor);
    let predecessor_marker = format!("#{}", claim.predecessor.number);

    let status = if !predecessor_merged {
        ProxyRevisionStatus::PredecessorUnmerged
    } else if !successor_merged {
        ProxyRevisionStatus::SuccessorUnmerged
    } else if !claim
        .successor_counterexample_evidence
        .contains(&predecessor_marker)
    {
        ProxyRevisionStatus::SuccessorDoesNotNamePredecessor
    } else if !predecessor.changed_paths.contains(&claim.shared_path)
        || !successor.changed_paths.contains(&claim.shared_path)
    {
        ProxyRevisionStatus::NoSharedImplementationPath
    } else if !claim
        .predecessor_source_evidence
        .contains(&claim.proxy_rule_marker)
    {
        ProxyRevisionStatus::PriorProxyRuleMissing
    } else if !claim
        .successor_counterexample_evidence
        .contains(&claim.counterexample_marker)
    {
        ProxyRevisionStatus::CounterexampleMissing
    } else if !claim
        .successor_replacement_evidence
        .contains(&claim.replacement_rule_marker)
    {
        ProxyRevisionStatus::ReplacementRuleMissing
    } else {
        ProxyRevisionStatus::ObservedProxyRevision
    };

    Ok(ProxyRevisionEvaluation {
        schema_version: PROXY_REVISION_SCHEMA_VERSION,
        repository: claim.repository.clone(),
        candidate_id: claim.candidate_id.clone(),
        status,
        semantic_axis_id: claim.semantic_axis_id.clone(),
        prior_value_ref: claim.prior_value_ref.clone(),
        replacement_value_ref: claim.replacement_value_ref.clone(),
        predecessor: claim.predecessor,
        successor: claim.successor,
        shared_path: claim.shared_path.clone(),
        automatic_generalization_authority: false,
    })
}

impl ProxyRevisionClaim {
    fn validate_shape(&self) -> Result<(), String> {
        if self.schema_version != PROXY_REVISION_SCHEMA_VERSION {
            return Err(format!(
                "unsupported proxy-revision schema version {}",
                self.schema_version
            ));
        }
        validate_repository(&self.repository)?;
        validate_single_line(&self.candidate_id, "candidate_id", MAX_ID_BYTES)?;
        validate_single_line(&self.semantic_axis_id, "semantic_axis_id", MAX_ID_BYTES)?;
        validate_single_line(&self.prior_value_ref, "prior_value_ref", MAX_ID_BYTES)?;
        validate_single_line(
            &self.replacement_value_ref,
            "replacement_value_ref",
            MAX_ID_BYTES,
        )?;
        if self.prior_value_ref == self.replacement_value_ref {
            return Err("proxy-revision prior and replacement values must differ".to_string());
        }
        validate_pr_ref(self.predecessor, "predecessor")?;
        validate_pr_ref(self.successor, "successor")?;
        if self.predecessor == self.successor {
            return Err("proxy-revision predecessor and successor must differ".to_string());
        }
        validate_repository_path(&self.shared_path)?;
        validate_source_evidence(
            &self.predecessor_source_evidence,
            "predecessor_source_evidence",
        )?;
        validate_source_evidence(
            &self.successor_counterexample_evidence,
            "successor_counterexample_evidence",
        )?;
        validate_source_evidence(
            &self.successor_replacement_evidence,
            "successor_replacement_evidence",
        )?;
        validate_marker(&self.proxy_rule_marker, "proxy_rule_marker")?;
        validate_marker(&self.counterexample_marker, "counterexample_marker")?;
        validate_marker(&self.replacement_rule_marker, "replacement_rule_marker")?;
        Ok(())
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
                "proxy-revision artifact {} is absent from project memory",
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
            "proxy-revision evidence for {} is absent from retained project-memory text",
            display_ref(reference)
        ));
    }
    Ok(())
}

fn merged(artifact: &ProjectArtifact) -> bool {
    artifact
        .revision
        .as_ref()
        .is_some_and(|revision| revision.merged)
}

fn validate_pr_ref(reference: ArtifactRef, label: &str) -> Result<(), String> {
    if reference.kind != ArtifactKind::PullRequest || reference.number == 0 {
        return Err(format!("{label} must be a positive pull-request reference"));
    }
    Ok(())
}

fn validate_source_evidence(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_SOURCE_EVIDENCE_BYTES || value.contains('\0') {
        return Err(format!("{label} is empty, malformed, or too long"));
    }
    Ok(())
}

fn validate_marker(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > MAX_MARKER_BYTES
        || value.contains('\0')
        || value.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
    {
        return Err(format!("{label} is empty, malformed, or too long"));
    }
    Ok(())
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

fn validate_repository_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(format!("non-canonical repository-relative path `{path}`"));
    }
    Ok(())
}

fn display_ref(reference: ArtifactRef) -> String {
    let kind = match reference.kind {
        ArtifactKind::PullRequest => "pr",
        ArtifactKind::Issue => "issue",
    };
    format!("{kind}#{}", reference.number)
}

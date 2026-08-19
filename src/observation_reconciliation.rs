use serde::{Deserialize, Serialize};

use crate::project_memory::{ArtifactKind, ArtifactRef, ProjectArtifact, ProjectMemoryPacket};

pub const OBSERVATION_RECONCILIATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_OBSERVATION_RECONCILIATION_BYTES: usize = 128 * 1024;
const MAX_ID_BYTES: usize = 256;
const MAX_MARKER_BYTES: usize = 1024;
const MAX_SOURCE_EVIDENCE_BYTES: usize = 8 * 1024;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationReconciliationClaim {
    pub schema_version: u32,
    pub repository: String,
    pub candidate_id: String,
    pub predecessor: ArtifactRef,
    pub reconciler: ArtifactRef,
    pub semantic_axis_id: String,
    pub authoritative_source_ref: String,
    pub lagging_source_ref: String,
    pub authoritative_value_ref: String,
    pub lagging_value_ref: String,
    pub implementation_path: String,
    pub test_path: String,
    pub reconciler_predecessor_evidence: String,
    pub authority_evidence: String,
    pub observation_evidence: String,
    pub convergence_evidence: String,
    pub exhaustion_evidence: String,
    pub authority_marker: String,
    pub authoritative_value_marker: String,
    pub lagging_value_marker: String,
    pub convergence_marker: String,
    pub exhaustion_marker: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationReconciliationStatus {
    PredecessorUnmerged,
    ReconcilerUnmerged,
    ReconcilerDoesNotNamePredecessor,
    AuthorityRuleMissing,
    DivergentObservationMissing,
    ConvergencePolicyMissing,
    PermanentDivergenceControlMissing,
    ImplementationPathMissing,
    TestPathMissing,
    ObservedReconciliation,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporaryDisagreementDisposition {
    BoundedConvergence,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistentDisagreementDisposition {
    HardFailure,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ObservationReconciliationEvaluation {
    pub schema_version: u32,
    pub repository: String,
    pub candidate_id: String,
    pub status: ObservationReconciliationStatus,
    pub semantic_axis_id: String,
    pub predecessor: ArtifactRef,
    pub reconciler: ArtifactRef,
    pub authoritative_source_ref: String,
    pub lagging_source_ref: String,
    pub authoritative_value_ref: String,
    pub lagging_value_ref: String,
    pub implementation_path: String,
    pub test_path: String,
    pub temporary_disagreement: TemporaryDisagreementDisposition,
    pub persistent_disagreement: PersistentDisagreementDisposition,
    pub automatic_authority_change: bool,
}

pub fn parse_observation_reconciliation_claim(
    bytes: &[u8],
) -> Result<ObservationReconciliationClaim, String> {
    if bytes.len() > MAX_OBSERVATION_RECONCILIATION_BYTES {
        return Err(format!(
            "observation-reconciliation claim exceeds {} bytes",
            MAX_OBSERVATION_RECONCILIATION_BYTES
        ));
    }
    let claim: ObservationReconciliationClaim = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid observation-reconciliation JSON: {error}"))?;
    claim.validate_shape()?;
    Ok(claim)
}

pub fn evaluate_observation_reconciliation(
    memory: &ProjectMemoryPacket,
    claim: &ObservationReconciliationClaim,
) -> Result<ObservationReconciliationEvaluation, String> {
    memory.validate()?;
    claim.validate_shape()?;
    if memory.repository != claim.repository {
        return Err(format!(
            "observation-reconciliation repository `{}` disagrees with project-memory repository `{}`",
            claim.repository, memory.repository
        ));
    }

    let predecessor = artifact(memory, claim.predecessor)?;
    let reconciler = artifact(memory, claim.reconciler)?;
    for evidence in [
        &claim.reconciler_predecessor_evidence,
        &claim.authority_evidence,
        &claim.observation_evidence,
        &claim.convergence_evidence,
        &claim.exhaustion_evidence,
    ] {
        validate_exact_source_excerpt(reconciler, evidence, claim.reconciler)?;
    }

    let predecessor_marker = format!("#{}", claim.predecessor.number);
    let status = if !merged(predecessor) {
        ObservationReconciliationStatus::PredecessorUnmerged
    } else if !merged(reconciler) {
        ObservationReconciliationStatus::ReconcilerUnmerged
    } else if !claim
        .reconciler_predecessor_evidence
        .contains(&predecessor_marker)
    {
        ObservationReconciliationStatus::ReconcilerDoesNotNamePredecessor
    } else if !claim.authority_evidence.contains(&claim.authority_marker) {
        ObservationReconciliationStatus::AuthorityRuleMissing
    } else if !claim
        .observation_evidence
        .contains(&claim.authoritative_value_marker)
        || !claim.observation_evidence.contains(&claim.lagging_value_marker)
    {
        ObservationReconciliationStatus::DivergentObservationMissing
    } else if !claim.convergence_evidence.contains(&claim.convergence_marker) {
        ObservationReconciliationStatus::ConvergencePolicyMissing
    } else if !claim.exhaustion_evidence.contains(&claim.exhaustion_marker) {
        ObservationReconciliationStatus::PermanentDivergenceControlMissing
    } else if !reconciler.changed_paths.contains(&claim.implementation_path) {
        ObservationReconciliationStatus::ImplementationPathMissing
    } else if !reconciler.changed_paths.contains(&claim.test_path) {
        ObservationReconciliationStatus::TestPathMissing
    } else {
        ObservationReconciliationStatus::ObservedReconciliation
    };

    Ok(ObservationReconciliationEvaluation {
        schema_version: OBSERVATION_RECONCILIATION_SCHEMA_VERSION,
        repository: claim.repository.clone(),
        candidate_id: claim.candidate_id.clone(),
        status,
        semantic_axis_id: claim.semantic_axis_id.clone(),
        predecessor: claim.predecessor,
        reconciler: claim.reconciler,
        authoritative_source_ref: claim.authoritative_source_ref.clone(),
        lagging_source_ref: claim.lagging_source_ref.clone(),
        authoritative_value_ref: claim.authoritative_value_ref.clone(),
        lagging_value_ref: claim.lagging_value_ref.clone(),
        implementation_path: claim.implementation_path.clone(),
        test_path: claim.test_path.clone(),
        temporary_disagreement: TemporaryDisagreementDisposition::BoundedConvergence,
        persistent_disagreement: PersistentDisagreementDisposition::HardFailure,
        automatic_authority_change: false,
    })
}

impl ObservationReconciliationClaim {
    fn validate_shape(&self) -> Result<(), String> {
        if self.schema_version != OBSERVATION_RECONCILIATION_SCHEMA_VERSION {
            return Err(format!(
                "unsupported observation-reconciliation schema version {}",
                self.schema_version
            ));
        }
        validate_repository(&self.repository)?;
        validate_single_line(&self.candidate_id, "candidate_id", MAX_ID_BYTES)?;
        validate_single_line(&self.semantic_axis_id, "semantic_axis_id", MAX_ID_BYTES)?;
        validate_single_line(
            &self.authoritative_source_ref,
            "authoritative_source_ref",
            MAX_ID_BYTES,
        )?;
        validate_single_line(&self.lagging_source_ref, "lagging_source_ref", MAX_ID_BYTES)?;
        validate_single_line(
            &self.authoritative_value_ref,
            "authoritative_value_ref",
            MAX_ID_BYTES,
        )?;
        validate_single_line(&self.lagging_value_ref, "lagging_value_ref", MAX_ID_BYTES)?;
        if self.authoritative_source_ref == self.lagging_source_ref {
            return Err("authoritative and lagging observation sources must differ".to_string());
        }
        if self.authoritative_value_ref == self.lagging_value_ref {
            return Err("authoritative and lagging observation values must differ".to_string());
        }
        validate_pr_ref(self.predecessor, "predecessor")?;
        validate_pr_ref(self.reconciler, "reconciler")?;
        if self.predecessor == self.reconciler {
            return Err("observation predecessor and reconciler must differ".to_string());
        }
        validate_repository_path(&self.implementation_path)?;
        validate_repository_path(&self.test_path)?;
        for (label, evidence) in [
            (
                "reconciler_predecessor_evidence",
                &self.reconciler_predecessor_evidence,
            ),
            ("authority_evidence", &self.authority_evidence),
            ("observation_evidence", &self.observation_evidence),
            ("convergence_evidence", &self.convergence_evidence),
            ("exhaustion_evidence", &self.exhaustion_evidence),
        ] {
            validate_source_evidence(evidence, label)?;
        }
        for (label, marker) in [
            ("authority_marker", &self.authority_marker),
            ("authoritative_value_marker", &self.authoritative_value_marker),
            ("lagging_value_marker", &self.lagging_value_marker),
            ("convergence_marker", &self.convergence_marker),
            ("exhaustion_marker", &self.exhaustion_marker),
        ] {
            validate_marker(marker, label)?;
        }
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
                "observation-reconciliation artifact {} is absent from project memory",
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
            "observation-reconciliation evidence for {} is absent from retained project-memory text",
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

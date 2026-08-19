use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::project_memory::{ArtifactKind, ArtifactRef, ProjectArtifact, ProjectMemoryPacket};

pub const LESSON_PROMOTION_SCHEMA_VERSION: u32 = 1;
pub const MAX_LESSON_PROMOTION_BYTES: usize = 128 * 1024;
const MAX_CANDIDATE_ID_BYTES: usize = 256;
const MAX_DISCRIMINATOR_BYTES: usize = 256;
const MAX_MARKER_BYTES: usize = 512;
const MAX_SOURCE_EVIDENCE_BYTES: usize = 8 * 1024;
const MAX_SCOPE_BYTES: usize = 1024;
const MAX_PATH_BYTES: usize = 4096;
const MAX_REPAIRS: usize = 16;
const MAX_ADJACENT: usize = 16;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairEvidence {
    pub artifact: ArtifactRef,
    pub source_evidence: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdjacentEvidence {
    pub artifact: ArtifactRef,
    pub discriminator_id: String,
    pub value_ref: String,
    pub marker: String,
    pub source_evidence: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementKind {
    RegressionTest,
    Lint,
    CiPolicy,
    ProjectRule,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardEvidence {
    pub artifact: ArtifactRef,
    pub discriminator_id: String,
    pub value_ref: String,
    pub marker: String,
    pub source_evidence: String,
    pub enforcement_kind: EnforcementKind,
    pub enforcement_path: String,
    pub scope_ref: String,
    pub covered_repairs: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LessonPromotionClaim {
    pub schema_version: u32,
    pub repository: String,
    pub candidate_id: String,
    pub candidate_discriminator_id: String,
    pub candidate_value_ref: String,
    pub repair_marker: String,
    pub repair_evidence: Vec<RepairEvidence>,
    #[serde(default)]
    pub adjacent_predecessors: Vec<AdjacentEvidence>,
    pub guard: GuardEvidence,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionStatus {
    InsufficientRepeatedRepairs,
    GuardClassMismatch,
    GuardCoverageIncludesDifferentClass,
    GuardCoverageIncomplete,
    ProposedGuard,
    ObservedPromotion,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct LessonPromotionEvaluation {
    pub schema_version: u32,
    pub repository: String,
    pub candidate_id: String,
    pub status: PromotionStatus,
    pub candidate_discriminator_id: String,
    pub candidate_value_ref: String,
    pub same_class_repairs: Vec<ArtifactRef>,
    pub adjacent_different_class: Vec<ArtifactRef>,
    pub guard: ArtifactRef,
    pub enforcement_kind: EnforcementKind,
    pub enforcement_path: String,
    pub scope_ref: String,
    pub missing_repair_coverage: Vec<ArtifactRef>,
    pub different_class_coverage: Vec<ArtifactRef>,
    pub automatic_policy_authority: bool,
}

pub fn parse_lesson_promotion_claim(bytes: &[u8]) -> Result<LessonPromotionClaim, String> {
    if bytes.len() > MAX_LESSON_PROMOTION_BYTES {
        return Err(format!(
            "lesson-promotion claim exceeds {} bytes",
            MAX_LESSON_PROMOTION_BYTES
        ));
    }
    let claim: LessonPromotionClaim = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid lesson-promotion JSON: {error}"))?;
    claim.validate_shape()?;
    Ok(claim)
}

pub fn evaluate_lesson_promotion(
    memory: &ProjectMemoryPacket,
    claim: &LessonPromotionClaim,
) -> Result<LessonPromotionEvaluation, String> {
    memory.validate()?;
    claim.validate_against(memory)?;

    let same_class_repairs = sorted_refs(claim.repair_evidence.iter().map(|item| item.artifact));
    let adjacent_different_class =
        sorted_refs(claim.adjacent_predecessors.iter().map(|item| item.artifact));
    let covered = claim
        .guard
        .covered_repairs
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let repair_set = same_class_repairs.iter().copied().collect::<BTreeSet<_>>();
    let adjacent_set = adjacent_different_class
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    let missing_repair_coverage = repair_set.difference(&covered).copied().collect::<Vec<_>>();
    let different_class_coverage = covered
        .intersection(&adjacent_set)
        .copied()
        .collect::<Vec<_>>();

    let guard = artifact(memory, claim.guard.artifact)?;
    let guard_merged = guard
        .revision
        .as_ref()
        .is_some_and(|revision| revision.merged);

    let status = if same_class_repairs.len() < 2 {
        PromotionStatus::InsufficientRepeatedRepairs
    } else if claim.guard.discriminator_id != claim.candidate_discriminator_id
        || claim.guard.value_ref != claim.candidate_value_ref
    {
        PromotionStatus::GuardClassMismatch
    } else if !different_class_coverage.is_empty() {
        PromotionStatus::GuardCoverageIncludesDifferentClass
    } else if !missing_repair_coverage.is_empty() {
        PromotionStatus::GuardCoverageIncomplete
    } else if !guard_merged {
        PromotionStatus::ProposedGuard
    } else {
        PromotionStatus::ObservedPromotion
    };

    Ok(LessonPromotionEvaluation {
        schema_version: LESSON_PROMOTION_SCHEMA_VERSION,
        repository: claim.repository.clone(),
        candidate_id: claim.candidate_id.clone(),
        status,
        candidate_discriminator_id: claim.candidate_discriminator_id.clone(),
        candidate_value_ref: claim.candidate_value_ref.clone(),
        same_class_repairs,
        adjacent_different_class,
        guard: claim.guard.artifact,
        enforcement_kind: claim.guard.enforcement_kind,
        enforcement_path: claim.guard.enforcement_path.clone(),
        scope_ref: claim.guard.scope_ref.clone(),
        missing_repair_coverage,
        different_class_coverage,
        automatic_policy_authority: false,
    })
}

impl LessonPromotionClaim {
    fn validate_shape(&self) -> Result<(), String> {
        if self.schema_version != LESSON_PROMOTION_SCHEMA_VERSION {
            return Err(format!(
                "unsupported lesson-promotion schema version {}",
                self.schema_version
            ));
        }
        validate_repository(&self.repository)?;
        validate_bounded_single_line(&self.candidate_id, "candidate_id", MAX_CANDIDATE_ID_BYTES)?;
        validate_bounded_single_line(
            &self.candidate_discriminator_id,
            "candidate_discriminator_id",
            MAX_DISCRIMINATOR_BYTES,
        )?;
        validate_bounded_single_line(
            &self.candidate_value_ref,
            "candidate_value_ref",
            MAX_DISCRIMINATOR_BYTES,
        )?;
        validate_bounded_single_line(&self.repair_marker, "repair_marker", MAX_MARKER_BYTES)?;
        if self.repair_evidence.is_empty() || self.repair_evidence.len() > MAX_REPAIRS {
            return Err(format!(
                "lesson-promotion claim must contain 1..={MAX_REPAIRS} repair receipts"
            ));
        }
        if self.adjacent_predecessors.len() > MAX_ADJACENT {
            return Err(format!(
                "lesson-promotion claim may contain at most {MAX_ADJACENT} adjacent predecessors"
            ));
        }

        let mut role_refs = BTreeSet::new();
        for repair in &self.repair_evidence {
            validate_pr_ref(repair.artifact, "repair artifact")?;
            validate_source_evidence(&repair.source_evidence)?;
            if !role_refs.insert(repair.artifact) {
                return Err(format!(
                    "duplicate lesson-promotion artifact {}",
                    display_ref(repair.artifact)
                ));
            }
        }
        for adjacent in &self.adjacent_predecessors {
            validate_pr_ref(adjacent.artifact, "adjacent predecessor")?;
            validate_bounded_single_line(
                &adjacent.discriminator_id,
                "adjacent discriminator_id",
                MAX_DISCRIMINATOR_BYTES,
            )?;
            validate_bounded_single_line(
                &adjacent.value_ref,
                "adjacent value_ref",
                MAX_DISCRIMINATOR_BYTES,
            )?;
            validate_bounded_single_line(&adjacent.marker, "adjacent marker", MAX_MARKER_BYTES)?;
            validate_source_evidence(&adjacent.source_evidence)?;
            if adjacent.discriminator_id == self.candidate_discriminator_id
                && adjacent.value_ref == self.candidate_value_ref
            {
                return Err(format!(
                    "adjacent predecessor {} cannot carry the candidate discriminator value",
                    display_ref(adjacent.artifact)
                ));
            }
            if !role_refs.insert(adjacent.artifact) {
                return Err(format!(
                    "duplicate lesson-promotion artifact {}",
                    display_ref(adjacent.artifact)
                ));
            }
        }

        validate_pr_ref(self.guard.artifact, "guard artifact")?;
        validate_bounded_single_line(
            &self.guard.discriminator_id,
            "guard discriminator_id",
            MAX_DISCRIMINATOR_BYTES,
        )?;
        validate_bounded_single_line(
            &self.guard.value_ref,
            "guard value_ref",
            MAX_DISCRIMINATOR_BYTES,
        )?;
        validate_bounded_single_line(&self.guard.marker, "guard marker", MAX_MARKER_BYTES)?;
        validate_source_evidence(&self.guard.source_evidence)?;
        validate_repository_path(&self.guard.enforcement_path)?;
        validate_bounded_single_line(&self.guard.scope_ref, "guard scope_ref", MAX_SCOPE_BYTES)?;
        if !role_refs.insert(self.guard.artifact) {
            return Err(format!(
                "guard artifact {} is also used as a predecessor",
                display_ref(self.guard.artifact)
            ));
        }

        let mut coverage = BTreeSet::new();
        for covered in &self.guard.covered_repairs {
            validate_pr_ref(*covered, "covered repair")?;
            if !coverage.insert(*covered) {
                return Err(format!(
                    "duplicate covered repair {}",
                    display_ref(*covered)
                ));
            }
        }
        Ok(())
    }

    fn validate_against(&self, memory: &ProjectMemoryPacket) -> Result<(), String> {
        self.validate_shape()?;
        if memory.repository != self.repository {
            return Err(format!(
                "lesson-promotion repository `{}` disagrees with project-memory repository `{}`",
                self.repository, memory.repository
            ));
        }

        for repair in &self.repair_evidence {
            let artifact = artifact(memory, repair.artifact)?;
            validate_merged_predecessor(artifact, repair.artifact)?;
            validate_exact_source_excerpt(artifact, &repair.source_evidence, repair.artifact)?;
            if !repair.source_evidence.contains(&self.repair_marker) {
                return Err(format!(
                    "repair evidence for {} does not contain candidate marker `{}`",
                    display_ref(repair.artifact),
                    self.repair_marker
                ));
            }
        }

        for adjacent in &self.adjacent_predecessors {
            let artifact = artifact(memory, adjacent.artifact)?;
            validate_merged_predecessor(artifact, adjacent.artifact)?;
            validate_exact_source_excerpt(artifact, &adjacent.source_evidence, adjacent.artifact)?;
            if !adjacent.source_evidence.contains(&adjacent.marker) {
                return Err(format!(
                    "adjacent evidence for {} does not contain marker `{}`",
                    display_ref(adjacent.artifact),
                    adjacent.marker
                ));
            }
        }

        let guard = artifact(memory, self.guard.artifact)?;
        validate_exact_source_excerpt(guard, &self.guard.source_evidence, self.guard.artifact)?;
        if !self.guard.source_evidence.contains(&self.guard.marker) {
            return Err(format!(
                "guard evidence for {} does not contain marker `{}`",
                display_ref(self.guard.artifact),
                self.guard.marker
            ));
        }
        if !guard.changed_paths.contains(&self.guard.enforcement_path) {
            return Err(format!(
                "guard enforcement path `{}` is absent from {} changed paths",
                self.guard.enforcement_path,
                display_ref(self.guard.artifact)
            ));
        }

        let predecessor_refs = self
            .repair_evidence
            .iter()
            .map(|item| item.artifact)
            .chain(self.adjacent_predecessors.iter().map(|item| item.artifact))
            .collect::<BTreeSet<_>>();
        for covered in &self.guard.covered_repairs {
            if !predecessor_refs.contains(covered) {
                return Err(format!(
                    "guard coverage references {} outside the admitted predecessor set",
                    display_ref(*covered)
                ));
            }
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
                "lesson-promotion artifact {} is absent from project memory",
                display_ref(reference)
            )
        })
}

fn validate_merged_predecessor(
    artifact: &ProjectArtifact,
    reference: ArtifactRef,
) -> Result<(), String> {
    if artifact.reference.kind != ArtifactKind::PullRequest {
        return Err(format!(
            "lesson-promotion predecessor {} must be a pull request",
            display_ref(reference)
        ));
    }
    if !artifact
        .revision
        .as_ref()
        .is_some_and(|revision| revision.merged)
    {
        return Err(format!(
            "lesson-promotion predecessor {} must be merged",
            display_ref(reference)
        ));
    }
    Ok(())
}

fn validate_exact_source_excerpt(
    artifact: &ProjectArtifact,
    evidence: &str,
    reference: ArtifactRef,
) -> Result<(), String> {
    if !artifact.evidence_text.contains(evidence) {
        return Err(format!(
            "lesson-promotion evidence for {} is absent from retained project-memory text",
            display_ref(reference)
        ));
    }
    Ok(())
}

fn validate_pr_ref(reference: ArtifactRef, label: &str) -> Result<(), String> {
    if reference.kind != ArtifactKind::PullRequest || reference.number == 0 {
        return Err(format!("{label} must be a positive pull-request reference"));
    }
    Ok(())
}

fn validate_source_evidence(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_SOURCE_EVIDENCE_BYTES || value.contains('\0') {
        return Err("source_evidence is empty, malformed, or too long".to_string());
    }
    Ok(())
}

fn validate_bounded_single_line(value: &str, label: &str, maximum: usize) -> Result<(), String> {
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

fn sorted_refs(values: impl Iterator<Item = ArtifactRef>) -> Vec<ArtifactRef> {
    values.collect::<BTreeSet<_>>().into_iter().collect()
}

fn display_ref(reference: ArtifactRef) -> String {
    let kind = match reference.kind {
        ArtifactKind::PullRequest => "pr",
        ArtifactKind::Issue => "issue",
    };
    format!("{kind}#{}", reference.number)
}

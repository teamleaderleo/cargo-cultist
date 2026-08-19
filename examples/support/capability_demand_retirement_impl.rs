use serde::{Deserialize, Serialize};

pub const RUN_RECEIPT_SCHEMA_VERSION: u32 = 1;
pub const PAIR_EVALUATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDigest {
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerVisibleCommon {
    pub task: ArtifactDigest,
    pub patch: ArtifactDigest,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorOnly {
    pub oracle: ArtifactDigest,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionManifest {
    pub packet: ArtifactDigest,
    pub decisive_evidence_present: bool,
    #[serde(default)]
    pub decisive_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionManifests {
    pub file_local_jei: ConditionManifest,
    pub scoped_jei: ConditionManifest,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialInputManifest {
    pub schema_version: u32,
    pub trial_id: String,
    pub repository: String,
    pub revision: String,
    pub target_path: String,
    pub target_blob_sha: String,
    pub worker_visible_common: WorkerVisibleCommon,
    pub evaluator_only: EvaluatorOnly,
    pub conditions: ConditionManifests,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionId {
    FileLocalJei,
    ScopedJei,
}

impl ConditionId {
    fn manifest<'a>(self, manifest: &'a TrialInputManifest) -> &'a ConditionManifest {
        match self {
            Self::FileLocalJei => &manifest.conditions.file_local_jei,
            Self::ScopedJei => &manifest.conditions.scoped_jei,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Success,
    Failed,
    CorrectEscalation,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayIdentity {
    pub repository: String,
    pub repository_revision: String,
    pub target_path: String,
    pub target_blob_sha: String,
    pub task_sha256: String,
    pub patch_sha256: String,
    pub worker_identity: String,
    pub harness_identity: String,
    pub affordance_identity: String,
    pub sampling_config_fingerprint: String,
    pub completion_contract_sha256: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReceipt {
    pub schema_version: u32,
    pub trial_id: String,
    pub pair_id: String,
    pub replicate_id: String,
    pub condition_id: ConditionId,
    pub condition_order: u32,
    pub session_id: String,
    pub fresh_session: bool,
    pub prior_condition_exposure: bool,
    pub packet_sha256: String,
    pub decisive_evidence_present: bool,
    pub identity: ReplayIdentity,
    pub outcome: RunOutcome,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementVerdict {
    PairedRetirementSignal,
    CorrectEscalationThenSuccess,
    NoDemandObserved,
    DemandPersists,
    Confounded,
    InvalidEvidencePair,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct PairEvaluation {
    pub schema_version: u32,
    pub trial_id: String,
    pub pair_id: String,
    pub replicate_id: String,
    pub verdict: RetirementVerdict,
}

fn identity_matches_manifest(identity: &ReplayIdentity, manifest: &TrialInputManifest) -> bool {
    identity.repository == manifest.repository
        && identity.repository_revision == manifest.revision
        && identity.target_path == manifest.target_path
        && identity.target_blob_sha == manifest.target_blob_sha
        && identity.task_sha256 == manifest.worker_visible_common.task.sha256
        && identity.patch_sha256 == manifest.worker_visible_common.patch.sha256
        && identity.completion_contract_sha256 == manifest.evaluator_only.oracle.sha256
}

fn condition_matches_manifest(receipt: &RunReceipt, manifest: &TrialInputManifest) -> bool {
    let expected = receipt.condition_id.manifest(manifest);
    receipt.packet_sha256 == expected.packet.sha256
        && receipt.decisive_evidence_present == expected.decisive_evidence_present
}

fn common_run_identity_matches(left: &RunReceipt, right: &RunReceipt) -> bool {
    left.identity == right.identity
        && left.trial_id == right.trial_id
        && left.pair_id == right.pair_id
        && left.replicate_id == right.replicate_id
}

fn sessions_are_isolated(left: &RunReceipt, right: &RunReceipt) -> bool {
    left.fresh_session
        && right.fresh_session
        && !left.prior_condition_exposure
        && !right.prior_condition_exposure
        && !left.session_id.trim().is_empty()
        && !right.session_id.trim().is_empty()
        && left.session_id != right.session_id
}

fn condition_order_is_valid(left: &RunReceipt, right: &RunReceipt) -> bool {
    matches!((left.condition_order, right.condition_order), (0, 1) | (1, 0))
}

fn select_conditions<'a>(
    left: &'a RunReceipt,
    right: &'a RunReceipt,
) -> Option<(&'a RunReceipt, &'a RunReceipt)> {
    match (left.condition_id, right.condition_id) {
        (ConditionId::FileLocalJei, ConditionId::ScopedJei) => Some((left, right)),
        (ConditionId::ScopedJei, ConditionId::FileLocalJei) => Some((right, left)),
        _ => None,
    }
}

pub fn evaluate_pair(
    manifest: &TrialInputManifest,
    left: &RunReceipt,
    right: &RunReceipt,
) -> PairEvaluation {
    let base = PairEvaluation {
        schema_version: PAIR_EVALUATION_SCHEMA_VERSION,
        trial_id: left.trial_id.clone(),
        pair_id: left.pair_id.clone(),
        replicate_id: left.replicate_id.clone(),
        verdict: RetirementVerdict::Confounded,
    };

    if manifest.schema_version != 1
        || left.schema_version != RUN_RECEIPT_SCHEMA_VERSION
        || right.schema_version != RUN_RECEIPT_SCHEMA_VERSION
        || manifest.trial_id != left.trial_id
        || manifest.trial_id != right.trial_id
        || !common_run_identity_matches(left, right)
        || !identity_matches_manifest(&left.identity, manifest)
        || !identity_matches_manifest(&right.identity, manifest)
        || !condition_matches_manifest(left, manifest)
        || !condition_matches_manifest(right, manifest)
        || !sessions_are_isolated(left, right)
        || !condition_order_is_valid(left, right)
    {
        return base;
    }

    let Some((baseline, treatment)) = select_conditions(left, right) else {
        return PairEvaluation {
            verdict: RetirementVerdict::InvalidEvidencePair,
            ..base
        };
    };

    if baseline.decisive_evidence_present || !treatment.decisive_evidence_present {
        return PairEvaluation {
            verdict: RetirementVerdict::InvalidEvidencePair,
            ..base
        };
    }

    let verdict = match (baseline.outcome, treatment.outcome) {
        (RunOutcome::Failed, RunOutcome::Success) => RetirementVerdict::PairedRetirementSignal,
        (RunOutcome::CorrectEscalation, RunOutcome::Success) => {
            RetirementVerdict::CorrectEscalationThenSuccess
        }
        (RunOutcome::Success, _) => RetirementVerdict::NoDemandObserved,
        _ => RetirementVerdict::DemandPersists,
    };

    PairEvaluation { verdict, ..base }
}

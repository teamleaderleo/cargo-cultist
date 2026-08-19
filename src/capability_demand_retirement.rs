use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_TRIAL_SPEC_BYTES: usize = 128 * 1024;
pub const MAX_TRIAL_MANIFEST_BYTES: usize = 128 * 1024;
pub const MAX_RUN_RECEIPT_BYTES: usize = 128 * 1024;

const MAX_ID_BYTES: usize = 256;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialSpec {
    pub schema_version: u32,
    #[serde(skip)]
    pub source_sha256: String,
    pub trial_id: String,
    pub repository: String,
    pub revision: String,
    pub target_path: String,
    pub target_blob_sha: String,
    pub worker_task: WorkerTask,
    pub oracle: Oracle,
    pub conditions: Vec<TrialCondition>,
    pub oracle_leak_control: OracleLeakControl,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerTask {
    pub prompt: String,
    pub patch: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Oracle {
    pub expected_disposition: String,
    pub blocking_reason: String,
    pub max_identifier_length: usize,
    pub proposed_identifier: String,
    pub proposed_identifier_length: usize,
    pub corrective_action: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacketKind {
    FileLocal,
    Scoped,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialCondition {
    pub id: String,
    pub packet_kind: PacketKind,
    pub budget_bytes: usize,
    pub scope: Option<String>,
    pub decisive_evidence_present: bool,
    pub decisive_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleLeakControl {
    pub historical_issue: String,
    pub allowed_as_worker_prompt: bool,
    pub prohibited_worker_prompt_fragments: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDigest {
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialInputManifest {
    pub schema_version: u32,
    pub trial_spec_sha256: String,
    pub trial_id: String,
    pub repository: String,
    pub revision: String,
    pub target_path: String,
    pub target_blob_sha: String,
    pub worker_visible_common: WorkerVisibleCommon,
    pub evaluator_only: EvaluatorOnly,
    pub conditions: BTreeMap<String, ManifestCondition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerVisibleCommon {
    pub task: ArtifactDigest,
    pub patch: ArtifactDigest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorOnly {
    pub oracle: ArtifactDigest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestCondition {
    pub packet: ArtifactDigest,
    pub packet_kind: PacketKind,
    pub budget_bytes: usize,
    pub scope: Option<String>,
    pub decisive_evidence_present: bool,
    #[serde(default)]
    pub decisive_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Success,
    Failed,
    CorrectEscalation,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceInspection {
    Consulted,
    NotConsulted,
    Unobservable,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerRunReceipt {
    pub schema_version: u32,
    pub trial_id: String,
    pub trial_spec_sha256: String,
    pub pair_id: String,
    pub run_id: String,
    pub condition_id: String,
    pub sequence_index: u32,
    pub repository: String,
    pub revision: String,
    pub target_path: String,
    pub target_blob_sha: String,
    pub task_sha256: String,
    pub patch_sha256: String,
    pub evidence_packet_sha256: String,
    pub completion_contract_sha256: String,
    pub worker_identity: String,
    pub harness_identity: String,
    pub affordance_identity: String,
    pub sampling_config_sha256: String,
    pub session_id: String,
    pub fresh_session: bool,
    pub prior_condition_exposure: bool,
    pub checkout_reset_receipt_sha256: String,
    pub worker_output_sha256: String,
    pub evaluated_outcome: RunOutcome,
    pub evidence_inspection: EvidenceInspection,
    pub context_expanded: bool,
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

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PairEvaluation {
    pub schema_version: u32,
    pub trial_id: String,
    pub pair_id: String,
    pub condition_ids: Vec<String>,
    pub verdict: RetirementVerdict,
    pub frozen_identity_match: bool,
    pub fresh_uncontaminated_sessions: bool,
    pub decisive_evidence_flip: bool,
    pub baseline_condition_id: Option<String>,
    pub treatment_condition_id: Option<String>,
    pub baseline_outcome: Option<RunOutcome>,
    pub treatment_outcome: Option<RunOutcome>,
    pub baseline_evidence_inspection: Option<EvidenceInspection>,
    pub treatment_evidence_inspection: Option<EvidenceInspection>,
    pub baseline_context_expanded: Option<bool>,
    pub treatment_context_expanded: Option<bool>,
    pub automatic_causal_claim: bool,
    pub automatic_generalization: bool,
}

pub fn parse_trial_spec(input: &[u8]) -> Result<TrialSpec, String> {
    ensure_bounded(input, MAX_TRIAL_SPEC_BYTES, "trial spec")?;
    let mut spec: TrialSpec = serde_json::from_slice(input).map_err(|error| error.to_string())?;
    validate_trial_spec(&spec)?;
    spec.source_sha256 = sha256_hex(input);
    Ok(spec)
}

pub fn parse_trial_manifest(input: &[u8]) -> Result<TrialInputManifest, String> {
    ensure_bounded(input, MAX_TRIAL_MANIFEST_BYTES, "trial input manifest")?;
    let manifest: TrialInputManifest =
        serde_json::from_slice(input).map_err(|error| error.to_string())?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn parse_run_receipt(input: &[u8]) -> Result<WorkerRunReceipt, String> {
    ensure_bounded(input, MAX_RUN_RECEIPT_BYTES, "worker run receipt")?;
    let receipt: WorkerRunReceipt =
        serde_json::from_slice(input).map_err(|error| error.to_string())?;
    validate_run_receipt(&receipt)?;
    Ok(receipt)
}

pub fn evaluate_pair(
    spec: &TrialSpec,
    manifest: &TrialInputManifest,
    first: &WorkerRunReceipt,
    second: &WorkerRunReceipt,
) -> Result<PairEvaluation, String> {
    validate_manifest_against_spec(spec, manifest)?;
    validate_receipt_against_manifest(first, manifest)?;
    validate_receipt_against_manifest(second, manifest)?;

    let first_condition = manifest
        .conditions
        .get(&first.condition_id)
        .ok_or_else(|| format!("unknown first condition {}", first.condition_id))?;
    let second_condition = manifest
        .conditions
        .get(&second.condition_id)
        .ok_or_else(|| format!("unknown second condition {}", second.condition_id))?;

    let decisive_evidence_flip = first.condition_id != second.condition_id
        && first_condition.decisive_evidence_present != second_condition.decisive_evidence_present;

    let (baseline, treatment) = if decisive_evidence_flip {
        if first_condition.decisive_evidence_present {
            (second, first)
        } else {
            (first, second)
        }
    } else {
        (first, second)
    };

    let frozen_identity_match = same_frozen_identity(first, second);
    let fresh_uncontaminated_sessions = first.fresh_session
        && second.fresh_session
        && !first.prior_condition_exposure
        && !second.prior_condition_exposure
        && first.session_id != second.session_id
        && first.run_id != second.run_id
        && first.sequence_index != second.sequence_index
        && BTreeSet::from([first.sequence_index, second.sequence_index]) == BTreeSet::from([1, 2]);

    let verdict = if !frozen_identity_match || !fresh_uncontaminated_sessions {
        RetirementVerdict::Confounded
    } else if !decisive_evidence_flip {
        RetirementVerdict::InvalidEvidencePair
    } else {
        match (baseline.evaluated_outcome, treatment.evaluated_outcome) {
            (RunOutcome::Failed, RunOutcome::Success) => RetirementVerdict::PairedRetirementSignal,
            (RunOutcome::CorrectEscalation, RunOutcome::Success) => {
                RetirementVerdict::CorrectEscalationThenSuccess
            }
            (RunOutcome::Success, _) => RetirementVerdict::NoDemandObserved,
            _ => RetirementVerdict::DemandPersists,
        }
    };

    let has_roles = decisive_evidence_flip;
    Ok(PairEvaluation {
        schema_version: 1,
        trial_id: manifest.trial_id.clone(),
        pair_id: first.pair_id.clone(),
        condition_ids: vec![first.condition_id.clone(), second.condition_id.clone()],
        verdict,
        frozen_identity_match,
        fresh_uncontaminated_sessions,
        decisive_evidence_flip,
        baseline_condition_id: has_roles.then(|| baseline.condition_id.clone()),
        treatment_condition_id: has_roles.then(|| treatment.condition_id.clone()),
        baseline_outcome: has_roles.then_some(baseline.evaluated_outcome),
        treatment_outcome: has_roles.then_some(treatment.evaluated_outcome),
        baseline_evidence_inspection: has_roles.then_some(baseline.evidence_inspection),
        treatment_evidence_inspection: has_roles.then_some(treatment.evidence_inspection),
        baseline_context_expanded: has_roles.then_some(baseline.context_expanded),
        treatment_context_expanded: has_roles.then_some(treatment.context_expanded),
        automatic_causal_claim: false,
        automatic_generalization: false,
    })
}

fn validate_trial_spec(spec: &TrialSpec) -> Result<(), String> {
    if spec.schema_version != 1 {
        return Err(format!(
            "unsupported trial schema version {}",
            spec.schema_version
        ));
    }
    validate_id(&spec.trial_id, "trial_id")?;
    validate_id(&spec.repository, "repository")?;
    validate_git_sha(&spec.revision, "revision")?;
    validate_path(&spec.target_path, "target_path")?;
    validate_git_sha(&spec.target_blob_sha, "target_blob_sha")?;
    if spec.worker_task.prompt.is_empty() || spec.worker_task.patch.is_empty() {
        return Err("worker task prompt and patch must be non-empty".into());
    }
    if spec.conditions.len() < 2 || spec.conditions.len() > 32 {
        return Err("trial must contain 2..32 conditions".into());
    }
    let mut ids = BTreeSet::new();
    for condition in &spec.conditions {
        validate_id(&condition.id, "condition id")?;
        if !ids.insert(condition.id.as_str()) {
            return Err(format!("duplicate trial condition {}", condition.id));
        }
        validate_condition_recipe(
            &condition.id,
            condition.packet_kind,
            condition.budget_bytes,
            condition.scope.as_deref(),
        )?;
        if condition.decisive_evidence_present && condition.decisive_evidence_refs.is_empty() {
            return Err(format!(
                "condition {} marks decisive evidence present without refs",
                condition.id
            ));
        }
        if !condition.decisive_evidence_present && !condition.decisive_evidence_refs.is_empty() {
            return Err(format!(
                "condition {} marks decisive evidence absent but retains refs",
                condition.id
            ));
        }
        for reference in &condition.decisive_evidence_refs {
            validate_git_sha(reference, "decisive evidence ref")?;
        }
    }
    Ok(())
}

fn validate_manifest(manifest: &TrialInputManifest) -> Result<(), String> {
    if manifest.schema_version != 1 {
        return Err(format!(
            "unsupported trial input manifest schema version {}",
            manifest.schema_version
        ));
    }
    validate_id(&manifest.trial_id, "trial_id")?;
    validate_sha256(&manifest.trial_spec_sha256, "trial_spec_sha256")?;
    validate_id(&manifest.repository, "repository")?;
    validate_git_sha(&manifest.revision, "revision")?;
    validate_path(&manifest.target_path, "target_path")?;
    validate_git_sha(&manifest.target_blob_sha, "target_blob_sha")?;
    validate_artifact_digest(&manifest.worker_visible_common.task, "task")?;
    validate_artifact_digest(&manifest.worker_visible_common.patch, "patch")?;
    validate_artifact_digest(&manifest.evaluator_only.oracle, "oracle")?;
    if manifest.conditions.len() < 2 || manifest.conditions.len() > 32 {
        return Err("trial input manifest must contain 2..32 conditions".into());
    }
    for (id, condition) in &manifest.conditions {
        validate_id(id, "condition id")?;
        validate_artifact_digest(&condition.packet, "condition packet")?;
        validate_condition_recipe(
            id,
            condition.packet_kind,
            condition.budget_bytes,
            condition.scope.as_deref(),
        )?;
        if condition.decisive_evidence_present && condition.decisive_evidence_refs.is_empty() {
            return Err(format!(
                "manifest condition {id} marks decisive evidence present without refs"
            ));
        }
        if !condition.decisive_evidence_present && !condition.decisive_evidence_refs.is_empty() {
            return Err(format!(
                "manifest condition {id} marks decisive evidence absent but retains refs"
            ));
        }
        for reference in &condition.decisive_evidence_refs {
            validate_git_sha(reference, "decisive evidence ref")?;
        }
    }
    Ok(())
}

fn validate_manifest_against_spec(
    spec: &TrialSpec,
    manifest: &TrialInputManifest,
) -> Result<(), String> {
    if spec.trial_id != manifest.trial_id
        || spec.repository != manifest.repository
        || spec.revision != manifest.revision
        || spec.target_path != manifest.target_path
        || spec.target_blob_sha != manifest.target_blob_sha
    {
        return Err("trial input manifest does not match frozen trial identity".into());
    }

    if manifest.trial_spec_sha256 != spec.source_sha256 {
        return Err("trial input manifest does not match exact frozen trial-spec bytes".into());
    }

    let expected_task = line_artifact_digest(&spec.worker_task.prompt);
    let expected_patch = line_artifact_digest(&spec.worker_task.patch);
    let expected_oracle = oracle_artifact_digest(&spec.oracle)?;
    if manifest.worker_visible_common.task != expected_task {
        return Err("task artifact does not match frozen trial spec".into());
    }
    if manifest.worker_visible_common.patch != expected_patch {
        return Err("patch artifact does not match frozen trial spec".into());
    }
    if manifest.evaluator_only.oracle != expected_oracle {
        return Err("oracle artifact does not match frozen trial spec".into());
    }

    let spec_conditions = spec
        .conditions
        .iter()
        .map(|condition| (condition.id.as_str(), condition))
        .collect::<BTreeMap<_, _>>();
    if spec_conditions.len() != manifest.conditions.len() {
        return Err("trial condition set differs between spec and manifest".into());
    }
    for (id, condition) in &manifest.conditions {
        let expected = spec_conditions
            .get(id.as_str())
            .ok_or_else(|| format!("manifest contains undeclared condition {id}"))?;
        if expected.packet_kind != condition.packet_kind
            || expected.budget_bytes != condition.budget_bytes
            || expected.scope != condition.scope
        {
            return Err(format!("condition {id} materialization recipe drifted"));
        }
        if expected.decisive_evidence_present != condition.decisive_evidence_present {
            return Err(format!("condition {id} decisive-evidence state drifted"));
        }
        let expected_refs = expected
            .decisive_evidence_refs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let manifest_refs = condition
            .decisive_evidence_refs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if expected_refs != manifest_refs {
            return Err(format!("condition {id} decisive-evidence refs drifted"));
        }
    }
    Ok(())
}

fn validate_run_receipt(receipt: &WorkerRunReceipt) -> Result<(), String> {
    if receipt.schema_version != 1 {
        return Err(format!(
            "unsupported worker run receipt schema version {}",
            receipt.schema_version
        ));
    }
    for (value, field) in [
        (&receipt.trial_id, "trial_id"),
        (&receipt.pair_id, "pair_id"),
        (&receipt.run_id, "run_id"),
        (&receipt.condition_id, "condition_id"),
        (&receipt.repository, "repository"),
        (&receipt.worker_identity, "worker_identity"),
        (&receipt.harness_identity, "harness_identity"),
        (&receipt.affordance_identity, "affordance_identity"),
        (&receipt.session_id, "session_id"),
    ] {
        validate_id(value, field)?;
    }
    if !(1..=2).contains(&receipt.sequence_index) {
        return Err("sequence_index must be 1 or 2 for a paired replay".into());
    }
    validate_git_sha(&receipt.revision, "revision")?;
    validate_path(&receipt.target_path, "target_path")?;
    validate_git_sha(&receipt.target_blob_sha, "target_blob_sha")?;
    for (value, field) in [
        (&receipt.trial_spec_sha256, "trial_spec_sha256"),
        (&receipt.task_sha256, "task_sha256"),
        (&receipt.patch_sha256, "patch_sha256"),
        (&receipt.evidence_packet_sha256, "evidence_packet_sha256"),
        (
            &receipt.completion_contract_sha256,
            "completion_contract_sha256",
        ),
        (&receipt.sampling_config_sha256, "sampling_config_sha256"),
        (
            &receipt.checkout_reset_receipt_sha256,
            "checkout_reset_receipt_sha256",
        ),
        (&receipt.worker_output_sha256, "worker_output_sha256"),
    ] {
        validate_sha256(value, field)?;
    }
    Ok(())
}

fn validate_receipt_against_manifest(
    receipt: &WorkerRunReceipt,
    manifest: &TrialInputManifest,
) -> Result<(), String> {
    if receipt.trial_id != manifest.trial_id
        || receipt.trial_spec_sha256 != manifest.trial_spec_sha256
        || receipt.repository != manifest.repository
        || receipt.revision != manifest.revision
        || receipt.target_path != manifest.target_path
        || receipt.target_blob_sha != manifest.target_blob_sha
    {
        return Err(format!(
            "run {} does not match the frozen trial coordinate",
            receipt.run_id
        ));
    }
    if receipt.task_sha256 != manifest.worker_visible_common.task.sha256
        || receipt.patch_sha256 != manifest.worker_visible_common.patch.sha256
        || receipt.completion_contract_sha256 != manifest.evaluator_only.oracle.sha256
    {
        return Err(format!(
            "run {} does not match frozen task/patch/completion fingerprints",
            receipt.run_id
        ));
    }
    let condition = manifest
        .conditions
        .get(&receipt.condition_id)
        .ok_or_else(|| format!("run {} names unknown condition", receipt.run_id))?;
    if receipt.evidence_packet_sha256 != condition.packet.sha256 {
        return Err(format!(
            "run {} evidence packet fingerprint does not match condition {}",
            receipt.run_id, receipt.condition_id
        ));
    }
    Ok(())
}

fn same_frozen_identity(first: &WorkerRunReceipt, second: &WorkerRunReceipt) -> bool {
    first.pair_id == second.pair_id
        && first.trial_id == second.trial_id
        && first.trial_spec_sha256 == second.trial_spec_sha256
        && first.repository == second.repository
        && first.revision == second.revision
        && first.target_path == second.target_path
        && first.target_blob_sha == second.target_blob_sha
        && first.task_sha256 == second.task_sha256
        && first.patch_sha256 == second.patch_sha256
        && first.completion_contract_sha256 == second.completion_contract_sha256
        && first.worker_identity == second.worker_identity
        && first.harness_identity == second.harness_identity
        && first.affordance_identity == second.affordance_identity
        && first.sampling_config_sha256 == second.sampling_config_sha256
}

fn validate_condition_recipe(
    id: &str,
    packet_kind: PacketKind,
    budget_bytes: usize,
    scope: Option<&str>,
) -> Result<(), String> {
    if budget_bytes == 0 || budget_bytes > 16 * 1024 * 1024 {
        return Err(format!("condition {id} has invalid budget"));
    }
    match (packet_kind, scope) {
        (PacketKind::FileLocal, None) => Ok(()),
        (PacketKind::Scoped, Some(scope)) => validate_path(scope, "condition scope"),
        (PacketKind::FileLocal, Some(_)) => {
            Err(format!("condition {id} is file-local but declares a scope"))
        }
        (PacketKind::Scoped, None) => Err(format!("condition {id} is scoped but omits its scope")),
    }
}

fn line_artifact_digest(text: &str) -> ArtifactDigest {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(b'\n');
    artifact_digest(&bytes)
}

fn oracle_artifact_digest(oracle: &Oracle) -> Result<ArtifactDigest, String> {
    let values = BTreeMap::from([
        (
            "blocking_reason",
            serde_json::Value::String(oracle.blocking_reason.clone()),
        ),
        (
            "corrective_action",
            serde_json::Value::String(oracle.corrective_action.clone()),
        ),
        (
            "expected_disposition",
            serde_json::Value::String(oracle.expected_disposition.clone()),
        ),
        (
            "max_identifier_length",
            serde_json::Value::from(oracle.max_identifier_length),
        ),
        (
            "proposed_identifier",
            serde_json::Value::String(oracle.proposed_identifier.clone()),
        ),
        (
            "proposed_identifier_length",
            serde_json::Value::from(oracle.proposed_identifier_length),
        ),
    ]);
    let mut bytes = serde_json::to_vec_pretty(&values).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(artifact_digest(&bytes))
}

fn artifact_digest(bytes: &[u8]) -> ArtifactDigest {
    ArtifactDigest {
        sha256: sha256_hex(bytes),
        bytes: bytes.len(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_artifact_digest(digest: &ArtifactDigest, field: &str) -> Result<(), String> {
    validate_sha256(&digest.sha256, &format!("{field}.sha256"))?;
    if digest.bytes == 0 || digest.bytes > 16 * 1024 * 1024 {
        return Err(format!("{field}.bytes is outside admitted boundary"));
    }
    Ok(())
}

fn ensure_bounded(input: &[u8], maximum: usize, field: &str) -> Result<(), String> {
    if input.len() > maximum {
        return Err(format!("{field} exceeds {maximum} bytes"));
    }
    Ok(())
}

fn validate_id(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_ID_BYTES {
        return Err(format!("{field} must contain 1..{MAX_ID_BYTES} bytes"));
    }
    Ok(())
}

fn validate_path(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value.starts_with('/')
        || value.starts_with("./")
        || value.ends_with('/')
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!(
            "{field} must be a canonical repository-relative path"
        ));
    }
    Ok(())
}

fn validate_git_sha(value: &str, field: &str) -> Result<(), String> {
    validate_hex(value, 40, field)
}

fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    validate_hex(value, 64, field)
}

fn validate_hex(value: &str, length: usize, field: &str) -> Result<(), String> {
    if value.len() != length || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{field} must be exactly {length} hexadecimal characters"
        ));
    }
    Ok(())
}

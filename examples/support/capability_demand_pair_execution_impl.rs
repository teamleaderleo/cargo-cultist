use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[path = "../../src/capability_demand_retirement.rs"]
pub mod capability_demand_retirement;

use capability_demand_retirement::{
    ArtifactDigest, EvidenceInspection, Oracle, RunOutcome, TrialInputManifest, TrialSpec,
    parse_run_receipt, parse_trial_manifest, parse_trial_spec,
};

pub const MAX_WORKER_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PAIR_PLAN_BYTES: usize = 512 * 1024;
pub const MAX_EXTERNAL_RUN_METADATA_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PairOrder {
    Ab,
    Ba,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OrderSelection {
    Explicit(PairOrder),
    Seeded(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OrderSource {
    Explicit,
    Seeded { seed_sha256: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairArtifactDigest {
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizerSlot {
    pub slot_id: String,
    pub run_id: String,
    pub sequence_index: u32,
    pub condition_id: String,
    pub evidence_packet: PairArtifactDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizerPlan {
    pub schema_version: u32,
    pub trial_id: String,
    pub pair_id: String,
    pub repository: String,
    pub revision: String,
    pub target_path: String,
    pub target_blob_sha: String,
    pub trial_spec_sha256: String,
    pub input_manifest_sha256: String,
    pub task: PairArtifactDigest,
    pub patch: PairArtifactDigest,
    pub completion_contract_sha256: String,
    pub order: PairOrder,
    pub order_source: OrderSource,
    pub slots: Vec<OrganizerSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerBundleFiles {
    pub task: PairArtifactDigest,
    pub patch: PairArtifactDigest,
    pub evidence: PairArtifactDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerBundleMetadata {
    pub schema_version: u32,
    pub trial_id: String,
    pub pair_id: String,
    pub slot_id: String,
    pub run_id: String,
    pub sequence_index: u32,
    pub repository: String,
    pub revision: String,
    pub target_path: String,
    pub target_blob_sha: String,
    pub files: WorkerBundleFiles,
}

#[derive(Debug, Clone)]
pub struct PreparedSlot {
    pub metadata: WorkerBundleMetadata,
    pub task: Vec<u8>,
    pub patch: Vec<u8>,
    pub evidence: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PreparedPair {
    pub plan: OrganizerPlan,
    pub slots: Vec<PreparedSlot>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOrigin {
    ExternalHarness,
    SyntheticTest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRunMetadata {
    pub schema_version: u32,
    pub slot_id: String,
    pub execution_origin: ExecutionOrigin,
    pub worker_identity: String,
    pub harness_identity: String,
    pub affordance_identity: String,
    pub session_id: String,
    pub fresh_session: bool,
    pub prior_condition_exposure: bool,
    pub evaluated_outcome: RunOutcome,
    pub evidence_inspection: EvidenceInspection,
    pub context_expanded: bool,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

pub fn artifact_digest(bytes: &[u8]) -> PairArtifactDigest {
    PairArtifactDigest {
        sha256: sha256_hex(bytes),
        bytes: bytes.len(),
    }
}

pub fn parse_order_selection(value: &str) -> Result<OrderSelection, String> {
    match value {
        "AB" | "ab" => Ok(OrderSelection::Explicit(PairOrder::Ab)),
        "BA" | "ba" => Ok(OrderSelection::Explicit(PairOrder::Ba)),
        _ => {
            let seed = value
                .strip_prefix("seed:")
                .ok_or_else(|| "order must be AB, BA, or seed:<organizer-seed>".to_string())?;
            if seed.is_empty() || seed.len() > 1024 {
                return Err("organizer seed must contain 1..1024 bytes".into());
            }
            Ok(OrderSelection::Seeded(seed.to_string()))
        }
    }
}

#[allow(clippy::too_many_arguments)]
// V1 keeps the exact frozen artifact set explicit at this research boundary.
pub fn prepare_pair(
    trial_bytes: &[u8],
    manifest_bytes: &[u8],
    task_bytes: &[u8],
    patch_bytes: &[u8],
    packet_one: &[u8],
    packet_two: &[u8],
    pair_id: &str,
    order_selection: OrderSelection,
) -> Result<PreparedPair, String> {
    validate_pair_id(pair_id)?;
    ensure_artifact_bound(task_bytes, "task")?;
    ensure_artifact_bound(patch_bytes, "patch")?;
    ensure_artifact_bound(packet_one, "packet one")?;
    ensure_artifact_bound(packet_two, "packet two")?;

    let spec = parse_trial_spec(trial_bytes)?;
    let manifest = parse_trial_manifest(manifest_bytes)?;
    validate_spec_manifest_pair(&spec, &manifest)?;

    let expected_task = line_terminated(&spec.worker_task.prompt);
    if task_bytes != expected_task {
        return Err("task bytes do not match the frozen trial worker prompt".into());
    }
    let expected_patch = line_terminated(&spec.worker_task.patch);
    if patch_bytes != expected_patch {
        return Err("patch bytes do not match the frozen trial proposed patch".into());
    }
    verify_digest(
        task_bytes,
        &manifest.worker_visible_common.task.sha256,
        manifest.worker_visible_common.task.bytes,
        "task",
    )?;
    verify_digest(
        patch_bytes,
        &manifest.worker_visible_common.patch.sha256,
        manifest.worker_visible_common.patch.bytes,
        "patch",
    )?;
    if manifest.evaluator_only.oracle != oracle_artifact_digest(&spec.oracle)? {
        return Err("oracle artifact does not match frozen trial spec".into());
    }

    if manifest.conditions.len() != 2 {
        return Err("execution packager v1 requires exactly two manifest conditions".into());
    }

    let mut packet_by_condition = BTreeMap::new();
    for packet in [packet_one, packet_two] {
        let condition_id = identify_packet_condition(packet, &manifest)?;
        if packet_by_condition.insert(condition_id, packet).is_some() {
            return Err("two supplied packets resolve to the same condition".into());
        }
    }
    if packet_by_condition.len() != manifest.conditions.len() {
        return Err("supplied packet set does not cover the complete condition pair".into());
    }

    let mut baseline_id = None;
    let mut treatment_id = None;
    for (condition_id, condition) in &manifest.conditions {
        if condition.decisive_evidence_present {
            if treatment_id.replace(condition_id.as_str()).is_some() {
                return Err(
                    "execution packager v1 requires exactly one treatment condition".into(),
                );
            }
        } else if baseline_id.replace(condition_id.as_str()).is_some() {
            return Err("execution packager v1 requires exactly one baseline condition".into());
        }
    }
    let baseline_id = baseline_id.ok_or_else(|| "baseline condition is missing".to_string())?;
    let treatment_id = treatment_id.ok_or_else(|| "treatment condition is missing".to_string())?;

    let (order, order_source) = resolve_order(pair_id, order_selection);
    let ordered_condition_ids = match order {
        PairOrder::Ab => [baseline_id, treatment_id],
        PairOrder::Ba => [treatment_id, baseline_id],
    };

    let task_digest = artifact_digest(task_bytes);
    let patch_digest = artifact_digest(patch_bytes);
    let mut organizer_slots = Vec::with_capacity(2);
    let mut prepared_slots = Vec::with_capacity(2);
    for (offset, condition_id) in ordered_condition_ids.into_iter().enumerate() {
        let sequence_index = (offset + 1) as u32;
        let slot_id = format!("slot-{sequence_index}");
        let run_id = format!("{pair_id}-run-{sequence_index}");
        let evidence = packet_by_condition
            .get(condition_id)
            .ok_or_else(|| format!("missing bytes for condition {condition_id}"))?;
        let evidence_digest = artifact_digest(evidence);

        organizer_slots.push(OrganizerSlot {
            slot_id: slot_id.clone(),
            run_id: run_id.clone(),
            sequence_index,
            condition_id: condition_id.to_string(),
            evidence_packet: evidence_digest.clone(),
        });
        prepared_slots.push(PreparedSlot {
            metadata: WorkerBundleMetadata {
                schema_version: 1,
                trial_id: manifest.trial_id.clone(),
                pair_id: pair_id.to_string(),
                slot_id,
                run_id,
                sequence_index,
                repository: manifest.repository.clone(),
                revision: manifest.revision.clone(),
                target_path: manifest.target_path.clone(),
                target_blob_sha: manifest.target_blob_sha.clone(),
                files: WorkerBundleFiles {
                    task: task_digest.clone(),
                    patch: patch_digest.clone(),
                    evidence: evidence_digest,
                },
            },
            task: task_bytes.to_vec(),
            patch: patch_bytes.to_vec(),
            evidence: evidence.to_vec(),
        });
    }

    Ok(PreparedPair {
        plan: OrganizerPlan {
            schema_version: 1,
            trial_id: manifest.trial_id.clone(),
            pair_id: pair_id.to_string(),
            repository: manifest.repository.clone(),
            revision: manifest.revision.clone(),
            target_path: manifest.target_path.clone(),
            target_blob_sha: manifest.target_blob_sha.clone(),
            trial_spec_sha256: spec.source_sha256.clone(),
            input_manifest_sha256: sha256_hex(manifest_bytes),
            task: task_digest,
            patch: patch_digest,
            completion_contract_sha256: manifest.evaluator_only.oracle.sha256.clone(),
            order,
            order_source,
            slots: organizer_slots,
        },
        slots: prepared_slots,
    })
}

pub fn parse_organizer_plan(input: &[u8]) -> Result<OrganizerPlan, String> {
    if input.len() > MAX_PAIR_PLAN_BYTES {
        return Err(format!(
            "organizer plan is {} bytes; maximum is {} bytes",
            input.len(),
            MAX_PAIR_PLAN_BYTES
        ));
    }
    let plan: OrganizerPlan = serde_json::from_slice(input).map_err(|error| error.to_string())?;
    validate_organizer_plan(&plan)?;
    Ok(plan)
}

pub fn parse_external_run_metadata(input: &[u8]) -> Result<ExternalRunMetadata, String> {
    if input.len() > MAX_EXTERNAL_RUN_METADATA_BYTES {
        return Err(format!(
            "external run metadata is {} bytes; maximum is {} bytes",
            input.len(),
            MAX_EXTERNAL_RUN_METADATA_BYTES
        ));
    }
    let metadata: ExternalRunMetadata =
        serde_json::from_slice(input).map_err(|error| error.to_string())?;
    validate_external_run_metadata(&metadata)?;
    Ok(metadata)
}

pub fn build_external_run_receipt(
    plan: &OrganizerPlan,
    metadata: &ExternalRunMetadata,
    sampling_config: &[u8],
    checkout_reset_receipt: &[u8],
    worker_output: &[u8],
) -> Result<Value, String> {
    build_run_receipt(
        plan,
        metadata,
        sampling_config,
        checkout_reset_receipt,
        worker_output,
        false,
    )
}

#[cfg(test)]
pub fn build_synthetic_test_run_receipt(
    plan: &OrganizerPlan,
    metadata: &ExternalRunMetadata,
    sampling_config: &[u8],
    checkout_reset_receipt: &[u8],
    worker_output: &[u8],
) -> Result<Value, String> {
    build_run_receipt(
        plan,
        metadata,
        sampling_config,
        checkout_reset_receipt,
        worker_output,
        true,
    )
}

fn build_run_receipt(
    plan: &OrganizerPlan,
    metadata: &ExternalRunMetadata,
    sampling_config: &[u8],
    checkout_reset_receipt: &[u8],
    worker_output: &[u8],
    allow_synthetic_test: bool,
) -> Result<Value, String> {
    ensure_artifact_bound(sampling_config, "sampling config")?;
    ensure_artifact_bound(checkout_reset_receipt, "checkout reset receipt")?;
    ensure_artifact_bound(worker_output, "worker output")?;
    match metadata.execution_origin {
        ExecutionOrigin::ExternalHarness => {}
        ExecutionOrigin::SyntheticTest if allow_synthetic_test => {}
        ExecutionOrigin::SyntheticTest => {
            return Err("synthetic test metadata cannot produce an external worker receipt".into());
        }
    }

    let slot = plan
        .slots
        .iter()
        .find(|slot| slot.slot_id == metadata.slot_id)
        .ok_or_else(|| format!("unknown organizer slot {}", metadata.slot_id))?;

    let value = serde_json::json!({
        "schema_version": 1,
        "trial_id": plan.trial_id,
        "trial_spec_sha256": plan.trial_spec_sha256,
        "pair_id": plan.pair_id,
        "run_id": slot.run_id,
        "condition_id": slot.condition_id,
        "sequence_index": slot.sequence_index,
        "repository": plan.repository,
        "revision": plan.revision,
        "target_path": plan.target_path,
        "target_blob_sha": plan.target_blob_sha,
        "task_sha256": plan.task.sha256,
        "patch_sha256": plan.patch.sha256,
        "evidence_packet_sha256": slot.evidence_packet.sha256,
        "completion_contract_sha256": plan.completion_contract_sha256,
        "worker_identity": metadata.worker_identity,
        "harness_identity": metadata.harness_identity,
        "affordance_identity": metadata.affordance_identity,
        "sampling_config_sha256": sha256_hex(sampling_config),
        "session_id": metadata.session_id,
        "fresh_session": metadata.fresh_session,
        "prior_condition_exposure": metadata.prior_condition_exposure,
        "checkout_reset_receipt_sha256": sha256_hex(checkout_reset_receipt),
        "worker_output_sha256": sha256_hex(worker_output),
        "evaluated_outcome": metadata.evaluated_outcome,
        "evidence_inspection": metadata.evidence_inspection,
        "context_expanded": metadata.context_expanded,
    });
    let encoded = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
    parse_run_receipt(&encoded)?;
    Ok(value)
}

fn resolve_order(pair_id: &str, selection: OrderSelection) -> (PairOrder, OrderSource) {
    match selection {
        OrderSelection::Explicit(order) => (order, OrderSource::Explicit),
        OrderSelection::Seeded(seed) => {
            let seed_sha256 = sha256_hex(seed.as_bytes());
            let mut hasher = Sha256::new();
            hasher.update(seed.as_bytes());
            hasher.update([0]);
            hasher.update(pair_id.as_bytes());
            let digest = hasher.finalize();
            let order = if digest[0] & 1 == 0 {
                PairOrder::Ab
            } else {
                PairOrder::Ba
            };
            (order, OrderSource::Seeded { seed_sha256 })
        }
    }
}

fn identify_packet_condition<'a>(
    packet: &[u8],
    manifest: &'a TrialInputManifest,
) -> Result<&'a str, String> {
    let digest = artifact_digest(packet);
    let matches = manifest
        .conditions
        .iter()
        .filter(|(_, condition)| {
            condition.packet.sha256 == digest.sha256 && condition.packet.bytes == digest.bytes
        })
        .map(|(condition_id, _)| condition_id.as_str())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [condition_id] => Ok(condition_id),
        [] => Err(format!(
            "supplied evidence packet {} ({} bytes) is not admitted by the manifest",
            digest.sha256, digest.bytes
        )),
        _ => Err("multiple conditions share the same exact packet identity".into()),
    }
}

fn validate_spec_manifest_pair(
    spec: &TrialSpec,
    manifest: &TrialInputManifest,
) -> Result<(), String> {
    if spec.trial_id != manifest.trial_id
        || spec.repository != manifest.repository
        || spec.revision != manifest.revision
        || spec.target_path != manifest.target_path
        || spec.target_blob_sha != manifest.target_blob_sha
    {
        return Err("trial spec and input manifest name different frozen coordinates".into());
    }
    if manifest.trial_spec_sha256 != spec.source_sha256 {
        return Err("trial input manifest does not match exact frozen trial-spec bytes".into());
    }

    let spec_conditions = spec
        .conditions
        .iter()
        .map(|condition| (condition.id.as_str(), condition))
        .collect::<BTreeMap<_, _>>();
    if spec_conditions.len() != manifest.conditions.len() {
        return Err("trial spec and input manifest condition sets differ".into());
    }
    for (condition_id, manifest_condition) in &manifest.conditions {
        let spec_condition = spec_conditions
            .get(condition_id.as_str())
            .ok_or_else(|| format!("manifest contains undeclared condition {condition_id}"))?;
        if spec_condition.packet_kind != manifest_condition.packet_kind
            || spec_condition.budget_bytes != manifest_condition.budget_bytes
            || spec_condition.scope != manifest_condition.scope
        {
            return Err(format!(
                "condition {condition_id} materialization recipe drifted"
            ));
        }
        let spec_refs = spec_condition
            .decisive_evidence_refs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let manifest_refs = manifest_condition
            .decisive_evidence_refs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if spec_condition.decisive_evidence_present != manifest_condition.decisive_evidence_present
            || spec_refs != manifest_refs
        {
            return Err(format!(
                "condition {condition_id} differs between trial spec and input manifest"
            ));
        }
    }
    Ok(())
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
    Ok(ArtifactDigest {
        sha256: sha256_hex(&bytes),
        bytes: bytes.len(),
    })
}

fn line_terminated(value: &str) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(b'\n');
    bytes
}

fn verify_digest(
    bytes: &[u8],
    expected_sha256: &str,
    expected_bytes: usize,
    label: &str,
) -> Result<(), String> {
    let actual = artifact_digest(bytes);
    if actual.sha256 != expected_sha256 || actual.bytes != expected_bytes {
        return Err(format!(
            "{label} does not match the frozen manifest: got {} / {} bytes",
            actual.sha256, actual.bytes
        ));
    }
    Ok(())
}

fn validate_pair_id(pair_id: &str) -> Result<(), String> {
    if pair_id.is_empty() || pair_id.len() > 120 {
        return Err("pair_id must contain 1..120 bytes".into());
    }
    if !pair_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err("pair_id may contain only ASCII letters, digits, '-', '_', '.', ':'".into());
    }
    Ok(())
}

fn validate_organizer_plan(plan: &OrganizerPlan) -> Result<(), String> {
    if plan.schema_version != 1 {
        return Err(format!(
            "unsupported organizer plan schema version {}",
            plan.schema_version
        ));
    }
    validate_pair_id(&plan.pair_id)?;
    if plan.trial_id.is_empty()
        || plan.repository.is_empty()
        || !is_git_sha(&plan.revision)
        || plan.target_path.is_empty()
        || !is_git_sha(&plan.target_blob_sha)
    {
        return Err("organizer plan has incomplete frozen identity".into());
    }
    for digest in [
        &plan.trial_spec_sha256,
        &plan.input_manifest_sha256,
        &plan.task.sha256,
        &plan.patch.sha256,
        &plan.completion_contract_sha256,
    ] {
        validate_sha256(digest, "organizer digest")?;
    }
    if plan.task.bytes == 0 || plan.patch.bytes == 0 {
        return Err("organizer task/patch digests must be non-empty".into());
    }
    if plan.slots.len() != 2 {
        return Err("organizer plan must contain exactly two slots".into());
    }
    let slot_ids = plan
        .slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let run_ids = plan
        .slots
        .iter()
        .map(|slot| slot.run_id.as_str())
        .collect::<BTreeSet<_>>();
    let sequences = plan
        .slots
        .iter()
        .map(|slot| slot.sequence_index)
        .collect::<BTreeSet<_>>();
    if slot_ids.len() != 2 || run_ids.len() != 2 || sequences != BTreeSet::from([1, 2]) {
        return Err(
            "organizer slots must have unique slot/run IDs and sequence indices {1,2}".into(),
        );
    }
    for slot in &plan.slots {
        if slot.slot_id.is_empty() || slot.run_id.is_empty() || slot.condition_id.is_empty() {
            return Err("organizer slot identity must be non-empty".into());
        }
        validate_sha256(&slot.evidence_packet.sha256, "evidence packet sha256")?;
        if slot.evidence_packet.bytes == 0 {
            return Err("organizer evidence packet must be non-empty".into());
        }
    }
    Ok(())
}

fn validate_external_run_metadata(metadata: &ExternalRunMetadata) -> Result<(), String> {
    if metadata.schema_version != 1 {
        return Err(format!(
            "unsupported external run metadata schema version {}",
            metadata.schema_version
        ));
    }
    for (value, field) in [
        (&metadata.slot_id, "slot_id"),
        (&metadata.worker_identity, "worker_identity"),
        (&metadata.harness_identity, "harness_identity"),
        (&metadata.affordance_identity, "affordance_identity"),
        (&metadata.session_id, "session_id"),
    ] {
        if value.is_empty() || value.len() > 256 {
            return Err(format!("{field} must contain 1..256 bytes"));
        }
    }
    Ok(())
}

fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{field} must be a 64-hex SHA-256"));
    }
    Ok(())
}

fn is_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn ensure_artifact_bound(bytes: &[u8], label: &str) -> Result<(), String> {
    if bytes.is_empty() {
        return Err(format!("{label} must be non-empty"));
    }
    if bytes.len() > MAX_WORKER_ARTIFACT_BYTES {
        return Err(format!(
            "{label} is {} bytes; maximum is {} bytes",
            bytes.len(),
            MAX_WORKER_ARTIFACT_BYTES
        ));
    }
    Ok(())
}

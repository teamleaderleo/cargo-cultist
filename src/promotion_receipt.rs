use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const PROMOTION_RECEIPT_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROMOTION_RECEIPT_REQUEST_BYTES: usize = 512 * 1024;
const MAX_COMMITS: usize = 512;
const MAX_PATHS: usize = 4096;
const MAX_CHECK_REFS: usize = 256;
const MAX_ATOM_BYTES: usize = 4096;
const SHA256_HEX_BYTES: usize = 64;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionReceiptRequest {
    pub schema_version: u32,
    pub tested: TestedPromotionState,
    pub current: CurrentPromotionState,
    pub branch_changed_paths: Vec<String>,
    pub consumed_contract_paths: Vec<String>,
    pub applicable_policy_paths: Vec<String>,
    pub intervening_commits: Vec<InterveningCommit>,
    pub compatibility_scope_complete: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TestedPromotionState {
    pub head_sha: String,
    pub tree_sha: String,
    pub change_set_sha256: String,
    pub base_sha: String,
    pub base_tree_sha: String,
    pub effective_merge_tree_sha: String,
    pub successful_check_refs: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentPromotionState {
    pub head_sha: String,
    pub tree_sha: String,
    pub change_set_sha256: String,
    pub base_sha: String,
    pub base_tree_sha: String,
    pub effective_merge_tree_sha: String,
    pub mergeable: bool,
    pub conflict: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InterveningCommit {
    pub sha: String,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionReceiptDisposition {
    ReceiptReusable,
    RerunRequired,
    InspectSemanticOverlap,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionReceiptReason {
    ExactEffectiveMergeTreeIdentity,
    EquivalentChangeSetSameBaseTree,
    ChangeSetChanged,
    MergeConflict,
    NotMergeable,
    BranchPathOverlap,
    ConsumedContractOverlap,
    ApplicablePolicyOverlap,
    CompleteCompatibilityScopeNoRelevantChange,
    SemanticIndependenceUnknown,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionPathOverlapKind {
    BranchPath,
    ConsumedContract,
    ApplicablePolicy,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionPathOverlap {
    pub commit_sha: String,
    pub changed_path: String,
    pub declared_path: String,
    pub kind: PromotionPathOverlapKind,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionReceiptEvaluation {
    pub schema_version: u32,
    pub disposition: PromotionReceiptDisposition,
    pub reasons: Vec<PromotionReceiptReason>,
    pub tested_head_sha: String,
    pub tested_tree_sha: String,
    pub tested_change_set_sha256: String,
    pub tested_base_sha: String,
    pub tested_base_tree_sha: String,
    pub tested_effective_merge_tree_sha: String,
    pub successful_check_refs: Vec<String>,
    pub current_head_sha: String,
    pub current_tree_sha: String,
    pub current_change_set_sha256: String,
    pub current_base_sha: String,
    pub current_base_tree_sha: String,
    pub current_effective_merge_tree_sha: String,
    pub intervening_commit_shas: Vec<String>,
    pub overlaps: Vec<PromotionPathOverlap>,
    pub compatibility_scope_complete: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PromotionReceiptError {
    message: String,
}

impl PromotionReceiptError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PromotionReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PromotionReceiptError {}

pub fn parse_promotion_receipt_request(
    bytes: &[u8],
) -> Result<PromotionReceiptRequest, PromotionReceiptError> {
    if bytes.len() > MAX_PROMOTION_RECEIPT_REQUEST_BYTES {
        return Err(PromotionReceiptError::new(format!(
            "promotion receipt request exceeds the {MAX_PROMOTION_RECEIPT_REQUEST_BYTES}-byte limit"
        )));
    }
    let request: PromotionReceiptRequest = serde_json::from_slice(bytes).map_err(|error| {
        PromotionReceiptError::new(format!("invalid promotion receipt JSON: {error}"))
    })?;
    validate_request(&request)?;
    Ok(request)
}

pub fn evaluate_promotion_receipt(
    request: &PromotionReceiptRequest,
) -> Result<PromotionReceiptEvaluation, PromotionReceiptError> {
    validate_request(request)?;

    let mut reasons = Vec::new();
    let mut overlaps = collect_overlaps(request);
    overlaps.sort();
    overlaps.dedup();

    let disposition = if request.current.conflict {
        reasons.push(PromotionReceiptReason::MergeConflict);
        PromotionReceiptDisposition::RerunRequired
    } else if !request.current.mergeable {
        reasons.push(PromotionReceiptReason::NotMergeable);
        PromotionReceiptDisposition::RerunRequired
    } else if request.tested.change_set_sha256 != request.current.change_set_sha256 {
        reasons.push(PromotionReceiptReason::ChangeSetChanged);
        PromotionReceiptDisposition::RerunRequired
    } else if request.tested.effective_merge_tree_sha == request.current.effective_merge_tree_sha {
        reasons.push(PromotionReceiptReason::ExactEffectiveMergeTreeIdentity);
        PromotionReceiptDisposition::ReceiptReusable
    } else if request.tested.base_tree_sha == request.current.base_tree_sha {
        reasons.push(PromotionReceiptReason::EquivalentChangeSetSameBaseTree);
        PromotionReceiptDisposition::ReceiptReusable
    } else if !overlaps.is_empty() {
        let kinds = overlaps
            .iter()
            .map(|overlap| overlap.kind)
            .collect::<BTreeSet<_>>();
        if kinds.contains(&PromotionPathOverlapKind::BranchPath) {
            reasons.push(PromotionReceiptReason::BranchPathOverlap);
        }
        if kinds.contains(&PromotionPathOverlapKind::ConsumedContract) {
            reasons.push(PromotionReceiptReason::ConsumedContractOverlap);
        }
        if kinds.contains(&PromotionPathOverlapKind::ApplicablePolicy) {
            reasons.push(PromotionReceiptReason::ApplicablePolicyOverlap);
        }
        PromotionReceiptDisposition::RerunRequired
    } else if request.compatibility_scope_complete {
        reasons.push(PromotionReceiptReason::CompleteCompatibilityScopeNoRelevantChange);
        PromotionReceiptDisposition::ReceiptReusable
    } else {
        reasons.push(PromotionReceiptReason::SemanticIndependenceUnknown);
        PromotionReceiptDisposition::InspectSemanticOverlap
    };

    reasons.sort();
    reasons.dedup();

    Ok(PromotionReceiptEvaluation {
        schema_version: PROMOTION_RECEIPT_SCHEMA_VERSION,
        disposition,
        reasons,
        tested_head_sha: request.tested.head_sha.clone(),
        tested_tree_sha: request.tested.tree_sha.clone(),
        tested_change_set_sha256: request.tested.change_set_sha256.clone(),
        tested_base_sha: request.tested.base_sha.clone(),
        tested_base_tree_sha: request.tested.base_tree_sha.clone(),
        tested_effective_merge_tree_sha: request.tested.effective_merge_tree_sha.clone(),
        successful_check_refs: request.tested.successful_check_refs.clone(),
        current_head_sha: request.current.head_sha.clone(),
        current_tree_sha: request.current.tree_sha.clone(),
        current_change_set_sha256: request.current.change_set_sha256.clone(),
        current_base_sha: request.current.base_sha.clone(),
        current_base_tree_sha: request.current.base_tree_sha.clone(),
        current_effective_merge_tree_sha: request.current.effective_merge_tree_sha.clone(),
        intervening_commit_shas: request
            .intervening_commits
            .iter()
            .map(|commit| commit.sha.clone())
            .collect(),
        overlaps,
        compatibility_scope_complete: request.compatibility_scope_complete,
    })
}

fn validate_request(request: &PromotionReceiptRequest) -> Result<(), PromotionReceiptError> {
    if request.schema_version != PROMOTION_RECEIPT_SCHEMA_VERSION {
        return Err(PromotionReceiptError::new(format!(
            "unsupported promotion receipt schema {}; expected {PROMOTION_RECEIPT_SCHEMA_VERSION}",
            request.schema_version
        )));
    }

    for (sha, field) in [
        (&request.tested.head_sha, "tested.head_sha"),
        (&request.tested.tree_sha, "tested.tree_sha"),
        (&request.tested.base_sha, "tested.base_sha"),
        (&request.tested.base_tree_sha, "tested.base_tree_sha"),
        (
            &request.tested.effective_merge_tree_sha,
            "tested.effective_merge_tree_sha",
        ),
        (&request.current.head_sha, "current.head_sha"),
        (&request.current.tree_sha, "current.tree_sha"),
        (&request.current.base_sha, "current.base_sha"),
        (&request.current.base_tree_sha, "current.base_tree_sha"),
        (
            &request.current.effective_merge_tree_sha,
            "current.effective_merge_tree_sha",
        ),
    ] {
        validate_git_sha(sha, field)?;
    }
    validate_sha256_receipt(
        &request.tested.change_set_sha256,
        "tested.change_set_sha256",
    )?;
    validate_sha256_receipt(
        &request.current.change_set_sha256,
        "current.change_set_sha256",
    )?;

    if request.tested.successful_check_refs.is_empty()
        || request.tested.successful_check_refs.len() > MAX_CHECK_REFS
    {
        return Err(PromotionReceiptError::new(
            "tested.successful_check_refs must be bounded and non-empty",
        ));
    }
    let mut check_refs = BTreeSet::new();
    for check_ref in &request.tested.successful_check_refs {
        validate_atom(check_ref, "successful check ref")?;
        if !check_refs.insert(check_ref) {
            return Err(PromotionReceiptError::new(format!(
                "duplicate successful check ref {check_ref}"
            )));
        }
    }

    validate_path_set(&request.branch_changed_paths, "branch_changed_paths")?;
    validate_path_set(&request.consumed_contract_paths, "consumed_contract_paths")?;
    validate_path_set(&request.applicable_policy_paths, "applicable_policy_paths")?;

    if request.intervening_commits.len() > MAX_COMMITS {
        return Err(PromotionReceiptError::new(
            "intervening_commits exceeds the admitted bound",
        ));
    }
    let mut commit_shas = BTreeSet::new();
    for commit in &request.intervening_commits {
        validate_git_sha(&commit.sha, "intervening commit sha")?;
        if !commit_shas.insert(&commit.sha) {
            return Err(PromotionReceiptError::new(format!(
                "duplicate intervening commit {}",
                commit.sha
            )));
        }
        validate_path_set(&commit.changed_paths, "intervening changed_paths")?;
    }

    if request.tested.base_tree_sha != request.current.base_tree_sha
        && request.intervening_commits.is_empty()
    {
        return Err(PromotionReceiptError::new(
            "base tree changed but no intervening commits were supplied",
        ));
    }

    Ok(())
}

fn collect_overlaps(request: &PromotionReceiptRequest) -> Vec<PromotionPathOverlap> {
    let mut overlaps = Vec::new();
    for commit in &request.intervening_commits {
        for changed_path in &commit.changed_paths {
            collect_path_kind_overlaps(
                &mut overlaps,
                &commit.sha,
                changed_path,
                &request.branch_changed_paths,
                PromotionPathOverlapKind::BranchPath,
            );
            collect_path_kind_overlaps(
                &mut overlaps,
                &commit.sha,
                changed_path,
                &request.consumed_contract_paths,
                PromotionPathOverlapKind::ConsumedContract,
            );
            collect_path_kind_overlaps(
                &mut overlaps,
                &commit.sha,
                changed_path,
                &request.applicable_policy_paths,
                PromotionPathOverlapKind::ApplicablePolicy,
            );
        }
    }
    overlaps
}

fn collect_path_kind_overlaps(
    overlaps: &mut Vec<PromotionPathOverlap>,
    commit_sha: &str,
    changed_path: &str,
    declared_paths: &[String],
    kind: PromotionPathOverlapKind,
) {
    for declared_path in declared_paths {
        if paths_overlap(changed_path, declared_path) {
            overlaps.push(PromotionPathOverlap {
                commit_sha: commit_sha.to_string(),
                changed_path: changed_path.to_string(),
                declared_path: declared_path.clone(),
                kind,
            });
        }
    }
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn validate_path_set(paths: &[String], field: &str) -> Result<(), PromotionReceiptError> {
    if paths.len() > MAX_PATHS {
        return Err(PromotionReceiptError::new(format!(
            "{field} exceeds the admitted path bound"
        )));
    }
    let mut seen = BTreeSet::new();
    for path in paths {
        validate_repo_path(path, field)?;
        if !seen.insert(path) {
            return Err(PromotionReceiptError::new(format!(
                "{field} contains duplicate path {path}"
            )));
        }
    }
    Ok(())
}

fn validate_repo_path(path: &str, field: &str) -> Result<(), PromotionReceiptError> {
    if path.is_empty()
        || path.trim() != path
        || path.len() > MAX_ATOM_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(PromotionReceiptError::new(format!(
            "{field} must contain canonical repository-relative path scopes"
        )));
    }
    Ok(())
}

fn validate_git_sha(value: &str, field: &str) -> Result<(), PromotionReceiptError> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PromotionReceiptError::new(format!(
            "{field} must be an exact lowercase 40-hex Git object id"
        )));
    }
    Ok(())
}

fn validate_sha256_receipt(value: &str, field: &str) -> Result<(), PromotionReceiptError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(PromotionReceiptError::new(format!(
            "{field} must use sha256:<hex>"
        )));
    };
    if hex.len() != SHA256_HEX_BYTES
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PromotionReceiptError::new(format!(
            "{field} must contain exactly {SHA256_HEX_BYTES} lowercase SHA-256 hex characters"
        )));
    }
    Ok(())
}

fn validate_atom(value: &str, field: &str) -> Result<(), PromotionReceiptError> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > MAX_ATOM_BYTES
        || value.contains('\0')
        || value.chars().any(char::is_control)
    {
        return Err(PromotionReceiptError::new(format!(
            "{field} must be bounded canonical text"
        )));
    }
    Ok(())
}

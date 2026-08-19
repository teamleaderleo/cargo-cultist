use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::promotion_receipt::{
    InterveningCommit, PromotionPathOverlapKind, PromotionReceiptDisposition,
    PromotionReceiptReason, PromotionReceiptRequest, evaluate_promotion_receipt,
};

pub const PROMOTION_BASE_LINEAGE_SCHEMA_VERSION: u32 = 1;
const MAX_RANGE_COMMITS: usize = 4096;
const MAX_COMPATIBILITY_OBJECTS: usize = 4096;
const GIT_SHA_HEX_BYTES: usize = 40;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionBaseLineageRequest {
    pub schema_version: u32,
    pub promotion: PromotionReceiptRequest,
    pub merge_base_sha: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tested_base_only: Option<PromotionBaseRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_base_only: Option<PromotionBaseRange>,
    pub compatibility_objects: Vec<PromotionCompatibilityObjectState>,
    pub base_path_receipts_complete: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionBaseRange {
    pub base_sha: String,
    pub head_sha: String,
    pub commit_count: usize,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionBaseRelation {
    SameTree,
    Forward,
    Rewind,
    Diverged,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionBaseDeltaSide {
    TestedBaseOnly,
    CurrentBaseOnly,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionBasePathOverlap {
    pub side: PromotionBaseDeltaSide,
    pub range_head_sha: String,
    pub changed_path: String,
    pub declared_path: String,
    pub kind: PromotionPathOverlapKind,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionCompatibilityObjectKind {
    ConsumedContract,
    ApplicablePolicy,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionCompatibilityObjectState {
    pub path: String,
    pub kind: PromotionCompatibilityObjectKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tested_object_sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_object_sha: Option<String>,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionCompatibilityObjectChange {
    pub path: String,
    pub kind: PromotionCompatibilityObjectKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tested_object_sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_object_sha: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionBaseLineageEvaluation {
    pub schema_version: u32,
    pub base_relation: PromotionBaseRelation,
    pub disposition: PromotionReceiptDisposition,
    pub reasons: Vec<PromotionReceiptReason>,
    pub merge_base_sha: String,
    pub tested_base_sha: String,
    pub tested_base_tree_sha: String,
    pub current_base_sha: String,
    pub current_base_tree_sha: String,
    pub tested_change_set_sha256: String,
    pub current_change_set_sha256: String,
    pub successful_check_refs: Vec<String>,
    pub tested_base_only: Option<PromotionBaseRange>,
    pub current_base_only: Option<PromotionBaseRange>,
    pub branch_path_overlaps: Vec<PromotionBasePathOverlap>,
    pub compatibility_objects: Vec<PromotionCompatibilityObjectState>,
    pub compatibility_changes: Vec<PromotionCompatibilityObjectChange>,
    pub base_path_receipts_complete: bool,
    pub compatibility_scope_complete: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PromotionBaseLineageError {
    message: String,
}

impl PromotionBaseLineageError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PromotionBaseLineageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PromotionBaseLineageError {}

pub fn evaluate_promotion_base_lineage(
    request: &PromotionBaseLineageRequest,
) -> Result<PromotionBaseLineageEvaluation, PromotionBaseLineageError> {
    let relation = validate_request(request)?;
    let mut branch_path_overlaps = collect_branch_path_overlaps(request);
    branch_path_overlaps.sort();
    branch_path_overlaps.dedup();
    let compatibility_changes = collect_compatibility_changes(request);

    let promotion = &request.promotion;
    let mut reasons = Vec::new();
    let disposition = if promotion.current.conflict {
        reasons.push(PromotionReceiptReason::MergeConflict);
        PromotionReceiptDisposition::RerunRequired
    } else if !promotion.current.mergeable {
        reasons.push(PromotionReceiptReason::NotMergeable);
        PromotionReceiptDisposition::RerunRequired
    } else if promotion.tested.change_set_sha256 != promotion.current.change_set_sha256 {
        reasons.push(PromotionReceiptReason::ChangeSetChanged);
        PromotionReceiptDisposition::RerunRequired
    } else if promotion.tested.effective_merge_tree_sha
        == promotion.current.effective_merge_tree_sha
    {
        reasons.push(PromotionReceiptReason::ExactEffectiveMergeTreeIdentity);
        PromotionReceiptDisposition::ReceiptReusable
    } else if promotion.tested.base_tree_sha == promotion.current.base_tree_sha {
        reasons.push(PromotionReceiptReason::EquivalentChangeSetSameBaseTree);
        PromotionReceiptDisposition::ReceiptReusable
    } else if !branch_path_overlaps.is_empty() || !compatibility_changes.is_empty() {
        if !branch_path_overlaps.is_empty() {
            reasons.push(PromotionReceiptReason::BranchPathOverlap);
        }
        if compatibility_changes
            .iter()
            .any(|change| change.kind == PromotionCompatibilityObjectKind::ConsumedContract)
        {
            reasons.push(PromotionReceiptReason::ConsumedContractOverlap);
        }
        if compatibility_changes
            .iter()
            .any(|change| change.kind == PromotionCompatibilityObjectKind::ApplicablePolicy)
        {
            reasons.push(PromotionReceiptReason::ApplicablePolicyOverlap);
        }
        PromotionReceiptDisposition::RerunRequired
    } else if promotion.compatibility_scope_complete {
        reasons.push(PromotionReceiptReason::CompleteCompatibilityScopeNoRelevantChange);
        PromotionReceiptDisposition::ReceiptReusable
    } else {
        reasons.push(PromotionReceiptReason::SemanticIndependenceUnknown);
        PromotionReceiptDisposition::InspectSemanticOverlap
    };
    reasons.sort();
    reasons.dedup();

    Ok(PromotionBaseLineageEvaluation {
        schema_version: PROMOTION_BASE_LINEAGE_SCHEMA_VERSION,
        base_relation: relation,
        disposition,
        reasons,
        merge_base_sha: request.merge_base_sha.clone(),
        tested_base_sha: promotion.tested.base_sha.clone(),
        tested_base_tree_sha: promotion.tested.base_tree_sha.clone(),
        current_base_sha: promotion.current.base_sha.clone(),
        current_base_tree_sha: promotion.current.base_tree_sha.clone(),
        tested_change_set_sha256: promotion.tested.change_set_sha256.clone(),
        current_change_set_sha256: promotion.current.change_set_sha256.clone(),
        successful_check_refs: promotion.tested.successful_check_refs.clone(),
        tested_base_only: request.tested_base_only.clone(),
        current_base_only: request.current_base_only.clone(),
        branch_path_overlaps,
        compatibility_objects: request.compatibility_objects.clone(),
        compatibility_changes,
        base_path_receipts_complete: request.base_path_receipts_complete,
        compatibility_scope_complete: promotion.compatibility_scope_complete,
    })
}

fn validate_request(
    request: &PromotionBaseLineageRequest,
) -> Result<PromotionBaseRelation, PromotionBaseLineageError> {
    if request.schema_version != PROMOTION_BASE_LINEAGE_SCHEMA_VERSION {
        return Err(PromotionBaseLineageError::new(format!(
            "unsupported promotion base-lineage schema {}; expected {PROMOTION_BASE_LINEAGE_SCHEMA_VERSION}",
            request.schema_version
        )));
    }
    validate_git_sha(&request.merge_base_sha, "merge_base_sha")?;
    if !request.promotion.intervening_commits.is_empty() {
        return Err(PromotionBaseLineageError::new(
            "base-lineage request owns base delta receipts; promotion.intervening_commits must be empty",
        ));
    }

    validate_compatibility_objects(request)?;
    if request.promotion.tested.base_tree_sha != request.promotion.current.base_tree_sha
        && !request.base_path_receipts_complete
    {
        return Err(PromotionBaseLineageError::new(
            "changed base lineage requires complete base-range changed-path receipts",
        ));
    }

    let relation =
        if request.promotion.tested.base_tree_sha == request.promotion.current.base_tree_sha {
            if request.tested_base_only.is_some() || request.current_base_only.is_some() {
                return Err(PromotionBaseLineageError::new(
                    "equal base trees must not carry base-only range receipts",
                ));
            }
            if request
                .compatibility_objects
                .iter()
                .any(|state| state.tested_object_sha != state.current_object_sha)
            {
                return Err(PromotionBaseLineageError::new(
                    "equal base trees cannot carry changed compatibility-object receipts",
                ));
            }
            PromotionBaseRelation::SameTree
        } else {
            match (&request.tested_base_only, &request.current_base_only) {
                (None, Some(current)) => {
                    validate_range(
                        current,
                        &request.merge_base_sha,
                        &request.promotion.current.base_sha,
                        "current_base_only",
                    )?;
                    if request.merge_base_sha != request.promotion.tested.base_sha {
                        return Err(PromotionBaseLineageError::new(
                            "forward base lineage requires merge_base_sha to equal tested base_sha",
                        ));
                    }
                    PromotionBaseRelation::Forward
                }
                (Some(tested), None) => {
                    validate_range(
                        tested,
                        &request.merge_base_sha,
                        &request.promotion.tested.base_sha,
                        "tested_base_only",
                    )?;
                    if request.merge_base_sha != request.promotion.current.base_sha {
                        return Err(PromotionBaseLineageError::new(
                            "rewind base lineage requires merge_base_sha to equal current base_sha",
                        ));
                    }
                    PromotionBaseRelation::Rewind
                }
                (Some(tested), Some(current)) => {
                    validate_range(
                        tested,
                        &request.merge_base_sha,
                        &request.promotion.tested.base_sha,
                        "tested_base_only",
                    )?;
                    validate_range(
                        current,
                        &request.merge_base_sha,
                        &request.promotion.current.base_sha,
                        "current_base_only",
                    )?;
                    PromotionBaseRelation::Diverged
                }
                (None, None) => {
                    return Err(PromotionBaseLineageError::new(
                        "changed base trees require a forward, rewind, or divergent range receipt",
                    ));
                }
            }
        };

    let mut synthetic = request.promotion.clone();
    if let Some(range) = &request.tested_base_only {
        synthetic.intervening_commits.push(InterveningCommit {
            sha: range.head_sha.clone(),
            changed_paths: range.changed_paths.clone(),
        });
    }
    if let Some(range) = &request.current_base_only {
        synthetic.intervening_commits.push(InterveningCommit {
            sha: range.head_sha.clone(),
            changed_paths: range.changed_paths.clone(),
        });
    }
    evaluate_promotion_receipt(&synthetic).map_err(|error| {
        PromotionBaseLineageError::new(format!("promotion receipt validation failed: {error}"))
    })?;

    Ok(relation)
}

fn validate_compatibility_objects(
    request: &PromotionBaseLineageRequest,
) -> Result<(), PromotionBaseLineageError> {
    if request.compatibility_objects.len() > MAX_COMPATIBILITY_OBJECTS {
        return Err(PromotionBaseLineageError::new(
            "compatibility_objects exceeds the admitted bound",
        ));
    }

    let expected = request
        .promotion
        .consumed_contract_paths
        .iter()
        .map(|path| {
            (
                path.as_str(),
                PromotionCompatibilityObjectKind::ConsumedContract,
            )
        })
        .chain(
            request
                .promotion
                .applicable_policy_paths
                .iter()
                .map(|path| {
                    (
                        path.as_str(),
                        PromotionCompatibilityObjectKind::ApplicablePolicy,
                    )
                }),
        )
        .collect::<BTreeSet<_>>();

    let mut observed = BTreeSet::new();
    for state in &request.compatibility_objects {
        let key = (state.path.as_str(), state.kind);
        if !expected.contains(&key) {
            return Err(PromotionBaseLineageError::new(format!(
                "unexpected compatibility-object receipt for {}",
                state.path
            )));
        }
        if !observed.insert(key) {
            return Err(PromotionBaseLineageError::new(format!(
                "duplicate compatibility-object receipt for {}",
                state.path
            )));
        }
        if state.tested_object_sha.is_none() && state.current_object_sha.is_none() {
            return Err(PromotionBaseLineageError::new(format!(
                "compatibility-object receipt {} must exist on at least one base",
                state.path
            )));
        }
        if let Some(sha) = &state.tested_object_sha {
            validate_git_sha(sha, "compatibility_object.tested_object_sha")?;
        }
        if let Some(sha) = &state.current_object_sha {
            validate_git_sha(sha, "compatibility_object.current_object_sha")?;
        }
    }

    if observed != expected {
        return Err(PromotionBaseLineageError::new(
            "compatibility_objects must exactly cover every declared consumed contract and applicable policy path",
        ));
    }
    Ok(())
}

fn validate_range(
    range: &PromotionBaseRange,
    expected_base_sha: &str,
    expected_head_sha: &str,
    field: &str,
) -> Result<(), PromotionBaseLineageError> {
    validate_git_sha(&range.base_sha, &format!("{field}.base_sha"))?;
    validate_git_sha(&range.head_sha, &format!("{field}.head_sha"))?;
    if range.base_sha != expected_base_sha || range.head_sha != expected_head_sha {
        return Err(PromotionBaseLineageError::new(format!(
            "{field} must bind the declared merge base to the exact corresponding promotion base"
        )));
    }
    if range.commit_count == 0 || range.commit_count > MAX_RANGE_COMMITS {
        return Err(PromotionBaseLineageError::new(format!(
            "{field}.commit_count must be within the admitted non-empty bound"
        )));
    }
    Ok(())
}

fn collect_branch_path_overlaps(
    request: &PromotionBaseLineageRequest,
) -> Vec<PromotionBasePathOverlap> {
    let mut overlaps = Vec::new();
    if let Some(range) = &request.tested_base_only {
        collect_range_branch_overlaps(
            &mut overlaps,
            PromotionBaseDeltaSide::TestedBaseOnly,
            range,
            &request.promotion.branch_changed_paths,
        );
    }
    if let Some(range) = &request.current_base_only {
        collect_range_branch_overlaps(
            &mut overlaps,
            PromotionBaseDeltaSide::CurrentBaseOnly,
            range,
            &request.promotion.branch_changed_paths,
        );
    }
    overlaps
}

fn collect_range_branch_overlaps(
    overlaps: &mut Vec<PromotionBasePathOverlap>,
    side: PromotionBaseDeltaSide,
    range: &PromotionBaseRange,
    branch_changed_paths: &[String],
) {
    for changed_path in &range.changed_paths {
        for declared_path in branch_changed_paths {
            if paths_overlap(changed_path, declared_path) {
                overlaps.push(PromotionBasePathOverlap {
                    side,
                    range_head_sha: range.head_sha.clone(),
                    changed_path: changed_path.clone(),
                    declared_path: declared_path.clone(),
                    kind: PromotionPathOverlapKind::BranchPath,
                });
            }
        }
    }
}

fn collect_compatibility_changes(
    request: &PromotionBaseLineageRequest,
) -> Vec<PromotionCompatibilityObjectChange> {
    request
        .compatibility_objects
        .iter()
        .filter(|state| state.tested_object_sha != state.current_object_sha)
        .map(|state| PromotionCompatibilityObjectChange {
            path: state.path.clone(),
            kind: state.kind,
            tested_object_sha: state.tested_object_sha.clone(),
            current_object_sha: state.current_object_sha.clone(),
        })
        .collect()
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

fn validate_git_sha(value: &str, field: &str) -> Result<(), PromotionBaseLineageError> {
    if value.len() != GIT_SHA_HEX_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PromotionBaseLineageError::new(format!(
            "{field} must be an exact lowercase 40-hex Git object id"
        )));
    }
    Ok(())
}

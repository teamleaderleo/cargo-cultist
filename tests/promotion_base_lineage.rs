#[allow(dead_code)]
#[path = "../src/promotion_base_lineage.rs"]
mod promotion_base_lineage;
#[allow(dead_code)]
#[path = "../src/promotion_receipt.rs"]
mod promotion_receipt;

use promotion_base_lineage::{
    PROMOTION_BASE_LINEAGE_SCHEMA_VERSION, PromotionBaseDeltaSide, PromotionBaseLineageRequest,
    PromotionBaseRange, PromotionBaseRelation, PromotionCompatibilityObjectKind,
    PromotionCompatibilityObjectState, evaluate_promotion_base_lineage,
};
use promotion_receipt::{
    CurrentPromotionState, PROMOTION_RECEIPT_SCHEMA_VERSION, PromotionPathOverlapKind,
    PromotionReceiptDisposition, PromotionReceiptReason, PromotionReceiptRequest,
    TestedPromotionState,
};

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 40).collect()
}

fn digest(byte: char) -> String {
    format!(
        "sha256:{}",
        std::iter::repeat_n(byte, 64).collect::<String>()
    )
}

fn object_state(
    path: &str,
    kind: PromotionCompatibilityObjectKind,
    tested: Option<&str>,
    current: Option<&str>,
) -> PromotionCompatibilityObjectState {
    PromotionCompatibilityObjectState {
        path: path.to_string(),
        kind,
        tested_object_sha: tested.map(str::to_string),
        current_object_sha: current.map(str::to_string),
    }
}

fn promotion() -> PromotionReceiptRequest {
    PromotionReceiptRequest {
        schema_version: PROMOTION_RECEIPT_SCHEMA_VERSION,
        tested: TestedPromotionState {
            head_sha: sha('a'),
            tree_sha: sha('b'),
            change_set_sha256: digest('a'),
            base_sha: sha('c'),
            base_tree_sha: sha('d'),
            effective_merge_tree_sha: sha('e'),
            successful_check_refs: vec!["github-actions:ci/tested".to_string()],
        },
        current: CurrentPromotionState {
            head_sha: sha('f'),
            tree_sha: sha('1'),
            change_set_sha256: digest('a'),
            base_sha: sha('2'),
            base_tree_sha: sha('3'),
            effective_merge_tree_sha: sha('4'),
            mergeable: true,
            conflict: false,
        },
        branch_changed_paths: vec!["src/feature.rs".to_string()],
        consumed_contract_paths: vec!["src/contracts".to_string()],
        applicable_policy_paths: vec![".github/workflows/ci.yml".to_string()],
        intervening_commits: Vec::new(),
        compatibility_scope_complete: false,
    }
}

fn unchanged_objects() -> Vec<PromotionCompatibilityObjectState> {
    vec![
        object_state(
            "src/contracts",
            PromotionCompatibilityObjectKind::ConsumedContract,
            Some(&sha('6')),
            Some(&sha('6')),
        ),
        object_state(
            ".github/workflows/ci.yml",
            PromotionCompatibilityObjectKind::ApplicablePolicy,
            Some(&sha('7')),
            Some(&sha('7')),
        ),
    ]
}

fn divergent() -> PromotionBaseLineageRequest {
    PromotionBaseLineageRequest {
        schema_version: PROMOTION_BASE_LINEAGE_SCHEMA_VERSION,
        promotion: promotion(),
        merge_base_sha: sha('5'),
        tested_base_only: Some(PromotionBaseRange {
            base_sha: sha('5'),
            head_sha: sha('c'),
            commit_count: 4,
            changed_paths: vec!["docs/old-only.md".to_string()],
        }),
        current_base_only: Some(PromotionBaseRange {
            base_sha: sha('5'),
            head_sha: sha('2'),
            commit_count: 34,
            changed_paths: vec!["docs/current-only.md".to_string()],
        }),
        compatibility_objects: unchanged_objects(),
        base_path_receipts_complete: true,
    }
}

#[test]
fn divergent_disjoint_base_delta_preserves_unknown_semantic_independence() {
    let request = divergent();
    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();

    assert_eq!(evaluation.base_relation, PromotionBaseRelation::Diverged);
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::InspectSemanticOverlap
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::SemanticIndependenceUnknown]
    );
    assert!(evaluation.branch_path_overlaps.is_empty());
    assert!(evaluation.compatibility_changes.is_empty());
    assert_eq!(
        evaluation.tested_base_only.as_ref().unwrap().commit_count,
        4
    );
    assert_eq!(
        evaluation.current_base_only.as_ref().unwrap().commit_count,
        34
    );
}

#[test]
fn identical_endpoint_contract_is_not_changed_just_because_both_sides_touched_path() {
    let mut request = divergent();
    request.tested_base_only.as_mut().unwrap().changed_paths =
        vec!["src/contracts/schema.rs".to_string()];
    request.current_base_only.as_mut().unwrap().changed_paths =
        vec!["src/contracts/schema.rs".to_string()];

    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::InspectSemanticOverlap
    );
    assert!(evaluation.compatibility_changes.is_empty());
    assert!(evaluation.branch_path_overlaps.is_empty());
}

#[test]
fn policy_endpoint_change_requires_rerun_without_false_contract_reason() {
    let mut request = divergent();
    let policy = request
        .compatibility_objects
        .iter_mut()
        .find(|state| state.kind == PromotionCompatibilityObjectKind::ApplicablePolicy)
        .unwrap();
    policy.current_object_sha = Some(sha('8'));

    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(evaluation.base_relation, PromotionBaseRelation::Diverged);
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ApplicablePolicyOverlap]
    );
    assert_eq!(evaluation.compatibility_changes.len(), 1);
    assert_eq!(
        evaluation.compatibility_changes[0].kind,
        PromotionCompatibilityObjectKind::ApplicablePolicy
    );
}

#[test]
fn removed_tested_side_consumed_contract_requires_rerun() {
    let mut request = divergent();
    let contract = request
        .compatibility_objects
        .iter_mut()
        .find(|state| state.kind == PromotionCompatibilityObjectKind::ConsumedContract)
        .unwrap();
    contract.current_object_sha = None;

    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ConsumedContractOverlap]
    );
    assert_eq!(evaluation.compatibility_changes.len(), 1);
}

#[test]
fn branch_path_collision_still_uses_divergent_range_receipt() {
    let mut request = divergent();
    request.current_base_only.as_mut().unwrap().changed_paths = vec!["src/feature.rs".to_string()];

    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::BranchPathOverlap]
    );
    assert_eq!(evaluation.branch_path_overlaps.len(), 1);
    assert_eq!(
        evaluation.branch_path_overlaps[0].side,
        PromotionBaseDeltaSide::CurrentBaseOnly
    );
    assert_eq!(
        evaluation.branch_path_overlaps[0].kind,
        PromotionPathOverlapKind::BranchPath
    );
}

#[test]
fn complete_compatibility_scope_can_reuse_divergent_disjoint_base() {
    let mut request = divergent();
    request.promotion.compatibility_scope_complete = true;

    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(evaluation.base_relation, PromotionBaseRelation::Diverged);
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::ReceiptReusable
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::CompleteCompatibilityScopeNoRelevantChange]
    );
}

#[test]
fn exact_effective_merge_tree_identity_overrides_endpoint_policy_change() {
    let mut request = divergent();
    request.promotion.current.effective_merge_tree_sha =
        request.promotion.tested.effective_merge_tree_sha.clone();
    let policy = request
        .compatibility_objects
        .iter_mut()
        .find(|state| state.kind == PromotionCompatibilityObjectKind::ApplicablePolicy)
        .unwrap();
    policy.current_object_sha = Some(sha('8'));

    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::ReceiptReusable
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ExactEffectiveMergeTreeIdentity]
    );
    assert_eq!(evaluation.compatibility_changes.len(), 1);
}

#[test]
fn forward_and_rewind_relations_are_explicit() {
    let mut forward = PromotionBaseLineageRequest {
        schema_version: PROMOTION_BASE_LINEAGE_SCHEMA_VERSION,
        promotion: promotion(),
        merge_base_sha: sha('c'),
        tested_base_only: None,
        current_base_only: Some(PromotionBaseRange {
            base_sha: sha('c'),
            head_sha: sha('2'),
            commit_count: 3,
            changed_paths: vec!["docs/forward.md".to_string()],
        }),
        compatibility_objects: unchanged_objects(),
        base_path_receipts_complete: true,
    };
    let evaluation = evaluate_promotion_base_lineage(&forward).unwrap();
    assert_eq!(evaluation.base_relation, PromotionBaseRelation::Forward);

    forward.promotion.tested.base_sha = sha('6');
    forward.promotion.current.base_sha = sha('c');
    forward.merge_base_sha = sha('c');
    forward.current_base_only = None;
    forward.tested_base_only = Some(PromotionBaseRange {
        base_sha: sha('c'),
        head_sha: sha('6'),
        commit_count: 2,
        changed_paths: vec!["docs/rewind.md".to_string()],
    });
    let evaluation = evaluate_promotion_base_lineage(&forward).unwrap();
    assert_eq!(evaluation.base_relation, PromotionBaseRelation::Rewind);
}

#[test]
fn equal_base_trees_are_same_tree_and_require_equal_compatibility_objects() {
    let mut request = divergent();
    request.promotion.current.base_tree_sha = request.promotion.tested.base_tree_sha.clone();
    request.tested_base_only = None;
    request.current_base_only = None;
    let evaluation = evaluate_promotion_base_lineage(&request).unwrap();
    assert_eq!(evaluation.base_relation, PromotionBaseRelation::SameTree);
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::ReceiptReusable
    );

    let policy = request
        .compatibility_objects
        .iter_mut()
        .find(|state| state.kind == PromotionCompatibilityObjectKind::ApplicablePolicy)
        .unwrap();
    policy.current_object_sha = Some(sha('8'));
    assert!(evaluate_promotion_base_lineage(&request).is_err());
}

#[test]
fn compatibility_object_receipts_must_exactly_cover_declared_contracts_and_policies() {
    let mut request = divergent();
    request.compatibility_objects.pop();
    let error = evaluate_promotion_base_lineage(&request).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("must exactly cover every declared consumed contract")
    );
}

#[test]
fn changed_base_lineage_rejects_incomplete_path_receipt_attestation() {
    let mut request = divergent();
    request.base_path_receipts_complete = false;
    let error = evaluate_promotion_base_lineage(&request).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("requires complete base-range changed-path receipts")
    );
}

#[test]
fn range_endpoints_must_bind_exact_declared_bases() {
    let mut request = divergent();
    request.current_base_only.as_mut().unwrap().head_sha = sha('7');
    let error = evaluate_promotion_base_lineage(&request).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("exact corresponding promotion base")
    );
}

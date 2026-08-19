#[allow(dead_code)]
#[path = "../src/promotion_receipt.rs"]
mod promotion_receipt;

use promotion_receipt::{
    CurrentPromotionState, InterveningCommit, PROMOTION_RECEIPT_SCHEMA_VERSION,
    PromotionPathOverlapKind, PromotionReceiptDisposition, PromotionReceiptReason,
    PromotionReceiptRequest, TestedPromotionState, evaluate_promotion_receipt,
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

fn request() -> PromotionReceiptRequest {
    PromotionReceiptRequest {
        schema_version: PROMOTION_RECEIPT_SCHEMA_VERSION,
        tested: TestedPromotionState {
            head_sha: sha('a'),
            tree_sha: sha('b'),
            change_set_sha256: digest('a'),
            base_sha: sha('c'),
            base_tree_sha: sha('d'),
            effective_merge_tree_sha: sha('e'),
            successful_check_refs: vec!["github-actions:ci/123".to_string()],
        },
        current: CurrentPromotionState {
            head_sha: sha('f'),
            tree_sha: sha('8'),
            change_set_sha256: digest('a'),
            base_sha: sha('1'),
            base_tree_sha: sha('2'),
            effective_merge_tree_sha: sha('3'),
            mergeable: true,
            conflict: false,
        },
        branch_changed_paths: vec!["src/feature.rs".to_string()],
        consumed_contract_paths: vec!["src/contracts".to_string()],
        applicable_policy_paths: vec![".github/workflows/ci.yml".to_string()],
        intervening_commits: vec![InterveningCommit {
            sha: sha('4'),
            changed_paths: vec!["docs/unrelated.md".to_string()],
        }],
        compatibility_scope_complete: false,
    }
}

#[test]
fn exact_effective_merge_tree_identity_reuses_receipt() {
    let mut request = request();
    request.current.base_sha = request.tested.base_sha.clone();
    request.current.base_tree_sha = request.tested.base_tree_sha.clone();
    request.current.effective_merge_tree_sha = request.tested.effective_merge_tree_sha.clone();
    request.intervening_commits.clear();

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::ReceiptReusable
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ExactEffectiveMergeTreeIdentity]
    );
    assert_eq!(evaluation.tested_head_sha, sha('a'));
    assert_eq!(evaluation.tested_tree_sha, sha('b'));
    assert_eq!(evaluation.current_head_sha, sha('f'));
    assert_eq!(evaluation.current_tree_sha, sha('8'));
    assert_eq!(evaluation.tested_change_set_sha256, digest('a'));
    assert_eq!(evaluation.current_change_set_sha256, digest('a'));
    assert_eq!(
        evaluation.successful_check_refs,
        vec!["github-actions:ci/123"]
    );
}

#[test]
fn reanchor_with_same_change_set_and_base_tree_reuses_receipt_even_if_full_tree_changes() {
    let mut request = request();
    request.current.base_tree_sha = request.tested.base_tree_sha.clone();
    request.current.base_sha = sha('5');
    request.current.effective_merge_tree_sha = sha('6');
    request.intervening_commits.clear();

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::ReceiptReusable
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::EquivalentChangeSetSameBaseTree]
    );
    assert_ne!(evaluation.tested_tree_sha, evaluation.current_tree_sha);
    assert_eq!(
        evaluation.tested_change_set_sha256,
        evaluation.current_change_set_sha256
    );
}

#[test]
fn reanchor_with_same_change_set_reaches_overlap_analysis_instead_of_tree_rerun() {
    let request = request();
    assert_ne!(request.tested.tree_sha, request.current.tree_sha);
    assert_eq!(
        request.tested.change_set_sha256,
        request.current.change_set_sha256
    );

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::InspectSemanticOverlap
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::SemanticIndependenceUnknown]
    );
}

#[test]
fn intervening_branch_path_overlap_requires_rerun() {
    let mut request = request();
    request.intervening_commits[0].changed_paths = vec!["src/feature.rs".to_string()];

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::BranchPathOverlap]
    );
    assert_eq!(evaluation.overlaps.len(), 1);
    assert_eq!(
        evaluation.overlaps[0].kind,
        PromotionPathOverlapKind::BranchPath
    );
}

#[test]
fn consumed_contract_prefix_overlap_requires_rerun() {
    let mut request = request();
    request.intervening_commits[0].changed_paths = vec!["src/contracts/schema.rs".to_string()];

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ConsumedContractOverlap]
    );
    assert_eq!(evaluation.overlaps[0].declared_path, "src/contracts");
}

#[test]
fn applicable_policy_change_requires_rerun() {
    let mut request = request();
    request.intervening_commits[0].changed_paths = vec![".github/workflows/ci.yml".to_string()];

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ApplicablePolicyOverlap]
    );
}

#[test]
fn path_disjoint_base_movement_remains_semantically_unknown() {
    let request = request();
    let evaluation = evaluate_promotion_receipt(&request).unwrap();

    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::InspectSemanticOverlap
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::SemanticIndependenceUnknown]
    );
    assert!(evaluation.overlaps.is_empty());
    assert_eq!(evaluation.intervening_commit_shas, vec![sha('4')]);
}

#[test]
fn explicitly_complete_compatibility_scope_can_reuse_disjoint_receipt() {
    let mut request = request();
    request.compatibility_scope_complete = true;

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::ReceiptReusable
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::CompleteCompatibilityScopeNoRelevantChange]
    );
    assert!(evaluation.compatibility_scope_complete);
}

#[test]
fn merge_conflict_or_nonmergeable_state_requires_reconcile_first() {
    let mut conflicted = request();
    conflicted.current.conflict = true;
    let evaluation = evaluate_promotion_receipt(&conflicted).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::MergeConflict]
    );

    let mut nonmergeable = request();
    nonmergeable.current.mergeable = false;
    let evaluation = evaluate_promotion_receipt(&nonmergeable).unwrap();
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::NotMergeable]
    );
}

#[test]
fn changed_change_set_requires_rerun_before_overlap_inference() {
    let mut request = request();
    request.current.change_set_sha256 = digest('7');

    let evaluation = evaluate_promotion_receipt(&request).unwrap();
    assert_eq!(
        evaluation.disposition,
        PromotionReceiptDisposition::RerunRequired
    );
    assert_eq!(
        evaluation.reasons,
        vec![PromotionReceiptReason::ChangeSetChanged]
    );
}

#[test]
fn malformed_change_set_digest_rejects() {
    let mut request = request();
    request.current.change_set_sha256 = sha('a');

    let error = evaluate_promotion_receipt(&request).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("change_set_sha256 must use sha256:<hex>")
    );
}

#[test]
fn changed_base_tree_requires_intervening_commit_receipts() {
    let mut request = request();
    request.intervening_commits.clear();

    let error = evaluate_promotion_receipt(&request).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("base tree changed but no intervening commits")
    );
}

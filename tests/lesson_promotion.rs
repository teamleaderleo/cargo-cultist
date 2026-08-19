#[path = "../src/project_memory.rs"]
mod project_memory;
#[path = "../src/lesson_promotion.rs"]
mod lesson_promotion;

use lesson_promotion::{
    MAX_LESSON_PROMOTION_BYTES, PromotionStatus, evaluate_lesson_promotion,
    parse_lesson_promotion_claim,
};
use project_memory::{ArtifactKind, ArtifactRef, parse_project_memory_packet};

const MEMORY: &[u8] = include_bytes!("../research/project-memory/stensibly-1575.json");
const CLAIM: &[u8] = include_bytes!("../research/lesson-promotion/stensibly-1575.json");

fn inputs() -> (
    project_memory::ProjectMemoryPacket,
    lesson_promotion::LessonPromotionClaim,
) {
    (
        parse_project_memory_packet(MEMORY).unwrap(),
        parse_lesson_promotion_claim(CLAIM).unwrap(),
    )
}

fn pr(number: u64) -> ArtifactRef {
    ArtifactRef {
        kind: ArtifactKind::PullRequest,
        number,
    }
}

#[test]
fn retained_stensibly_episode_is_an_observed_promotion() {
    let (memory, claim) = inputs();
    let evaluation = evaluate_lesson_promotion(&memory, &claim).unwrap();

    assert_eq!(evaluation.status, PromotionStatus::ObservedPromotion);
    assert_eq!(evaluation.same_class_repairs, vec![pr(1571), pr(1573)]);
    assert_eq!(evaluation.adjacent_different_class, vec![pr(1569)]);
    assert_eq!(evaluation.guard, pr(1575));
    assert_eq!(
        evaluation.enforcement_path,
        "test/convex-index-identifier-limit.test.ts"
    );
    assert_eq!(evaluation.scope_ref, "convex/**/*.ts");
    assert!(evaluation.missing_repair_coverage.is_empty());
    assert!(evaluation.different_class_coverage.is_empty());
    assert!(!evaluation.automatic_policy_authority);
}

#[test]
fn adjacent_different_class_predecessor_cannot_count_as_guard_coverage() {
    let (memory, mut claim) = inputs();
    claim.guard.covered_repairs.push(pr(1569));

    let evaluation = evaluate_lesson_promotion(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        PromotionStatus::GuardCoverageIncludesDifferentClass
    );
    assert_eq!(evaluation.different_class_coverage, vec![pr(1569)]);
}

#[test]
fn missing_same_class_repair_keeps_guard_coverage_incomplete() {
    let (memory, mut claim) = inputs();
    claim.guard.covered_repairs.retain(|item| *item != pr(1573));

    let evaluation = evaluate_lesson_promotion(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, PromotionStatus::GuardCoverageIncomplete);
    assert_eq!(evaluation.missing_repair_coverage, vec![pr(1573)]);
}

#[test]
fn one_repair_is_insufficient_for_repeated_repair_promotion() {
    let (memory, mut claim) = inputs();
    claim.repair_evidence.truncate(1);
    claim.guard.covered_repairs = vec![pr(1571)];

    let evaluation = evaluate_lesson_promotion(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        PromotionStatus::InsufficientRepeatedRepairs
    );
}

#[test]
fn mismatched_guard_discriminator_stays_separate() {
    let (memory, mut claim) = inputs();
    claim.guard.value_ref = "node_runtime_bundle".to_string();

    let evaluation = evaluate_lesson_promotion(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, PromotionStatus::GuardClassMismatch);
}

#[test]
fn unmerged_common_guard_is_only_proposed() {
    let (mut memory, claim) = inputs();
    let guard = memory
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == pr(1575))
        .unwrap();
    guard.revision.as_mut().unwrap().merged = false;

    let evaluation = evaluate_lesson_promotion(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, PromotionStatus::ProposedGuard);
}

#[test]
fn different_class_predecessor_cannot_be_relabelled_as_same_class_without_marker() {
    let (memory, mut claim) = inputs();
    let adjacent = claim.adjacent_predecessors.remove(0);
    claim.repair_evidence[0].artifact = adjacent.artifact;
    claim.repair_evidence[0].source_evidence = adjacent.source_evidence;

    let error = evaluate_lesson_promotion(&memory, &claim).unwrap_err();
    assert!(error.contains("does not contain candidate marker"));
}

#[test]
fn invented_source_excerpt_is_rejected() {
    let (memory, mut claim) = inputs();
    claim.repair_evidence[0].source_evidence =
        "This repair definitely proves the universal rule.".to_string();

    let error = evaluate_lesson_promotion(&memory, &claim).unwrap_err();
    assert!(error.contains("absent from retained project-memory text"));
}

#[test]
fn guard_enforcement_path_must_be_in_guard_diff() {
    let (memory, mut claim) = inputs();
    claim.guard.enforcement_path = "convex/schema.ts".to_string();

    let error = evaluate_lesson_promotion(&memory, &claim).unwrap_err();
    assert!(error.contains("absent from pr#1575 changed paths"));
}

#[test]
fn claim_input_is_bounded_before_json_parse() {
    let bytes = vec![b' '; MAX_LESSON_PROMOTION_BYTES + 1];
    let error = parse_lesson_promotion_claim(&bytes).unwrap_err();
    assert!(error.contains("exceeds"));
}

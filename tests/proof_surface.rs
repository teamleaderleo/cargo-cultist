#[path = "../src/project_memory.rs"]
mod project_memory;
#[path = "../src/proof_surface.rs"]
mod proof_surface;

use project_memory::{ArtifactKind, ArtifactRef, parse_project_memory_packet};
use proof_surface::{
    MAX_PROOF_SURFACE_BYTES, ProofArtifactKind, ProofSurfaceStatus, evaluate_proof_surface,
    parse_proof_surface_claim,
};

const MEMORY: &[u8] = include_bytes!("../research/project-memory/stensibly-1515.json");
const CLAIM: &[u8] = include_bytes!("../research/proof-surface/stensibly-1515.json");

fn inputs() -> (
    project_memory::ProjectMemoryPacket,
    proof_surface::ProofSurfaceClaim,
) {
    let memory = parse_project_memory_packet(MEMORY).unwrap();
    memory.summary().unwrap();
    let claim = parse_proof_surface_claim(CLAIM).unwrap();
    (memory, claim)
}

fn pr(number: u64) -> ArtifactRef {
    ArtifactRef {
        kind: ArtifactKind::PullRequest,
        number,
    }
}

#[test]
fn retained_stensibly_episode_is_an_observed_surface_mismatch() {
    let (memory, claim) = inputs();
    let evaluation = evaluate_proof_surface(&memory, &claim).unwrap();

    assert_eq!(
        evaluation.status,
        ProofSurfaceStatus::ObservedProofSurfaceMismatch
    );
    assert_eq!(evaluation.subject, pr(1515));
    assert!(evaluation.behavior_passed);
    assert_eq!(
        evaluation.required_artifact_kind,
        ProofArtifactKind::IssueConversationComment
    );
    assert_eq!(
        evaluation.produced_artifact_kind,
        Some(ProofArtifactKind::PullRequestReview)
    );
    assert!(!evaluation.proof_valid);
    assert!(!evaluation.automatic_behavior_failure);
    assert!(!evaluation.automatic_acceptance);
}

#[test]
fn semantic_event_type_wins_over_conversation_like_body_text() {
    let (memory, claim) = inputs();
    assert!(claim.provider_event.body.contains("conversation comment"));

    let evaluation = evaluate_proof_surface(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.produced_artifact_kind,
        Some(ProofArtifactKind::PullRequestReview)
    );
    assert_eq!(
        evaluation.status,
        ProofSurfaceStatus::ObservedProofSurfaceMismatch
    );
}

#[test]
fn matching_issue_conversation_comment_is_a_valid_surface() {
    let (memory, mut claim) = inputs();
    claim.provider_event.url = format!(
        "https://github.com/teamleaderleo/stensibly/pull/1515#issuecomment-{}",
        claim.provider_event.id
    );
    claim.provider_event.review_state = None;

    let evaluation = evaluate_proof_surface(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, ProofSurfaceStatus::ProofSurfaceMatched);
    assert_eq!(
        evaluation.produced_artifact_kind,
        Some(ProofArtifactKind::IssueConversationComment)
    );
    assert!(evaluation.proof_valid);
    assert!(evaluation.behavior_passed);
}

#[test]
fn behavior_success_must_be_explicit_in_retained_source() {
    let (memory, mut claim) = inputs();
    claim.behavior_marker = "behavior success marker absent".to_string();

    let evaluation = evaluate_proof_surface(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ProofSurfaceStatus::BehaviorEvidenceMissing
    );
    assert!(!evaluation.behavior_passed);
}

#[test]
fn proof_requirement_must_be_explicit_in_retained_source() {
    let (memory, mut claim) = inputs();
    claim.requirement_evidence = claim.behavior_evidence.clone();

    let evaluation = evaluate_proof_surface(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ProofSurfaceStatus::RequirementEvidenceMissing
    );
}

#[test]
fn provider_event_body_must_match_the_retained_event() {
    let (memory, mut claim) = inputs();
    claim.provider_body_marker = "provider event marker absent".to_string();

    let evaluation = evaluate_proof_surface(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ProofSurfaceStatus::ProviderEventBodyMissing
    );
}

#[test]
fn provider_event_url_and_id_must_classify_together() {
    let (memory, mut claim) = inputs();
    claim.provider_event.url =
        "https://github.com/teamleaderleo/stensibly/pull/1515#pullrequestreview-1".to_string();

    let evaluation = evaluate_proof_surface(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ProofSurfaceStatus::ProducedArtifactUnclassifiable
    );
    assert_eq!(evaluation.produced_artifact_kind, None);
    assert!(!evaluation.proof_valid);
}

#[test]
fn provider_event_must_belong_to_the_subject_pull_request() {
    let (memory, mut claim) = inputs();
    claim.provider_event.url = format!(
        "https://github.com/teamleaderleo/stensibly/pull/1514#pullrequestreview-{}",
        claim.provider_event.id
    );

    let evaluation = evaluate_proof_surface(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ProofSurfaceStatus::ProducedArtifactUnclassifiable
    );
}

#[test]
fn required_kind_and_source_marker_are_bound_together() {
    let (memory, mut claim) = inputs();
    claim.required_artifact_kind = ProofArtifactKind::PullRequestReview;

    let error = evaluate_proof_surface(&memory, &claim).unwrap_err();
    assert!(error.contains("requirement_marker"));
}

#[test]
fn invented_behavior_excerpt_is_rejected() {
    let (memory, mut claim) = inputs();
    claim.behavior_evidence = "The behavior definitely worked in every possible sense.".to_string();

    let error = evaluate_proof_surface(&memory, &claim).unwrap_err();
    assert!(error.contains("absent from retained project-memory text"));
}

#[test]
fn claim_input_is_bounded_before_json_parse() {
    let bytes = vec![b' '; MAX_PROOF_SURFACE_BYTES + 1];
    let error = parse_proof_surface_claim(&bytes).unwrap_err();
    assert!(error.contains("exceeds"));
}

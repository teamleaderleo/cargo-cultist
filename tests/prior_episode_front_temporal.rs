#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/closure_episode.rs"]
mod closure_episode;
#[path = "../src/lesson_promotion.rs"]
mod lesson_promotion;
#[path = "../src/observation_reconciliation.rs"]
mod observation_reconciliation;
#[path = "../src/prior_episode_front.rs"]
mod prior_episode_front;
#[path = "../src/project_memory.rs"]
mod project_memory;
#[path = "../src/proof_surface.rs"]
mod proof_surface;
#[path = "../src/proxy_revision.rs"]
mod proxy_revision;
#[path = "../src/review_memory.rs"]
mod review_memory;

use lesson_promotion::{PromotionStatus, parse_lesson_promotion_claim};
use observation_reconciliation::{
    ObservationReconciliationStatus, parse_observation_reconciliation_claim,
};
use prior_episode_front::{
    PRIOR_EPISODE_FRONT_SCHEMA_VERSION, PriorEpisodeFrontItem, PriorEpisodeFrontQuery,
    PriorEpisodeInput, PriorEpisodeNextAction, evaluate_prior_episode_front,
    parse_prior_episode_front_query,
};
use project_memory::{ArtifactKind, ArtifactRef, parse_project_memory_packet};
use proof_surface::{ProofSurfaceStatus, parse_proof_surface_claim};
use proxy_revision::{ProxyRevisionStatus, parse_proxy_revision_claim};

fn memory(bytes: &[u8]) -> project_memory::ProjectMemoryPacket {
    let packet = parse_project_memory_packet(bytes).unwrap();
    packet.summary().unwrap();
    packet
}

fn query() -> PriorEpisodeFrontQuery {
    PriorEpisodeFrontQuery {
        schema_version: PRIOR_EPISODE_FRONT_SCHEMA_VERSION,
        inputs: vec![
            PriorEpisodeInput::LessonPromotion {
                id: "stensibly:index-limit-guard".to_string(),
                memory: Box::new(memory(include_bytes!(
                    "../research/project-memory/stensibly-1575.json"
                ))),
                claim: Box::new(
                    parse_lesson_promotion_claim(include_bytes!(
                        "../research/lesson-promotion/stensibly-1575.json"
                    ))
                    .unwrap(),
                ),
            },
            PriorEpisodeInput::ProxyRevision {
                id: "stensibly:responsibility-proxy".to_string(),
                memory: Box::new(memory(include_bytes!(
                    "../research/project-memory/stensibly-1604-1605.json"
                ))),
                claim: Box::new(
                    parse_proxy_revision_claim(include_bytes!(
                        "../research/proxy-revision/stensibly-1604-1605.json"
                    ))
                    .unwrap(),
                ),
            },
            PriorEpisodeInput::ObservationReconciliation {
                id: "stensibly:worker-origin-convergence".to_string(),
                memory: Box::new(memory(include_bytes!(
                    "../research/project-memory/stensibly-1609-1610.json"
                ))),
                claim: Box::new(
                    parse_observation_reconciliation_claim(include_bytes!(
                        "../research/observation-reconciliation/stensibly-1609-1610.json"
                    ))
                    .unwrap(),
                ),
            },
            PriorEpisodeInput::ProofSurface {
                id: "stensibly:r5q7-proof-surface".to_string(),
                memory: Box::new(memory(include_bytes!(
                    "../research/project-memory/stensibly-1515.json"
                ))),
                claim: Box::new(
                    parse_proof_surface_claim(include_bytes!(
                        "../research/proof-surface/stensibly-1515.json"
                    ))
                    .unwrap(),
                ),
            },
        ],
    }
}

#[test]
fn four_retained_temporal_species_project_to_exact_next_actions_in_order() {
    let serialized = serde_json::to_vec(&query()).unwrap();
    let parsed = parse_prior_episode_front_query(&serialized).unwrap();
    let front = evaluate_prior_episode_front(&parsed).unwrap();

    assert_eq!(front.items.len(), 4);
    assert!(front.quiet.is_empty());

    let PriorEpisodeFrontItem::LessonPromotion {
        id,
        next,
        evaluation,
    } = &front.items[0]
    else {
        panic!("expected lesson-promotion item first")
    };
    assert_eq!(id, "stensibly:index-limit-guard");
    assert_eq!(*next, PriorEpisodeNextAction::UseAcceptedGuard);
    assert_eq!(evaluation.status, PromotionStatus::ObservedPromotion);
    assert!(!evaluation.automatic_policy_authority);

    let PriorEpisodeFrontItem::ProxyRevision {
        id,
        next,
        evaluation,
    } = &front.items[1]
    else {
        panic!("expected proxy-revision item second")
    };
    assert_eq!(id, "stensibly:responsibility-proxy");
    assert_eq!(*next, PriorEpisodeNextAction::UseCorrectedPredicate);
    assert_eq!(evaluation.status, ProxyRevisionStatus::ObservedProxyRevision);
    assert!(!evaluation.automatic_generalization_authority);

    let PriorEpisodeFrontItem::ObservationReconciliation {
        id,
        next,
        evaluation,
    } = &front.items[2]
    else {
        panic!("expected observation-reconciliation item third")
    };
    assert_eq!(id, "stensibly:worker-origin-convergence");
    assert_eq!(*next, PriorEpisodeNextAction::AwaitBoundedConvergence);
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::ObservedReconciliation
    );
    assert!(!evaluation.automatic_authority_change);

    let PriorEpisodeFrontItem::ProofSurface {
        id,
        next,
        evaluation,
    } = &front.items[3]
    else {
        panic!("expected proof-surface item fourth")
    };
    assert_eq!(id, "stensibly:r5q7-proof-surface");
    assert_eq!(*next, PriorEpisodeNextAction::ProduceRequiredProofArtifact);
    assert_eq!(
        evaluation.status,
        ProofSurfaceStatus::ObservedProofSurfaceMismatch
    );
    assert!(evaluation.behavior_passed);
    assert!(!evaluation.proof_valid);
}

#[test]
fn selected_temporal_episode_with_incomplete_disposition_fails_closed_with_local_id() {
    let mut query = query();
    let PriorEpisodeInput::LessonPromotion { claim, .. } = &mut query.inputs[0] else {
        panic!("expected lesson-promotion input")
    };
    claim.guard.covered_repairs.retain(|reference| {
        *reference
            != ArtifactRef {
                kind: ArtifactKind::PullRequest,
                number: 1573,
            }
    });

    let error = evaluate_prior_episode_front(&query).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("`stensibly:index-limit-guard`"));
    assert!(message.contains("unsupported lesson-promotion status GuardCoverageIncomplete"));
}

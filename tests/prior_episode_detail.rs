#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/closure_episode.rs"]
mod closure_episode;
#[path = "../src/lesson_promotion.rs"]
mod lesson_promotion;
#[path = "../src/observation_reconciliation.rs"]
mod observation_reconciliation;
#[path = "../src/prior_episode_detail.rs"]
mod prior_episode_detail;
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

use lesson_promotion::parse_lesson_promotion_claim;
use prior_episode_detail::{PriorEpisodeDetail, project_prior_episode_detail};
use prior_episode_front::{PriorEpisodeInput, PriorEpisodeNextAction};
use project_memory::{ArtifactKind, ArtifactRef, parse_project_memory_packet};
use proxy_revision::parse_proxy_revision_claim;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TrialSpec {
    oracle: Oracle,
}

#[derive(Debug, Deserialize)]
struct Oracle {
    max_identifier_length: usize,
    proposed_identifier_length: usize,
}

fn lesson_input() -> PriorEpisodeInput {
    let memory = parse_project_memory_packet(include_bytes!(
        "../research/project-memory/stensibly-1575.json"
    ))
    .unwrap();
    memory.summary().unwrap();
    let claim = parse_lesson_promotion_claim(include_bytes!(
        "../research/lesson-promotion/stensibly-1575.json"
    ))
    .unwrap();

    PriorEpisodeInput::LessonPromotion {
        id: "stensibly:index-limit-guard".to_string(),
        memory: Box::new(memory),
        claim: Box::new(claim),
    }
}

#[test]
fn selected_guard_detail_recovers_only_the_accepted_operational_evidence() {
    let detail = project_prior_episode_detail(&lesson_input()).unwrap();
    let PriorEpisodeDetail::AcceptedGuard {
        id,
        next,
        candidate_value_ref,
        operational_marker,
        guard,
        guard_source_evidence,
        enforcement_path,
        scope_ref,
        same_class_repairs,
        automatic_policy_authority,
        ..
    } = &detail;

    assert_eq!(id, "stensibly:index-limit-guard");
    assert_eq!(*next, PriorEpisodeNextAction::UseAcceptedGuard);
    assert_eq!(candidate_value_ref, "index_identifier_limit");
    assert_eq!(operational_marker, "64-character identifier limit");
    assert_eq!(
        *guard,
        ArtifactRef {
            kind: ArtifactKind::PullRequest,
            number: 1575,
        }
    );
    assert!(guard_source_evidence.contains("fails when any exceed 64 characters"));
    assert_eq!(
        enforcement_path,
        "test/convex-index-identifier-limit.test.ts"
    );
    assert_eq!(scope_ref, "convex/**/*.ts");
    assert_eq!(
        same_class_repairs,
        &vec![
            ArtifactRef {
                kind: ArtifactKind::PullRequest,
                number: 1571,
            },
            ArtifactRef {
                kind: ArtifactKind::PullRequest,
                number: 1573,
            },
        ]
    );
    assert!(!automatic_policy_authority);

    let serialized = serde_json::to_string(&detail).unwrap();
    assert!(serialized.contains("64-character identifier limit"));
    assert!(serialized.contains("fails when any exceed 64 characters"));
    assert!(!serialized.contains("node_runtime_bundle"));
    assert!(!serialized.contains("node:crypto"));
    assert!(!serialized.contains("1569"));

    let trial: TrialSpec = serde_json::from_slice(include_bytes!(
        "../research/capability-demand-retirement/stensibly-convex-index-review-v1.json"
    ))
    .unwrap();
    assert_eq!(trial.oracle.max_identifier_length, 64);
    assert_eq!(trial.oracle.proposed_identifier_length, 68);
    assert!(trial.oracle.proposed_identifier_length > trial.oracle.max_identifier_length);
}

#[test]
fn incomplete_selected_guard_is_rejected_before_detail_projection() {
    let mut input = lesson_input();
    let PriorEpisodeInput::LessonPromotion { claim, .. } = &mut input else {
        panic!("expected lesson promotion input")
    };
    claim.guard.covered_repairs.retain(|reference| {
        *reference
            != ArtifactRef {
                kind: ArtifactKind::PullRequest,
                number: 1573,
            }
    });

    let error = project_prior_episode_detail(&input).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("selected prior episode is not actionable")
    );
    assert!(error.to_string().contains("GuardCoverageIncomplete"));
}

#[test]
fn v1_refuses_unimplemented_temporal_detail_kinds() {
    let memory = parse_project_memory_packet(include_bytes!(
        "../research/project-memory/stensibly-1604-1605.json"
    ))
    .unwrap();
    memory.summary().unwrap();
    let claim = parse_proxy_revision_claim(include_bytes!(
        "../research/proxy-revision/stensibly-1604-1605.json"
    ))
    .unwrap();
    let input = PriorEpisodeInput::ProxyRevision {
        id: "stensibly:responsibility-proxy".to_string(),
        memory: Box::new(memory),
        claim: Box::new(claim),
    };

    let error = project_prior_episode_detail(&input).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("supports lesson_promotion inputs only")
    );
}

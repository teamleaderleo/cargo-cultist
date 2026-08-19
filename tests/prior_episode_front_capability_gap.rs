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
use prior_episode_front::{
    PRIOR_EPISODE_FRONT_SCHEMA_VERSION, PriorEpisodeFrontItem, PriorEpisodeFrontQuery,
    PriorEpisodeInput, PriorEpisodeNextAction, evaluate_prior_episode_front,
};
use project_memory::parse_project_memory_packet;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TrialSpec {
    oracle: Oracle,
}

#[derive(Debug, Deserialize)]
struct Oracle {
    blocking_reason: String,
    max_identifier_length: usize,
    proposed_identifier_length: usize,
    corrective_action: String,
}

#[test]
fn selected_guard_action_does_not_yet_carry_the_oracles_operational_limit() {
    let memory = parse_project_memory_packet(include_bytes!(
        "../research/project-memory/stensibly-1575.json"
    ))
    .unwrap();
    memory.summary().unwrap();
    let claim = parse_lesson_promotion_claim(include_bytes!(
        "../research/lesson-promotion/stensibly-1575.json"
    ))
    .unwrap();

    assert_eq!(claim.repair_marker, "64-character identifier limit");

    let query = PriorEpisodeFrontQuery {
        schema_version: PRIOR_EPISODE_FRONT_SCHEMA_VERSION,
        inputs: vec![PriorEpisodeInput::LessonPromotion {
            id: "stensibly:index-limit-guard".to_string(),
            memory: Box::new(memory),
            claim: Box::new(claim),
        }],
    };
    let front = evaluate_prior_episode_front(&query).unwrap();
    assert_eq!(front.items.len(), 1);

    let PriorEpisodeFrontItem::LessonPromotion {
        next, evaluation, ..
    } = &front.items[0]
    else {
        panic!("expected lesson-promotion front item")
    };
    assert_eq!(*next, PriorEpisodeNextAction::UseAcceptedGuard);
    assert_eq!(evaluation.status, PromotionStatus::ObservedPromotion);
    assert_eq!(evaluation.candidate_value_ref, "index_identifier_limit");
    assert_eq!(
        evaluation.enforcement_path,
        "test/convex-index-identifier-limit.test.ts"
    );
    assert_eq!(evaluation.scope_ref, "convex/**/*.ts");
    assert!(!evaluation.automatic_policy_authority);

    let worker_visible_front = serde_json::to_string(&front).unwrap();
    assert!(worker_visible_front.contains("use_accepted_guard"));
    assert!(worker_visible_front.contains("index_identifier_limit"));
    assert!(!worker_visible_front.contains("64-character identifier limit"));
    assert!(!worker_visible_front.contains("max_identifier_length"));
    assert!(!worker_visible_front.contains("corrective_action"));

    let trial: TrialSpec = serde_json::from_slice(include_bytes!(
        "../research/capability-demand-retirement/stensibly-convex-index-review-v1.json"
    ))
    .unwrap();
    assert_eq!(trial.oracle.blocking_reason, "convex_index_identifier_limit");
    assert_eq!(trial.oracle.max_identifier_length, 64);
    assert_eq!(trial.oracle.proposed_identifier_length, 68);
    assert_eq!(
        trial.oracle.corrective_action,
        "shorten_identifier_preserve_field_order"
    );
}

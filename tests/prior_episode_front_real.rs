#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/closure_episode.rs"]
mod closure_episode;
#[path = "../src/prior_episode_front.rs"]
mod prior_episode_front;
#[path = "../src/review_memory.rs"]
mod review_memory;

use applicability::ApplicabilityStatus;
use closure_episode::{ClearanceStatus, ClosureKind, IssueState};
use prior_episode_front::{
    PriorEpisodeFrontItem, PriorEpisodeNextAction, evaluate_prior_episode_front,
    parse_prior_episode_front_query,
};
use review_memory::ReviewThreadDisposition;

#[test]
fn pr_agent_2424_front_preserves_stale_review_invalidation() {
    let query = parse_prior_episode_front_query(include_bytes!(
        "../research/prior-episode-front/pr-agent-2424.json"
    ))
    .unwrap();
    let front = evaluate_prior_episode_front(&query).unwrap();

    assert_eq!(front.items.len(), 1);
    assert!(front.quiet.is_empty());
    let PriorEpisodeFrontItem::Review {
        id,
        next,
        evaluation,
    } = &front.items[0]
    else {
        panic!("expected review item")
    };
    assert_eq!(id, "pr-agent:2424:github-fallback-publish-state");
    assert_eq!(
        *next,
        PriorEpisodeNextAction::RecomputeAndRefreshReviewThread
    );
    assert_eq!(
        evaluation.disposition,
        ReviewThreadDisposition::RefreshExistingThread
    );
    assert_eq!(evaluation.matches.len(), 1);
    assert_eq!(
        evaluation.matches[0].applicability.status,
        ApplicabilityStatus::Invalid
    );
    assert_eq!(
        evaluation.matches[0].source_ref,
        "github:pull/2424/review-comment/3355870564"
    );
}

#[test]
fn claude_code_rereport_front_keeps_clearance_unknown_after_second_close() {
    let query = parse_prior_episode_front_query(include_bytes!(
        "../research/prior-episode-front/claude-code-57507.json"
    ))
    .unwrap();
    let front = evaluate_prior_episode_front(&query).unwrap();

    assert_eq!(front.items.len(), 1);
    assert!(front.quiet.is_empty());
    let PriorEpisodeFrontItem::IssueClosure {
        id,
        next,
        evaluation,
        source_refs,
    } = &front.items[0]
    else {
        panic!("expected issue-closure item")
    };
    assert_eq!(id, "claude-code:31294-to-57507:inactive-rereport");
    assert_eq!(
        *next,
        PriorEpisodeNextAction::InspectPriorFailureAndRereport
    );
    assert_eq!(evaluation.prior_state, IssueState::Closed);
    assert_eq!(evaluation.later_state, IssueState::Closed);
    assert_eq!(evaluation.closure_kind, ClosureKind::AdministrativeInactive);
    assert_eq!(evaluation.clearance, ClearanceStatus::Unknown);
    assert_eq!(
        source_refs,
        &vec![
            "github:issue/31294/comment/4230270046".to_string(),
            "github:issue/57507".to_string(),
        ]
    );
}

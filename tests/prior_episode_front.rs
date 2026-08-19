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

use applicability::{ApplicabilityStatus, EvaluationContext, PathScope, PathScopeMode};
use closure_episode::{
    CLOSURE_EPISODE_SCHEMA_VERSION, ClearanceStatus, ClosureKind, ClosureReceipt,
    DuplicateChallengeReceipt, IssueClosureEpisode, IssueSnapshot, IssueState, ReReportReceipt,
    ReReportRelation,
};
use prior_episode_front::{
    PRIOR_EPISODE_FRONT_SCHEMA_VERSION, PriorEpisodeFrontItem, PriorEpisodeFrontQuery,
    PriorEpisodeInput, PriorEpisodeNextAction, PriorEpisodeQuietReason,
    evaluate_prior_episode_front, parse_prior_episode_front_query,
};
use review_memory::{
    CurrentConcern, REVIEW_MEMORY_SCHEMA_VERSION, ReviewMemoryQuery, ReviewMemoryRecord,
    ReviewOutcome, ReviewSubject, ReviewThreadDisposition,
};

const HEAD_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEAD_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn review_record(revision: &str, work: &str) -> ReviewMemoryRecord {
    ReviewMemoryRecord {
        event_id: "github:pull/7/review-comment/100".to_string(),
        concern_key: "review:fixture:unused-result".to_string(),
        source_ref: "github:pull/7/review-comment/100".to_string(),
        subject: ReviewSubject {
            repository: "owner/repo".to_string(),
            work: work.to_string(),
            revision: revision.to_string(),
            scope: Some(PathScope {
                mode: PathScopeMode::Exact,
                path: "src/lib.rs".to_string(),
            }),
        },
        outcome: ReviewOutcome::Dismissed,
        resolution_ref: Some("github:pull/7/review-comment/101".to_string()),
    }
}

fn review_query(
    records: Vec<ReviewMemoryRecord>,
    revision: Option<&str>,
    work: Option<&str>,
    path: Option<&str>,
) -> ReviewMemoryQuery {
    ReviewMemoryQuery {
        schema_version: REVIEW_MEMORY_SCHEMA_VERSION,
        current: CurrentConcern {
            concern_key: "review:fixture:unused-result".to_string(),
            context: EvaluationContext {
                repository: Some("owner/repo".to_string()),
                revision: revision.map(str::to_string),
                work: work.map(str::to_string),
                path: path.map(str::to_string),
            },
        },
        records,
    }
}

fn issue(number: u64, state: IssueState) -> IssueSnapshot {
    IssueSnapshot {
        number,
        title: format!("issue {number}"),
        state,
        state_reason: Some("not_planned".to_string()),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        closed_at: (state == IssueState::Closed).then(|| "2026-01-02T00:00:00Z".to_string()),
        closed_by: (state == IssueState::Closed).then(|| "github-actions[bot]".to_string()),
    }
}

fn closure_episode(later_state: IssueState) -> IssueClosureEpisode {
    IssueClosureEpisode {
        schema_version: CLOSURE_EPISODE_SCHEMA_VERSION,
        repository: "owner/repo".to_string(),
        prior: issue(10, IssueState::Closed),
        later: issue(20, later_state),
        closure: ClosureReceipt {
            issue: 10,
            comment_id: 100,
            source_ref: "github:issue/10/comment/100".to_string(),
            actor: "github-actions[bot]".to_string(),
            kind: ClosureKind::AdministrativeInactive,
            evidence: "Closing for now — inactive for too long. Please [open a new issue](https://github.com/owner/repo/issues/new/choose) if this is still relevant.".to_string(),
        },
        re_report: ReReportReceipt {
            from_issue: 20,
            to_issue: 10,
            relation: ReReportRelation::ReReportOf,
            source_ref: "github:issue/20".to_string(),
            evidence: "**Re-reporting** the bug from #10 (closed earlier).".to_string(),
        },
        duplicate_challenge: Some(DuplicateChallengeReceipt {
            suggestion_comment_id: 90,
            suggestion_source_ref: "github:issue/10/comment/90".to_string(),
            suggestion_actor: "github-actions[bot]".to_string(),
            suggestion_evidence: "Found possible duplicate issues.".to_string(),
            rejection_comment_id: 91,
            rejection_source_ref: "github:issue/10/comment/91".to_string(),
            rejection_actor: "reporter".to_string(),
            rejection_evidence: "Not a duplicate of the suggested issues.".to_string(),
        }),
    }
}

fn front(inputs: Vec<PriorEpisodeInput>) -> PriorEpisodeFrontQuery {
    PriorEpisodeFrontQuery {
        schema_version: PRIOR_EPISODE_FRONT_SCHEMA_VERSION,
        inputs,
    }
}

#[test]
fn moved_head_review_surfaces_recompute_and_refresh() {
    let output = evaluate_prior_episode_front(&front(vec![PriorEpisodeInput::ReviewMemory {
        id: "review:stale".to_string(),
        query: review_query(
            vec![review_record(HEAD_A, "github:pull/7")],
            Some(HEAD_B),
            Some("github:pull/7"),
            Some("src/lib.rs"),
        ),
    }]))
    .unwrap();

    assert_eq!(output.items.len(), 1);
    assert!(output.quiet.is_empty());
    let PriorEpisodeFrontItem::Review {
        id,
        next,
        evaluation,
    } = &output.items[0]
    else {
        panic!("expected review front item")
    };
    assert_eq!(id, "review:stale");
    assert_eq!(
        *next,
        PriorEpisodeNextAction::RecomputeAndRefreshReviewThread
    );
    assert_eq!(
        evaluation.disposition,
        ReviewThreadDisposition::RefreshExistingThread
    );
    assert_eq!(
        evaluation.matches[0].applicability.status,
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn exact_head_review_surfaces_thread_reuse() {
    let output = evaluate_prior_episode_front(&front(vec![PriorEpisodeInput::ReviewMemory {
        id: "review:current".to_string(),
        query: review_query(
            vec![review_record(HEAD_A, "github:pull/7")],
            Some(HEAD_A),
            Some("github:pull/7"),
            Some("src/lib.rs"),
        ),
    }]))
    .unwrap();

    let PriorEpisodeFrontItem::Review {
        next, evaluation, ..
    } = &output.items[0]
    else {
        panic!("expected review front item")
    };
    assert_eq!(*next, PriorEpisodeNextAction::ReuseExistingReviewThread);
    assert_eq!(
        evaluation.disposition,
        ReviewThreadDisposition::ReuseCurrentThread
    );
}

#[test]
fn missing_coordinate_with_prior_lineage_surfaces_need_context() {
    let output = evaluate_prior_episode_front(&front(vec![PriorEpisodeInput::ReviewMemory {
        id: "review:missing-head".to_string(),
        query: review_query(
            vec![review_record(HEAD_A, "github:pull/7")],
            None,
            Some("github:pull/7"),
            Some("src/lib.rs"),
        ),
    }]))
    .unwrap();

    let PriorEpisodeFrontItem::Review {
        next, evaluation, ..
    } = &output.items[0]
    else {
        panic!("expected review front item")
    };
    assert_eq!(
        *next,
        PriorEpisodeNextAction::AcquireMissingReviewCoordinate
    );
    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NeedContext);
}

#[test]
fn missing_coordinate_without_prior_lineage_stays_quiet() {
    let output = evaluate_prior_episode_front(&front(vec![PriorEpisodeInput::ReviewMemory {
        id: "review:no-memory".to_string(),
        query: review_query(Vec::new(), None, Some("github:pull/7"), Some("src/lib.rs")),
    }]))
    .unwrap();

    assert!(output.items.is_empty());
    assert_eq!(output.quiet.len(), 1);
    assert_eq!(
        output.quiet[0].reason,
        PriorEpisodeQuietReason::NoPriorReviewLineage
    );
    assert_eq!(
        output.quiet[0].evaluation.disposition,
        ReviewThreadDisposition::NeedContext
    );
}

#[test]
fn unrelated_review_lineage_stays_quiet_with_receipt() {
    let output = evaluate_prior_episode_front(&front(vec![PriorEpisodeInput::ReviewMemory {
        id: "review:other-work".to_string(),
        query: review_query(
            vec![review_record(HEAD_A, "github:pull/7")],
            Some(HEAD_A),
            Some("github:pull/8"),
            Some("src/lib.rs"),
        ),
    }]))
    .unwrap();

    assert!(output.items.is_empty());
    assert_eq!(output.quiet.len(), 1);
    assert_eq!(
        output.quiet[0].reason,
        PriorEpisodeQuietReason::NoCurrentReviewLineage
    );
    assert_eq!(
        output.quiet[0].evaluation.disposition,
        ReviewThreadDisposition::NewThread
    );
    assert_eq!(output.quiet[0].evaluation.matches.len(), 1);
}

#[test]
fn explicit_closure_rereport_surfaces_inspection_with_unknown_clearance() {
    let output = evaluate_prior_episode_front(&front(vec![PriorEpisodeInput::IssueClosure {
        id: "issue:rereport".to_string(),
        episode: Box::new(closure_episode(IssueState::Open)),
    }]))
    .unwrap();

    assert_eq!(output.items.len(), 1);
    let PriorEpisodeFrontItem::IssueClosure {
        next,
        evaluation,
        source_refs,
        ..
    } = &output.items[0]
    else {
        panic!("expected issue-closure front item")
    };
    assert_eq!(
        *next,
        PriorEpisodeNextAction::InspectPriorFailureAndRereport
    );
    assert_eq!(evaluation.clearance, ClearanceStatus::Unknown);
    assert_eq!(
        source_refs,
        &vec![
            "github:issue/10/comment/100".to_string(),
            "github:issue/20".to_string(),
            "github:issue/10/comment/90".to_string(),
            "github:issue/10/comment/91".to_string(),
        ]
    );
}

#[test]
fn closing_later_rereport_does_not_remove_front_item() {
    let output = evaluate_prior_episode_front(&front(vec![PriorEpisodeInput::IssueClosure {
        id: "issue:closed-rereport".to_string(),
        episode: Box::new(closure_episode(IssueState::Closed)),
    }]))
    .unwrap();

    let PriorEpisodeFrontItem::IssueClosure { evaluation, .. } = &output.items[0] else {
        panic!("expected issue-closure front item")
    };
    assert_eq!(evaluation.later_state, IssueState::Closed);
    assert_eq!(evaluation.clearance, ClearanceStatus::Unknown);
}

#[test]
fn empty_input_produces_empty_front_and_quiet_set() {
    let output = evaluate_prior_episode_front(&front(Vec::new())).unwrap();
    assert!(output.items.is_empty());
    assert!(output.quiet.is_empty());
}

#[test]
fn surfaced_items_preserve_admitted_input_order_across_species() {
    let output = evaluate_prior_episode_front(&front(vec![
        PriorEpisodeInput::ReviewMemory {
            id: "review:first".to_string(),
            query: review_query(
                vec![review_record(HEAD_A, "github:pull/7")],
                Some(HEAD_B),
                Some("github:pull/7"),
                Some("src/lib.rs"),
            ),
        },
        PriorEpisodeInput::ReviewMemory {
            id: "review:quiet".to_string(),
            query: review_query(
                Vec::new(),
                Some(HEAD_A),
                Some("github:pull/7"),
                Some("src/lib.rs"),
            ),
        },
        PriorEpisodeInput::IssueClosure {
            id: "issue:third".to_string(),
            episode: Box::new(closure_episode(IssueState::Open)),
        },
    ]))
    .unwrap();

    assert_eq!(output.items.len(), 2);
    assert_eq!(output.quiet.len(), 1);
    assert!(matches!(
        &output.items[0],
        PriorEpisodeFrontItem::Review { id, .. } if id == "review:first"
    ));
    assert!(matches!(
        &output.items[1],
        PriorEpisodeFrontItem::IssueClosure { id, .. } if id == "issue:third"
    ));
    assert_eq!(output.quiet[0].id, "review:quiet");
}

#[test]
fn duplicate_packet_local_ids_reject_before_source_projection() {
    let input = PriorEpisodeInput::ReviewMemory {
        id: "duplicate".to_string(),
        query: review_query(
            Vec::new(),
            Some(HEAD_A),
            Some("github:pull/7"),
            Some("src/lib.rs"),
        ),
    };
    let error = evaluate_prior_episode_front(&front(vec![input.clone(), input])).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("duplicate prior-episode input id")
    );
}

#[test]
fn source_evaluator_error_names_packet_local_id() {
    let output = evaluate_prior_episode_front(&front(vec![PriorEpisodeInput::ReviewMemory {
        id: "bad-review".to_string(),
        query: review_query(
            vec![review_record(
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                "github:pull/7",
            )],
            Some(HEAD_B),
            Some("github:pull/7"),
            Some("src/lib.rs"),
        ),
    }]));
    let error = output.unwrap_err();
    assert!(error.to_string().contains("`bad-review`"));
    assert!(error.to_string().contains("review-memory evidence"));
}

#[test]
fn parser_rejects_unknown_machine_fields() {
    let mut value = serde_json::to_value(front(Vec::new())).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("score".to_string(), serde_json::json!(1.0));
    let bytes = serde_json::to_vec(&value).unwrap();
    let error = parse_prior_episode_front_query(&bytes).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

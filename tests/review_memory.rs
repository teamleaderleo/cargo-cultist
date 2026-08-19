#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/review_memory.rs"]
mod review_memory;

use applicability::{ApplicabilityStatus, EvaluationContext, PathScope, PathScopeMode};
use review_memory::{
    CurrentConcern, REVIEW_MEMORY_SCHEMA_VERSION, ReviewMemoryMatchKind, ReviewMemoryQuery,
    ReviewMemoryRecord, ReviewOutcome, ReviewSubject, ReviewThreadDisposition,
    evaluate_review_memory, parse_review_memory_query,
};

const HEAD_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEAD_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEAD_C: &str = "cccccccccccccccccccccccccccccccccccccccc";

fn record(
    event_id: &str,
    revision: &str,
    work: &str,
    scope: Option<PathScope>,
    outcome: ReviewOutcome,
    resolution_ref: Option<&str>,
) -> ReviewMemoryRecord {
    ReviewMemoryRecord {
        event_id: event_id.to_string(),
        concern_key: "review:unused-result:src/lib.rs".to_string(),
        source_ref: "github:review-comment/100".to_string(),
        subject: ReviewSubject {
            repository: "owner/repo".to_string(),
            work: work.to_string(),
            revision: revision.to_string(),
            scope,
        },
        outcome,
        resolution_ref: resolution_ref.map(str::to_string),
    }
}

fn query(
    records: Vec<ReviewMemoryRecord>,
    revision: Option<&str>,
    work: Option<&str>,
    path: Option<&str>,
) -> ReviewMemoryQuery {
    ReviewMemoryQuery {
        schema_version: REVIEW_MEMORY_SCHEMA_VERSION,
        current: CurrentConcern {
            concern_key: "review:unused-result:src/lib.rs".to_string(),
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

#[test]
fn same_exact_head_reuses_current_thread() {
    let evaluation = evaluate_review_memory(&query(
        vec![record(
            "event:1",
            HEAD_A,
            "#7",
            None,
            ReviewOutcome::Open,
            None,
        )],
        Some(HEAD_A),
        Some("#7"),
        None,
    ))
    .unwrap();

    assert_eq!(
        evaluation.disposition,
        ReviewThreadDisposition::ReuseCurrentThread
    );
    assert_eq!(
        evaluation.matches[0].match_kind,
        ReviewMemoryMatchKind::Current
    );
    assert_eq!(
        evaluation.matches[0].applicability.status,
        ApplicabilityStatus::Applies
    );
}

#[test]
fn moved_head_refreshes_thread_and_invalidates_old_resolution() {
    let evaluation = evaluate_review_memory(&query(
        vec![record(
            "event:1",
            HEAD_A,
            "#7",
            None,
            ReviewOutcome::RejectedWithEvidence,
            Some("github:review-comment/101"),
        )],
        Some(HEAD_B),
        Some("#7"),
        None,
    ))
    .unwrap();

    assert_eq!(
        evaluation.disposition,
        ReviewThreadDisposition::RefreshExistingThread
    );
    assert_eq!(
        evaluation.matches[0].match_kind,
        ReviewMemoryMatchKind::PriorHead
    );
    assert_eq!(
        evaluation.matches[0].applicability.status,
        ApplicabilityStatus::Invalid
    );
    assert_eq!(
        evaluation.matches[0].outcome,
        ReviewOutcome::RejectedWithEvidence
    );
}

#[test]
fn missing_current_head_requires_context() {
    let evaluation = evaluate_review_memory(&query(
        vec![record(
            "event:1",
            HEAD_A,
            "#7",
            None,
            ReviewOutcome::Open,
            None,
        )],
        None,
        Some("#7"),
        None,
    ))
    .unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NeedContext);
    assert_eq!(
        evaluation.matches[0].match_kind,
        ReviewMemoryMatchKind::ContextMissing
    );
    assert_eq!(
        evaluation.matches[0].applicability.status,
        ApplicabilityStatus::Unknown
    );
}

#[test]
fn same_concern_key_on_different_work_starts_new_thread() {
    let evaluation = evaluate_review_memory(&query(
        vec![record(
            "event:1",
            HEAD_A,
            "#7",
            None,
            ReviewOutcome::Open,
            None,
        )],
        Some(HEAD_A),
        Some("#8"),
        None,
    ))
    .unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NewThread);
    assert_eq!(
        evaluation.matches[0].match_kind,
        ReviewMemoryMatchKind::Unrelated
    );
}

#[test]
fn same_concern_key_on_different_scope_starts_new_thread() {
    let scope = PathScope {
        mode: PathScopeMode::Exact,
        path: "src/lib.rs".to_string(),
    };
    let evaluation = evaluate_review_memory(&query(
        vec![record(
            "event:1",
            HEAD_A,
            "#7",
            Some(scope),
            ReviewOutcome::Open,
            None,
        )],
        Some(HEAD_A),
        Some("#7"),
        Some("src/other.rs"),
    ))
    .unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NewThread);
    assert_eq!(
        evaluation.matches[0].match_kind,
        ReviewMemoryMatchKind::Unrelated
    );
}

#[test]
fn missing_scope_context_does_not_refresh_from_head_mismatch_alone() {
    let scope = PathScope {
        mode: PathScopeMode::Exact,
        path: "src/lib.rs".to_string(),
    };
    let evaluation = evaluate_review_memory(&query(
        vec![record(
            "event:1",
            HEAD_A,
            "#7",
            Some(scope),
            ReviewOutcome::Dismissed,
            Some("github:review-comment/101"),
        )],
        Some(HEAD_B),
        Some("#7"),
        None,
    ))
    .unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NeedContext);
    assert_eq!(
        evaluation.matches[0].match_kind,
        ReviewMemoryMatchKind::ContextMissing
    );
}

#[test]
fn multiple_prior_events_are_retained_without_latest_inference() {
    let evaluation = evaluate_review_memory(&query(
        vec![
            record(
                "event:b",
                HEAD_B,
                "#7",
                None,
                ReviewOutcome::Dismissed,
                Some("github:review-comment/102"),
            ),
            record(
                "event:a",
                HEAD_A,
                "#7",
                None,
                ReviewOutcome::PatchChanged,
                Some("github:commit/change"),
            ),
        ],
        Some(HEAD_C),
        Some("#7"),
        None,
    ))
    .unwrap();

    assert_eq!(
        evaluation.disposition,
        ReviewThreadDisposition::RefreshExistingThread
    );
    assert_eq!(
        evaluation
            .matches
            .iter()
            .map(|entry| entry.event_id.as_str())
            .collect::<Vec<_>>(),
        vec!["event:a", "event:b"]
    );
    assert!(
        evaluation
            .matches
            .iter()
            .all(|entry| entry.match_kind == ReviewMemoryMatchKind::PriorHead)
    );
}

#[test]
fn records_for_other_concerns_do_not_create_thread_identity() {
    let mut other = record("event:other", HEAD_A, "#7", None, ReviewOutcome::Open, None);
    other.concern_key = "review:different-concern".to_string();

    let evaluation =
        evaluate_review_memory(&query(vec![other], Some(HEAD_A), Some("#7"), None)).unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NewThread);
    assert!(evaluation.matches.is_empty());
}

#[test]
fn empty_memory_starts_new_thread() {
    let evaluation =
        evaluate_review_memory(&query(Vec::new(), Some(HEAD_A), Some("#7"), None)).unwrap();

    assert_eq!(evaluation.disposition, ReviewThreadDisposition::NewThread);
    assert!(evaluation.matches.is_empty());
}

#[test]
fn duplicate_and_conflicting_event_ids_reject() {
    let first = record("event:1", HEAD_A, "#7", None, ReviewOutcome::Open, None);
    let duplicate = first.clone();
    let mut conflict = first.clone();
    conflict.source_ref = "github:review-comment/999".to_string();

    let duplicate_error = evaluate_review_memory(&query(
        vec![first.clone(), duplicate],
        Some(HEAD_A),
        Some("#7"),
        None,
    ))
    .unwrap_err();
    assert!(
        duplicate_error
            .to_string()
            .contains("duplicate review event_id")
    );

    let conflict_error = evaluate_review_memory(&query(
        vec![first, conflict],
        Some(HEAD_A),
        Some("#7"),
        None,
    ))
    .unwrap_err();
    assert!(
        conflict_error
            .to_string()
            .contains("conflicting duplicate review event_id")
    );
}

#[test]
fn reviewed_revision_must_be_exact_lowercase_sha() {
    let bad = record(
        "event:1",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "#7",
        None,
        ReviewOutcome::Open,
        None,
    );
    let error =
        evaluate_review_memory(&query(vec![bad], Some(HEAD_A), Some("#7"), None)).unwrap_err();
    assert!(error.to_string().contains("exact 40-character lowercase"));
}

#[test]
fn malformed_current_revision_rejects_even_with_empty_memory() {
    let error = evaluate_review_memory(&query(Vec::new(), Some("head-main"), Some("#7"), None))
        .unwrap_err();
    assert!(error.to_string().contains("current.context.revision"));
}

#[test]
fn resolution_reference_matches_outcome_state() {
    let open_with_resolution = record(
        "event:1",
        HEAD_A,
        "#7",
        None,
        ReviewOutcome::Open,
        Some("github:review-comment/101"),
    );
    let resolved_without_reference = record(
        "event:2",
        HEAD_A,
        "#7",
        None,
        ReviewOutcome::Dismissed,
        None,
    );

    let open_error = evaluate_review_memory(&query(
        vec![open_with_resolution],
        Some(HEAD_A),
        Some("#7"),
        None,
    ))
    .unwrap_err();
    assert!(
        open_error
            .to_string()
            .contains("must not carry resolution_ref")
    );

    let resolved_error = evaluate_review_memory(&query(
        vec![resolved_without_reference],
        Some(HEAD_A),
        Some("#7"),
        None,
    ))
    .unwrap_err();
    assert!(
        resolved_error
            .to_string()
            .contains("requires resolution_ref")
    );
}

#[test]
fn pr_agent_2184_retained_fixture_refreshes_without_inheriting_resolution() {
    let query = parse_review_memory_query(include_bytes!(
        "../research/review-memory/pr-agent-2184.json"
    ))
    .unwrap();
    let evaluation = evaluate_review_memory(&query).unwrap();

    assert_eq!(
        evaluation.disposition,
        ReviewThreadDisposition::RefreshExistingThread
    );
    assert_eq!(evaluation.matches.len(), 1);
    assert_eq!(
        evaluation.matches[0].match_kind,
        ReviewMemoryMatchKind::PriorHead
    );
    assert_eq!(
        evaluation.matches[0].applicability.status,
        ApplicabilityStatus::Invalid
    );
    assert_eq!(evaluation.matches[0].source_ref, "github:issue/2184");
    assert_eq!(evaluation.matches[0].outcome, ReviewOutcome::Dismissed);
}

#![allow(dead_code)]

#[path = "../src/closure_episode.rs"]
mod closure_episode;

use closure_episode::{
    CLOSURE_EPISODE_SCHEMA_VERSION, ClearanceStatus, ClosureEpisodeDisposition, ClosureKind,
    ClosureReceipt, DuplicateChallengeReceipt, IssueClosureEpisode, IssueSnapshot, IssueState,
    ReReportReceipt, ReReportRelation, evaluate_closure_episode, parse_closure_episode,
};

const REPOSITORY: &str = "owner/repo";
const ADMIN_CLOSURE: &str = "Closing for now — inactive for too long. Please [open a new issue](https://github.com/owner/repo/issues/new/choose) if this is still relevant.";

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

fn episode() -> IssueClosureEpisode {
    IssueClosureEpisode {
        schema_version: CLOSURE_EPISODE_SCHEMA_VERSION,
        repository: REPOSITORY.to_string(),
        prior: issue(10, IssueState::Closed),
        later: issue(20, IssueState::Open),
        closure: ClosureReceipt {
            issue: 10,
            comment_id: 100,
            source_ref: "github:issue/10/comment/100".to_string(),
            actor: "github-actions[bot]".to_string(),
            kind: ClosureKind::AdministrativeInactive,
            evidence: ADMIN_CLOSURE.to_string(),
        },
        re_report: ReReportReceipt {
            from_issue: 20,
            to_issue: 10,
            relation: ReReportRelation::ReReportOf,
            source_ref: "github:issue/20".to_string(),
            evidence: "**Re-reporting** the bug from #10 (closed earlier).".to_string(),
        },
        duplicate_challenge: None,
    }
}

#[test]
fn administrative_closure_plus_rereport_keeps_clearance_unknown() {
    let evaluation = evaluate_closure_episode(&episode()).unwrap();

    assert_eq!(evaluation.prior_state, IssueState::Closed);
    assert_eq!(evaluation.later_state, IssueState::Open);
    assert_eq!(evaluation.closure_kind, ClosureKind::AdministrativeInactive);
    assert!(evaluation.re_report_observed);
    assert_eq!(evaluation.clearance, ClearanceStatus::Unknown);
    assert_eq!(
        evaluation.disposition,
        ClosureEpisodeDisposition::InspectPriorFailure
    );
}

#[test]
fn closing_the_later_rereport_does_not_clear_the_prior_failure() {
    let mut input = episode();
    input.later = issue(20, IssueState::Closed);

    let evaluation = evaluate_closure_episode(&input).unwrap();

    assert_eq!(evaluation.later_state, IssueState::Closed);
    assert_eq!(evaluation.clearance, ClearanceStatus::Unknown);
    assert_eq!(
        evaluation.disposition,
        ClosureEpisodeDisposition::InspectPriorFailure
    );
}

#[test]
fn not_planned_state_reason_alone_does_not_create_inactivity_semantics() {
    let mut input = episode();
    input.closure.kind = ClosureKind::Other;
    input.closure.actor = "maintainer".to_string();
    input.closure.evidence = "Closed without a retained repair receipt.".to_string();

    let evaluation = evaluate_closure_episode(&input).unwrap();

    assert_eq!(evaluation.closure_kind, ClosureKind::Other);
    assert_eq!(evaluation.clearance, ClearanceStatus::Unknown);
}

#[test]
fn administrative_inactive_requires_exact_bot_actor_and_evidence() {
    let mut wrong_actor = episode();
    wrong_actor.closure.actor = "human".to_string();
    let error = evaluate_closure_episode(&wrong_actor).unwrap_err();
    assert!(error.to_string().contains("github-actions[bot]"));

    let mut wrong_evidence = episode();
    wrong_evidence.closure.evidence = "Closing because inactive.".to_string();
    let error = evaluate_closure_episode(&wrong_evidence).unwrap_err();
    assert!(error.to_string().contains("admitted exact GitHub bot form"));
}

#[test]
fn rereport_evidence_must_name_the_declared_prior_issue() {
    let mut input = episode();
    input.re_report.evidence = "**Re-reporting** the bug from #11 (closed earlier).".to_string();
    let error = evaluate_closure_episode(&input).unwrap_err();
    assert!(error.to_string().contains("not prior issue #10"));

    let mut arbitrary = episode();
    arbitrary.re_report.evidence = "Related to #10".to_string();
    let error = evaluate_closure_episode(&arbitrary).unwrap_err();
    assert!(error.to_string().contains("not an admitted exact re-report form"));
}

#[test]
fn prior_issue_must_actually_be_closed() {
    let mut input = episode();
    input.prior = issue(10, IssueState::Open);
    let error = evaluate_closure_episode(&input).unwrap_err();
    assert!(error.to_string().contains("prior issue to be closed"));
}

#[test]
fn duplicate_challenge_is_source_evidence_without_changing_clearance() {
    let mut input = episode();
    input.duplicate_challenge = Some(DuplicateChallengeReceipt {
        suggestion_comment_id: 90,
        suggestion_source_ref: "github:issue/10/comment/90".to_string(),
        suggestion_actor: "github-actions[bot]".to_string(),
        suggestion_evidence: "Found 3 possible duplicate issues.".to_string(),
        rejection_comment_id: 91,
        rejection_source_ref: "github:issue/10/comment/91".to_string(),
        rejection_actor: "reporter".to_string(),
        rejection_evidence: "Not a duplicate of the suggested issues.".to_string(),
    });

    let evaluation = evaluate_closure_episode(&input).unwrap();
    assert_eq!(evaluation.clearance, ClearanceStatus::Unknown);
}

#[test]
fn parser_rejects_unknown_machine_fields() {
    let mut value = serde_json::to_value(episode()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("confidence".to_string(), serde_json::json!(0.99));
    let bytes = serde_json::to_vec(&value).unwrap();
    let error = parse_closure_episode(&bytes).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

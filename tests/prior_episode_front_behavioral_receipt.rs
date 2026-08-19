#[allow(dead_code)]
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;

use behavioral_receipt::{BehavioralOutcome, BehavioralReceipt, Delivery, validate_receipt};

#[test]
fn retained_prior_episode_front_receipt_records_observed_plan_change() {
    let receipt: BehavioralReceipt = serde_json::from_slice(include_bytes!(
        "../research/behavioral-receipts/prior-episode-front-237.json"
    ))
    .unwrap();
    validate_receipt(&receipt).unwrap();

    assert_eq!(receipt.repository, "teamleaderleo/cultist");
    assert_eq!(receipt.revision, "b3e80cfa7e0e238bb6f4aae9fd241d9d3ea4fef9");
    assert_eq!(
        receipt.task,
        "issue-41-temporal-precedent-behavioral-followup"
    );
    assert_eq!(receipt.evidence_kind, "prior-episode-front-composition");
    assert_eq!(receipt.evidence_ref, "github:pull/237");
    assert_eq!(receipt.delivery, Delivery::Surfaced);
    assert!(receipt.consulted);
    assert_eq!(receipt.outcome, BehavioralOutcome::ChangedNextAction);
    assert_eq!(
        receipt.action.as_deref(),
        Some(
            "stop adding isolated temporal episode species and move to behavioral-effect evaluation of selected prior-episode actions"
        )
    );
}

#[test]
fn action_changing_receipt_still_requires_concrete_action() {
    let mut receipt: BehavioralReceipt = serde_json::from_slice(include_bytes!(
        "../research/behavioral-receipts/prior-episode-front-237.json"
    ))
    .unwrap();
    receipt.action = None;

    let error = validate_receipt(&receipt).unwrap_err();
    assert!(error.to_string().contains("changed_next_action"));
    assert!(error.to_string().contains("concrete action"));
}

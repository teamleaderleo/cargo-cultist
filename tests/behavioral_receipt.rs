#[allow(dead_code)]
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;

use behavioral_receipt::{
    BEHAVIORAL_RECEIPT_SCHEMA_VERSION, BehavioralOutcome, BehavioralReceipt, Delivery,
    validate_receipt,
};

fn receipt(outcome: BehavioralOutcome) -> BehavioralReceipt {
    BehavioralReceipt {
        schema_version: BEHAVIORAL_RECEIPT_SCHEMA_VERSION,
        repository: "teamleaderleo/cultist".to_string(),
        revision: "4f8f9fcd8ef1dab1881d07d063a034d9d5d7f136".to_string(),
        task: "issue-137-behavioral-pressure".to_string(),
        evidence_kind: "generated-companion-missing".to_string(),
        evidence_ref: "report-fingerprint/F1".to_string(),
        delivery: Delivery::Surfaced,
        consulted: true,
        outcome,
        action: None,
    }
}

#[test]
fn accepts_surfaced_action_change_with_concrete_action() {
    let mut value = receipt(BehavioralOutcome::ChangedNextAction);
    value.action = Some("run cargo generator before continuing".to_string());
    validate_receipt(&value).unwrap();
}

#[test]
fn accepts_prevented_wrong_turn_with_concrete_action() {
    let mut value = receipt(BehavioralOutcome::PreventedOrReversedWrongTurn);
    value.action = Some("drop the stale implementation approach".to_string());
    validate_receipt(&value).unwrap();
}

#[test]
fn accepts_correct_quiet_negative() {
    let mut value = receipt(BehavioralOutcome::CorrectQuietNegative);
    value.delivery = Delivery::Quiet;
    value.consulted = false;
    validate_receipt(&value).unwrap();
}

#[test]
fn accepts_ignored_surfaced_evidence() {
    let mut value = receipt(BehavioralOutcome::Ignored);
    value.consulted = false;
    validate_receipt(&value).unwrap();
}

#[test]
fn rejects_action_change_without_concrete_action() {
    let value = receipt(BehavioralOutcome::ChangedNextAction);
    let error = validate_receipt(&value).unwrap_err();
    assert!(error.to_string().contains("requires a concrete action"));
}

#[test]
fn rejects_unconsulted_outcome_that_claims_interpretation() {
    let mut value = receipt(BehavioralOutcome::Irrelevant);
    value.consulted = false;
    let error = validate_receipt(&value).unwrap_err();
    assert!(error.to_string().contains("requires consulted=true"));
}

#[test]
fn rejects_action_on_useful_same_action_receipt() {
    let mut value = receipt(BehavioralOutcome::UsefulSameAction);
    value.action = Some("keep doing the same thing".to_string());
    let error = validate_receipt(&value).unwrap_err();
    assert!(error.to_string().contains("must omit action"));
}

#[test]
fn rejects_quiet_receipt_with_nonquiet_outcome() {
    let mut value = receipt(BehavioralOutcome::Irrelevant);
    value.delivery = Delivery::Quiet;
    value.consulted = false;
    let error = validate_receipt(&value).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("requires outcome=correct_quiet_negative")
    );
}

#[test]
fn rejects_surfaced_receipt_claiming_quiet_negative() {
    let value = receipt(BehavioralOutcome::CorrectQuietNegative);
    let error = validate_receipt(&value).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("cannot use outcome=correct_quiet_negative")
    );
}

#[test]
fn rejects_nonexact_revision_coordinate() {
    let mut value = receipt(BehavioralOutcome::UsefulSameAction);
    value.revision = "main".to_string();
    let error = validate_receipt(&value).unwrap_err();
    assert!(error.to_string().contains("exact 40-character"));
}

#[test]
fn rejects_unknown_machine_fields() {
    let json = serde_json::json!({
        "schema_version": BEHAVIORAL_RECEIPT_SCHEMA_VERSION,
        "repository": "teamleaderleo/cultist",
        "revision": "4f8f9fcd8ef1dab1881d07d063a034d9d5d7f136",
        "task": "issue-137-behavioral-pressure",
        "evidence_kind": "preflight-explicit-coordination",
        "evidence_ref": "report-fingerprint/F1",
        "delivery": "surfaced",
        "consulted": true,
        "outcome": "useful_same_action",
        "future_semantics": true
    });

    let error = serde_json::from_value::<BehavioralReceipt>(json).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn exact_receipt_round_trips() {
    let mut value = receipt(BehavioralOutcome::NeededStrongerEvidence);
    value.action = Some("retrieve exact-head project memory".to_string());
    let json = serde_json::to_string(&value).unwrap();
    let decoded: BehavioralReceipt = serde_json::from_str(&json).unwrap();
    validate_receipt(&decoded).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn retained_collaboration_receipt_is_valid_action_change_evidence() {
    let fixture = include_str!("../research/behavioral-receipts/collaboration-140.json");
    let receipt: BehavioralReceipt = serde_json::from_str(fixture).unwrap();
    validate_receipt(&receipt).unwrap();
    assert_eq!(receipt.delivery, Delivery::Surfaced);
    assert!(receipt.consulted);
    assert_eq!(receipt.outcome, BehavioralOutcome::ChangedNextAction);
    assert!(
        receipt
            .action
            .as_deref()
            .is_some_and(|action| action.contains("rebuild the product lane"))
    );
}

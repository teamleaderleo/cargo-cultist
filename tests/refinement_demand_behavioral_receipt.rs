#[allow(dead_code)]
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;

use behavioral_receipt::{BehavioralOutcome, BehavioralReceipt, Delivery, validate_receipt};

const RECEIPT: &str =
    include_str!("../research/behavioral-receipts/refinement-demand-planning-255.json");

#[test]
fn demand_gated_planning_episode_is_a_valid_changed_next_action_receipt() {
    let receipt: BehavioralReceipt = serde_json::from_str(RECEIPT).unwrap();
    validate_receipt(&receipt).unwrap();

    assert_eq!(receipt.repository, "teamleaderleo/cultist");
    assert_eq!(receipt.revision, "1065086c45ad167037d290b83d77d52975d1f1a9");
    assert_eq!(
        receipt.task,
        "pull-255-demand-gated-refinement-probe-planning"
    );
    assert_eq!(
        receipt.evidence_kind,
        "refinement-investigation-demand-gate"
    );
    assert_eq!(
        receipt.evidence_ref,
        "github:pull/255@1065086c45ad167037d290b83d77d52975d1f1a9"
    );
    assert_eq!(receipt.delivery, Delivery::Surfaced);
    assert!(receipt.consulted);
    assert_eq!(receipt.outcome, BehavioralOutcome::ChangedNextAction);
    assert_eq!(
        receipt.action.as_deref(),
        Some(
            "plan the rust-edit-class probe only for syntax-changing-current-cohort and skip evidence acquisition for reverse-edit-class-control and singleton-commit-partition"
        )
    );
}

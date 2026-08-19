#[allow(dead_code)]
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;

use behavioral_receipt::{BehavioralOutcome, BehavioralReceipt, Delivery, validate_receipt};

#[test]
fn retained_active_work_quiet_control_is_valid() {
    let fixture = include_str!("../research/behavioral-receipts/active-work-140-quiet.json");
    let receipt: BehavioralReceipt = serde_json::from_str(fixture).unwrap();
    validate_receipt(&receipt).unwrap();
    assert_eq!(receipt.delivery, Delivery::Quiet);
    assert!(!receipt.consulted);
    assert_eq!(receipt.outcome, BehavioralOutcome::CorrectQuietNegative);
    assert!(receipt.action.is_none());
}

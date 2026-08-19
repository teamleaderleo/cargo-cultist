#[allow(dead_code)]
#[path = "../src/behavioral_episode.rs"]
mod behavioral_episode;
#[allow(dead_code)]
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;

use behavioral_episode::parse_behavioral_episode_batch;
use behavioral_receipt::{BehavioralOutcome, BehavioralReceipt, Delivery};

const CORPUS: &[u8] =
    include_bytes!("../research/behavioral-episodes/cultist-collaboration-v1.json");
const PROJECT_MEMORY_RECEIPT: &str =
    include_str!("../research/behavioral-receipts/project-memory-relation-166.json");

#[test]
fn retained_cultist_collaboration_batch_is_valid_and_unique() {
    let batch = parse_behavioral_episode_batch(CORPUS).unwrap();
    assert_eq!(batch.episodes.len(), 3);

    assert_eq!(batch.episodes[0].receipt.delivery, Delivery::Surfaced);
    assert_eq!(
        batch.episodes[0].receipt.outcome,
        BehavioralOutcome::ChangedNextAction
    );

    assert_eq!(batch.episodes[1].receipt.delivery, Delivery::Quiet);
    assert_eq!(
        batch.episodes[1].receipt.outcome,
        BehavioralOutcome::CorrectQuietNegative
    );

    assert_eq!(
        batch.episodes[2].receipt.evidence_kind,
        "project-memory-relation-strengthening"
    );
    assert_eq!(
        batch.episodes[2].receipt.outcome,
        BehavioralOutcome::ChangedNextAction
    );
}

#[test]
fn standalone_project_memory_receipt_matches_retained_episode() {
    let batch = parse_behavioral_episode_batch(CORPUS).unwrap();
    let standalone: BehavioralReceipt = serde_json::from_str(PROJECT_MEMORY_RECEIPT).unwrap();
    assert_eq!(batch.episodes[2].receipt, standalone);
}

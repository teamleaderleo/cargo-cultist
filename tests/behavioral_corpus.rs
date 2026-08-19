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
const SEMANTIC_COLLISION_RECEIPT: &str =
    include_str!("../research/behavioral-receipts/project-memory-primary-case-collision-174.json");
const KNOWN_STALE_RECEIPT: &str =
    include_str!("../research/behavioral-receipts/known-stale-observation-210.json");
const DEMAND_GATED_RECEIPT: &str =
    include_str!("../research/behavioral-receipts/refinement-demand-planning-255.json");

#[test]
fn retained_cultist_collaboration_batch_is_valid_and_unique() {
    let batch = parse_behavioral_episode_batch(CORPUS).unwrap();
    assert_eq!(batch.episodes.len(), 6);

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

    assert_eq!(
        batch.episodes[3].receipt.evidence_kind,
        "project-memory-contract-collision"
    );
    assert_eq!(
        batch.episodes[3].receipt.outcome,
        BehavioralOutcome::ChangedNextAction
    );

    assert_eq!(
        batch.episodes[4].receipt.evidence_kind,
        "known-stale-observation-counterexample"
    );
    assert_eq!(
        batch.episodes[4].receipt.outcome,
        BehavioralOutcome::ChangedNextAction
    );

    assert_eq!(
        batch.episodes[5].receipt.evidence_kind,
        "refinement-investigation-demand-gate"
    );
    assert_eq!(
        batch.episodes[5].receipt.outcome,
        BehavioralOutcome::ChangedNextAction
    );
}

#[test]
fn standalone_decision_receipts_match_retained_episodes() {
    let batch = parse_behavioral_episode_batch(CORPUS).unwrap();
    let relation: BehavioralReceipt = serde_json::from_str(PROJECT_MEMORY_RECEIPT).unwrap();
    let collision: BehavioralReceipt = serde_json::from_str(SEMANTIC_COLLISION_RECEIPT).unwrap();
    let known_stale: BehavioralReceipt = serde_json::from_str(KNOWN_STALE_RECEIPT).unwrap();
    let demand_gated: BehavioralReceipt = serde_json::from_str(DEMAND_GATED_RECEIPT).unwrap();

    assert_eq!(batch.episodes[2].receipt, relation);
    assert_eq!(batch.episodes[3].receipt, collision);
    assert_eq!(batch.episodes[4].receipt, known_stale);
    assert_eq!(batch.episodes[5].receipt, demand_gated);
}

#[allow(dead_code)]
#[path = "../src/behavioral_episode.rs"]
mod behavioral_episode;
#[allow(dead_code)]
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;

use behavioral_episode::{
    BEHAVIORAL_EPISODE_SCHEMA_VERSION, BehavioralEpisode, BehavioralEpisodeBatch,
    MAX_BEHAVIORAL_EPISODE_BATCH_BYTES, parse_behavioral_episode_batch,
    validate_behavioral_episode_batch,
};
use behavioral_receipt::{BehavioralOutcome, BehavioralReceipt};

fn retained_receipt(path: &str) -> BehavioralReceipt {
    let json = match path {
        "collaboration" => {
            include_str!("../research/behavioral-receipts/collaboration-140.json")
        }
        "quiet" => include_str!("../research/behavioral-receipts/active-work-140-quiet.json"),
        other => panic!("unknown retained receipt {other}"),
    };
    serde_json::from_str(json).unwrap()
}

fn retained_batch() -> BehavioralEpisodeBatch {
    BehavioralEpisodeBatch {
        schema_version: BEHAVIORAL_EPISODE_SCHEMA_VERSION,
        episodes: vec![
            BehavioralEpisode {
                episode_id: "pull-140:main-head-movement:4f8f9fcd->85e0b08b".to_string(),
                receipt: retained_receipt("collaboration"),
            },
            BehavioralEpisode {
                episode_id: "github-actions:run/32242752523#active-work-heads-up".to_string(),
                receipt: retained_receipt("quiet"),
            },
        ],
    }
}

#[test]
fn accepts_two_distinct_retained_collaboration_episodes() {
    let batch = retained_batch();
    validate_behavioral_episode_batch(&batch).unwrap();
    assert_eq!(batch.episodes.len(), 2);
}

#[test]
fn exact_duplicate_episode_id_rejects_instead_of_double_counting() {
    let mut batch = retained_batch();
    batch.episodes.push(batch.episodes[1].clone());
    let error = validate_behavioral_episode_batch(&batch).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("duplicate behavioral episode_id")
    );
}

#[test]
fn conflicting_same_episode_id_rejects_hard() {
    let mut batch = retained_batch();
    let mut conflicting = batch.episodes[0].clone();
    conflicting.receipt.outcome = BehavioralOutcome::NeededStrongerEvidence;
    conflicting.receipt.action = Some("inspect another exact evidence source".to_string());
    batch.episodes.push(conflicting);

    let error = validate_behavioral_episode_batch(&batch).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("conflicting duplicate behavioral episode_id")
    );
}

#[test]
fn repeated_delivery_of_same_receipt_is_distinct_with_new_episode_id() {
    let mut batch = retained_batch();
    batch.episodes.push(BehavioralEpisode {
        episode_id: "heldout:repeat-delivery-02".to_string(),
        receipt: batch.episodes[1].receipt.clone(),
    });
    validate_behavioral_episode_batch(&batch).unwrap();
    assert_eq!(batch.episodes.len(), 3);
}

#[test]
fn rejects_noncanonical_episode_identity() {
    let mut batch = retained_batch();
    batch.episodes[0].episode_id = " bad\nepisode ".to_string();
    let error = validate_behavioral_episode_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("canonical observation identity"));
}

#[test]
fn batch_round_trips_without_changing_episode_identity() {
    let batch = retained_batch();
    let json = serde_json::to_vec(&batch).unwrap();
    let decoded = parse_behavioral_episode_batch(&json).unwrap();
    assert_eq!(decoded, batch);
}

#[test]
fn oversized_batch_rejects_before_json_parsing() {
    let bytes = vec![b' '; MAX_BEHAVIORAL_EPISODE_BATCH_BYTES + 1];
    let error = parse_behavioral_episode_batch(&bytes).unwrap_err();
    assert!(error.to_string().contains("exceeds"));
}

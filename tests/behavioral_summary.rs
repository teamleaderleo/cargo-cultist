#[allow(dead_code)]
#[path = "../src/behavioral_episode.rs"]
mod behavioral_episode;
#[allow(dead_code)]
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;
#[allow(dead_code)]
#[path = "../src/behavioral_summary.rs"]
mod behavioral_summary;

use behavioral_episode::parse_behavioral_episode_batch;
use behavioral_summary::{BehavioralSummary, summarize_behavioral_episodes};

const CORPUS: &[u8] =
    include_bytes!("../research/behavioral-episodes/cultist-collaboration-v1.json");

fn retained_summary() -> BehavioralSummary {
    let batch = parse_behavioral_episode_batch(CORPUS).unwrap();
    summarize_behavioral_episodes(&batch).unwrap()
}

#[test]
fn retained_corpus_has_expected_descriptive_counts() {
    let summary = retained_summary();
    assert_eq!(summary.total_episodes, 6);
    assert_eq!(summary.surfaced, 5);
    assert_eq!(summary.quiet, 1);
    assert_eq!(summary.consulted, 5);

    let changed = summary
        .by_outcome
        .iter()
        .find(|count| count.key == "changed_next_action")
        .unwrap();
    assert_eq!(changed.count, 5);
    assert_eq!(changed.episode_ids.len(), 5);

    let quiet = summary
        .by_outcome
        .iter()
        .find(|count| count.key == "correct_quiet_negative")
        .unwrap();
    assert_eq!(quiet.count, 1);
    assert_eq!(
        quiet.episode_ids,
        vec!["github-actions:run/32242752523#active-work-heads-up"]
    );
}

#[test]
fn every_count_keeps_the_exact_episode_ids_it_summarizes() {
    let summary = retained_summary();

    for count in summary
        .by_outcome
        .iter()
        .chain(summary.by_evidence_kind.iter())
    {
        assert_eq!(count.count, count.episode_ids.len());
        assert!(!count.episode_ids.is_empty());
        let mut sorted = count.episode_ids.clone();
        sorted.sort();
        assert_eq!(count.episode_ids, sorted);
    }
}

#[test]
fn evidence_families_remain_separate_instead_of_becoming_one_score() {
    let summary = retained_summary();
    let keys = summary
        .by_evidence_kind
        .iter()
        .map(|count| count.key.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        keys,
        vec![
            "active-work-heads-up",
            "concurrent-main-head-movement",
            "known-stale-observation-counterexample",
            "project-memory-contract-collision",
            "project-memory-relation-strengthening",
            "refinement-investigation-demand-gate",
        ]
    );
}

#[test]
fn summary_round_trips_as_plain_descriptive_data() {
    let summary = retained_summary();
    let json = serde_json::to_string(&summary).unwrap();
    let decoded: BehavioralSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, summary);
}

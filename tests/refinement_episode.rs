#![allow(dead_code)]

use std::collections::BTreeSet;

#[path = "../src/behavioral_episode.rs"]
mod behavioral_episode;
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;

use behavioral_episode::parse_behavioral_episode_batch;
use behavioral_receipt::BehavioralOutcome;
use refinement_episode::{
    HeldOutStatus, MAX_REFINEMENT_EPISODE_BATCH_BYTES, RefinementStatus,
    parse_refinement_episode_batch, validate_refinement_episode_batch,
};

const CORPUS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");
const BEHAVIORAL_CORPUS: &[u8] =
    include_bytes!("../research/behavioral-episodes/cultist-collaboration-v1.json");

#[test]
fn retained_cultist_refinement_corpus_preserves_selected_and_rejected_candidates() {
    let batch = parse_refinement_episode_batch(CORPUS).unwrap();
    assert_eq!(batch.episodes.len(), 3);

    let justification = &batch.episodes[0];
    assert_eq!(justification.id, "justification/open-obligation-v1");
    assert_eq!(
        justification.selected_transition.as_deref(),
        Some("allow-open-zero-edge")
    );
    assert!(justification.behavioral_episode_ids.is_empty());
    assert_eq!(
        justification.candidate_refinements[0].status,
        RefinementStatus::Weakened
    );

    let history = &batch.episodes[1];
    assert_eq!(history.id, "history/oxc-edit-class-v1");
    assert_eq!(history.candidate_refinements.len(), 3);
    assert_eq!(
        history.candidate_refinements[0]
            .replay_result
            .held_out_status,
        HeldOutStatus::Passed
    );
    assert!(
        history
            .candidate_refinements
            .iter()
            .any(|candidate| { candidate.status == RefinementStatus::RejectedNoImprovement })
    );
    assert!(
        history
            .candidate_refinements
            .iter()
            .any(|candidate| { candidate.status == RefinementStatus::RejectedOverfit })
    );

    let project_memory = &batch.episodes[2];
    assert_eq!(
        project_memory.id,
        "project-memory/primary-case-contract-collision-v1"
    );
    assert_eq!(
        project_memory.candidate_refinements[0].status,
        RefinementStatus::Split
    );
    assert_eq!(
        project_memory.behavioral_episode_ids,
        vec!["project-memory:primary-case-contract-collision:9792bfe->df5ae59"]
    );
}

#[test]
fn selected_transition_must_name_a_kept_candidate() {
    let mut batch = parse_refinement_episode_batch(CORPUS).unwrap();
    batch.episodes[1].selected_transition = Some("singleton-commit-partition".to_string());
    let error = validate_refinement_episode_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("rejected status"));
}

#[test]
fn kept_candidate_cannot_lose_expected_replay_cases() {
    let mut batch = parse_refinement_episode_batch(CORPUS).unwrap();
    batch.episodes[0].candidate_refinements[0]
        .replay_result
        .expected_cases_lost = 1;
    let error = validate_refinement_episode_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("loses expected replay cases"));
}

#[test]
fn no_improvement_candidate_cannot_claim_a_resolved_counterexample() {
    let mut batch = parse_refinement_episode_batch(CORPUS).unwrap();
    let candidate = batch.episodes[1]
        .candidate_refinements
        .iter_mut()
        .find(|candidate| candidate.status == RefinementStatus::RejectedNoImprovement)
        .unwrap();
    candidate.replay_result.counterexamples_resolved = 1;
    let error = validate_refinement_episode_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("preserve the baseline replay"));
}

#[test]
fn candidate_discriminator_must_be_admitted_by_the_episode() {
    let mut batch = parse_refinement_episode_batch(CORPUS).unwrap();
    batch.episodes[0].candidate_refinements[0].discriminator_refs =
        vec!["invented_discriminator".to_string()];
    let error = validate_refinement_episode_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("unadmitted discriminator"));
}

#[test]
fn behavioral_episode_links_resolve_against_retained_observation_corpus() {
    let refinement_batch = parse_refinement_episode_batch(CORPUS).unwrap();
    let behavioral_batch = parse_behavioral_episode_batch(BEHAVIORAL_CORPUS).unwrap();
    let behavioral_ids = behavioral_batch
        .episodes
        .iter()
        .map(|episode| episode.episode_id.as_str())
        .collect::<BTreeSet<_>>();

    let linked_ids = refinement_batch
        .episodes
        .iter()
        .flat_map(|episode| episode.behavioral_episode_ids.iter())
        .collect::<Vec<_>>();
    assert_eq!(linked_ids.len(), 1);
    assert!(behavioral_ids.contains(linked_ids[0].as_str()));

    let linked_episode = behavioral_batch
        .episodes
        .iter()
        .find(|episode| episode.episode_id == *linked_ids[0])
        .unwrap();
    assert_eq!(
        linked_episode.receipt.outcome,
        BehavioralOutcome::ChangedNextAction
    );
}

#[test]
fn behavioral_episode_links_are_optional_and_explicit() {
    let mut batch = parse_refinement_episode_batch(CORPUS).unwrap();
    batch.episodes[0].behavioral_episode_ids = vec!["observed:episode-1".to_string()];
    validate_refinement_episode_batch(&batch).unwrap();

    let encoded = serde_json::to_vec(&batch).unwrap();
    let decoded = parse_refinement_episode_batch(&encoded).unwrap();
    assert_eq!(
        decoded.episodes[0].behavioral_episode_ids,
        vec!["observed:episode-1"]
    );
}

#[test]
fn duplicate_episode_ids_fail_before_any_aggregate_can_count_them() {
    let mut batch = parse_refinement_episode_batch(CORPUS).unwrap();
    batch.episodes[1].id = batch.episodes[0].id.clone();
    let error = validate_refinement_episode_batch(&batch).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("duplicate refinement episode id")
    );
}

#[test]
fn oversized_batch_rejects_before_json_parsing() {
    let bytes = vec![b' '; MAX_REFINEMENT_EPISODE_BATCH_BYTES + 1];
    let error = parse_refinement_episode_batch(&bytes).unwrap_err();
    assert!(error.to_string().contains("exceeds"));
}

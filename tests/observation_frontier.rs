#![allow(dead_code)]

use std::collections::BTreeSet;

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;

use discriminator_observation::{
    DiscriminatorObservationBatch, DiscriminatorValueState, parse_discriminator_observation_batch,
};
use observation_frontier::{
    MAX_OBSERVATION_FRONTIER_REQUEST_BYTES, OBSERVATION_FRONTIER_SCHEMA_VERSION,
    ObservationFrontierRequest, ObservationFrontierStatus, ObservationRequirement,
    evaluate_observation_frontiers, parse_observation_frontier_request,
};
use refinement_episode::parse_refinement_episode_batch;

const OBSERVATIONS: &[u8] =
    include_bytes!("../research/discriminator-observations/cultist-v1.json");
const REFINEMENTS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");

fn observation_batch() -> DiscriminatorObservationBatch {
    parse_discriminator_observation_batch(OBSERVATIONS).unwrap()
}

fn requirement(discriminator_id: &str, subject_ref: &str) -> ObservationRequirement {
    ObservationRequirement {
        discriminator_id: discriminator_id.to_string(),
        subject_ref: subject_ref.to_string(),
    }
}

fn request(
    requirements: Vec<ObservationRequirement>,
    observations: DiscriminatorObservationBatch,
) -> ObservationFrontierRequest {
    ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements,
        observations,
    }
}

#[test]
fn retained_selected_refinement_discriminators_resolve_current() {
    let observations = observation_batch();
    let refinements = parse_refinement_episode_batch(REFINEMENTS).unwrap();
    let mut requirements = Vec::new();

    for episode in &refinements.episodes {
        let selected = episode.selected_transition.as_ref().unwrap();
        let candidate = episode
            .candidate_refinements
            .iter()
            .find(|candidate| candidate.id == *selected)
            .unwrap();
        for discriminator_id in &candidate.discriminator_refs {
            let matches = observations
                .observations
                .iter()
                .filter(|observation| {
                    observation.discriminator_id == *discriminator_id
                        && matches!(
                            &observation.value_state,
                            DiscriminatorValueState::Known { .. }
                        )
                })
                .collect::<Vec<_>>();
            assert_eq!(
                matches.len(),
                1,
                "retained fixture requires one current subject for {discriminator_id}"
            );
            requirements.push(requirement(discriminator_id, &matches[0].subject_ref));
        }
    }

    let evaluation = evaluate_observation_frontiers(&request(requirements, observations)).unwrap();
    assert_eq!(evaluation.frontiers.len(), 4);
    assert!(
        evaluation
            .frontiers
            .iter()
            .all(|frontier| frontier.status == ObservationFrontierStatus::Current)
    );
}

#[test]
fn removing_one_selected_observation_creates_explicit_missing_frontier() {
    let mut observations = observation_batch();
    let removed = observations.observations.remove(0);
    let evaluation = evaluate_observation_frontiers(&request(
        vec![requirement(&removed.discriminator_id, &removed.subject_ref)],
        observations,
    ))
    .unwrap();

    let frontier = &evaluation.frontiers[0];
    assert_eq!(frontier.status, ObservationFrontierStatus::Missing);
    assert!(frontier.current.is_empty());
    assert!(frontier.unknown.is_empty());
    assert!(frontier.invalid.is_empty());
}

#[test]
fn same_discriminator_on_wrong_subject_does_not_satisfy_requirement() {
    let mut observations = observation_batch();
    let mut wrong_subject = observations.observations.remove(0);
    let discriminator_id = wrong_subject.discriminator_id.clone();
    let required_subject = wrong_subject.subject_ref.clone();
    wrong_subject.observation_id = "justification/wrong-subject:clearing-presence".to_string();
    wrong_subject.subject_ref = "refinement:justification/another-obligation".to_string();
    observations.observations.push(wrong_subject);

    let evaluation = evaluate_observation_frontiers(&request(
        vec![requirement(&discriminator_id, &required_subject)],
        observations,
    ))
    .unwrap();
    let frontier = &evaluation.frontiers[0];
    assert_eq!(frontier.status, ObservationFrontierStatus::Missing);
    assert_eq!(frontier.other_subject.len(), 1);
}

#[test]
fn unknown_matching_observation_produces_unknown_frontier() {
    let mut observations = observation_batch();
    let discriminator_id = observations.observations[0].discriminator_id.clone();
    let subject_ref = observations.observations[0].subject_ref.clone();
    observations.observations[0].value_state = DiscriminatorValueState::Unknown {
        reason_ref: "applicability:missing-current-revision".to_string(),
    };

    let evaluation = evaluate_observation_frontiers(&request(
        vec![requirement(&discriminator_id, &subject_ref)],
        observations,
    ))
    .unwrap();
    let frontier = &evaluation.frontiers[0];
    assert_eq!(frontier.status, ObservationFrontierStatus::Unknown);
    assert_eq!(frontier.unknown.len(), 1);
    assert!(frontier.current.is_empty());
}

#[test]
fn invalid_matching_observation_produces_invalid_frontier() {
    let mut observations = observation_batch();
    let discriminator_id = observations.observations[1].discriminator_id.clone();
    let subject_ref = observations.observations[1].subject_ref.clone();
    observations.observations[1].value_state = DiscriminatorValueState::Invalid {
        reason_ref: "applicability:revision-moved".to_string(),
    };

    let evaluation = evaluate_observation_frontiers(&request(
        vec![requirement(&discriminator_id, &subject_ref)],
        observations,
    ))
    .unwrap();
    let frontier = &evaluation.frontiers[0];
    assert_eq!(frontier.status, ObservationFrontierStatus::Invalid);
    assert_eq!(frontier.invalid.len(), 1);
    assert!(frontier.current.is_empty());
}

#[test]
fn current_precedes_unknown_while_preserving_both_receipts() {
    let mut observations = observation_batch();
    let mut unknown = observations.observations[1].clone();
    unknown.observation_id = "history/oxc-edit-class-v1:unknown-second-source".to_string();
    unknown.source_receipt = "research:cohort-refinement.md#unknown-edit-class".to_string();
    unknown.value_state = DiscriminatorValueState::Unknown {
        reason_ref: "classifier:missing-source-version".to_string(),
    };
    let discriminator_id = unknown.discriminator_id.clone();
    let subject_ref = unknown.subject_ref.clone();
    observations.observations.push(unknown);

    let evaluation = evaluate_observation_frontiers(&request(
        vec![requirement(&discriminator_id, &subject_ref)],
        observations,
    ))
    .unwrap();
    let frontier = &evaluation.frontiers[0];
    assert_eq!(frontier.status, ObservationFrontierStatus::Current);
    assert_eq!(frontier.current.len(), 1);
    assert_eq!(frontier.unknown.len(), 1);
}

#[test]
fn unknown_precedes_invalid_when_no_current_observation_exists() {
    let mut observations = observation_batch();
    let mut invalid = observations.observations[0].clone();
    invalid.observation_id = "justification/open-obligation-v1:invalid-second-source".to_string();
    invalid.source_receipt = "research:durable-unknown-obligation.md#stale".to_string();
    invalid.value_state = DiscriminatorValueState::Invalid {
        reason_ref: "applicability:head-moved".to_string(),
    };
    let discriminator_id = invalid.discriminator_id.clone();
    let subject_ref = invalid.subject_ref.clone();
    observations.observations[0].value_state = DiscriminatorValueState::Unknown {
        reason_ref: "applicability:missing-head".to_string(),
    };
    observations.observations.push(invalid);

    let evaluation = evaluate_observation_frontiers(&request(
        vec![requirement(&discriminator_id, &subject_ref)],
        observations,
    ))
    .unwrap();
    let frontier = &evaluation.frontiers[0];
    assert_eq!(frontier.status, ObservationFrontierStatus::Unknown);
    assert_eq!(frontier.unknown.len(), 1);
    assert_eq!(frontier.invalid.len(), 1);
}

#[test]
fn duplicate_exact_requirement_rejects() {
    let observations = observation_batch();
    let requirement = requirement(
        &observations.observations[0].discriminator_id,
        &observations.observations[0].subject_ref,
    );
    let error = evaluate_observation_frontiers(&request(
        vec![requirement.clone(), requirement],
        observations,
    ))
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("duplicate observation requirement")
    );
}

#[test]
fn frontier_output_order_is_deterministic() {
    let observations = observation_batch();
    let requirements = observations
        .observations
        .iter()
        .rev()
        .map(|observation| requirement(&observation.discriminator_id, &observation.subject_ref))
        .collect::<Vec<_>>();
    let evaluation = evaluate_observation_frontiers(&request(requirements, observations)).unwrap();
    let keys = evaluation
        .frontiers
        .iter()
        .map(|frontier| {
            (
                frontier.discriminator_id.as_str(),
                frontier.subject_ref.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let sorted = keys.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(keys, sorted.into_iter().collect::<Vec<_>>());
}

#[test]
fn request_json_round_trip_revalidates_the_frontier_input() {
    let observations = observation_batch();
    let request = request(
        vec![requirement(
            &observations.observations[0].discriminator_id,
            &observations.observations[0].subject_ref,
        )],
        observations,
    );
    let encoded = serde_json::to_vec(&request).unwrap();
    let decoded = parse_observation_frontier_request(&encoded).unwrap();
    assert_eq!(decoded, request);
}

#[test]
fn oversized_request_rejects_before_json_parsing() {
    let bytes = vec![b' '; MAX_OBSERVATION_FRONTIER_REQUEST_BYTES + 1];
    let error = parse_observation_frontier_request(&bytes).unwrap_err();
    assert!(error.to_string().contains("exceeds"));
}

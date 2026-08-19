#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;

use discriminator_observation::{
    DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION, DiscriminatorObservation, DiscriminatorValueState,
    MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES, ObservationApplicabilityStatus,
    enumerate_discriminator_partitions, parse_discriminator_observation_batch,
    validate_discriminator_observation_batch,
};
use refinement_episode::parse_refinement_episode_batch;

const OBSERVATIONS: &[u8] =
    include_bytes!("../research/discriminator-observations/cultist-v1.json");
const REFINEMENTS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");

#[test]
fn retained_observations_cover_every_selected_refinement_discriminator() {
    let observations = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    let enumeration = enumerate_discriminator_partitions(&observations).unwrap();
    let refinements = parse_refinement_episode_batch(REFINEMENTS).unwrap();

    let current = enumeration
        .discriminators
        .iter()
        .map(|discriminator| {
            (
                discriminator.discriminator_id.as_str(),
                !discriminator.known_partitions.is_empty(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for episode in &refinements.episodes {
        let selected = episode
            .selected_transition
            .as_ref()
            .expect("retained fixture has selected transition");
        let candidate = episode
            .candidate_refinements
            .iter()
            .find(|candidate| candidate.id == *selected)
            .expect("selected candidate exists");
        for discriminator in &candidate.discriminator_refs {
            assert_eq!(
                current.get(discriminator.as_str()),
                Some(&true),
                "selected discriminator {discriminator} lacks a current KNOWN+APPLIES observation"
            );
        }
    }
}

#[test]
fn known_applicable_observations_enumerate_by_discriminator_and_value() {
    let batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    let enumeration = enumerate_discriminator_partitions(&batch).unwrap();
    assert_eq!(
        enumeration.schema_version,
        DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION
    );
    assert_eq!(enumeration.discriminators.len(), 4);
    assert!(enumeration.discriminators.iter().all(|discriminator| {
        discriminator.known_partitions.len() == 1
            && discriminator.unknown.is_empty()
            && discriminator.invalid.is_empty()
    }));
}

#[test]
fn unknown_value_with_applicable_source_stays_unknown() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    batch.observations[0].value_state = DiscriminatorValueState::Unknown {
        reason_ref: "classifier:missing-value".to_string(),
    };

    let enumeration = enumerate_discriminator_partitions(&batch).unwrap();
    let discriminator = enumeration
        .discriminators
        .iter()
        .find(|item| item.discriminator_id == "clearing_evidence_presence")
        .unwrap();
    assert!(discriminator.known_partitions.is_empty());
    assert_eq!(discriminator.unknown.len(), 1);
    assert_eq!(
        discriminator.unknown[0].value_unknown_reason_ref.as_deref(),
        Some("classifier:missing-value")
    );
}

#[test]
fn known_value_with_unknown_applicability_stays_unknown_and_preserves_value() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    batch.observations[0].applicability.status = ObservationApplicabilityStatus::Unknown;
    batch.observations[0].applicability.receipt_ref =
        "applicability:missing-current-revision".to_string();

    let enumeration = enumerate_discriminator_partitions(&batch).unwrap();
    let discriminator = enumeration
        .discriminators
        .iter()
        .find(|item| item.discriminator_id == "clearing_evidence_presence")
        .unwrap();
    assert!(discriminator.known_partitions.is_empty());
    assert_eq!(discriminator.unknown.len(), 1);
    assert_eq!(
        discriminator.unknown[0].known_value_ref.as_deref(),
        Some("absent")
    );
}

#[test]
fn known_value_with_invalid_applicability_stays_invalid_and_preserves_value() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    batch.observations[1].applicability.status = ObservationApplicabilityStatus::Invalid;
    batch.observations[1].applicability.receipt_ref = "applicability:revision-moved".to_string();

    let enumeration = enumerate_discriminator_partitions(&batch).unwrap();
    let discriminator = enumeration
        .discriminators
        .iter()
        .find(|item| item.discriminator_id == "edit_class")
        .unwrap();
    assert!(discriminator.known_partitions.is_empty());
    assert_eq!(discriminator.invalid.len(), 1);
    assert_eq!(
        discriminator.invalid[0].known_value_ref.as_deref(),
        Some("syntax_changed")
    );
}

#[test]
fn exact_duplicate_observation_identity_rejects() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    batch.observations.push(batch.observations[0].clone());
    let error = validate_discriminator_observation_batch(&batch).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("duplicate discriminator observation id")
    );
}

#[test]
fn conflicting_duplicate_observation_identity_rejects() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    let mut conflicting = batch.observations[0].clone();
    conflicting.value_state = DiscriminatorValueState::Known {
        value_ref: "observed".to_string(),
    };
    batch.observations.push(conflicting);
    let error = validate_discriminator_observation_batch(&batch).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("conflicting discriminator observation id")
    );
}

#[test]
fn missing_source_receipt_rejects() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    batch.observations[0].source_receipt.clear();
    let error = validate_discriminator_observation_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("source_receipt"));
}

#[test]
fn missing_applicability_receipt_rejects() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    batch.observations[0].applicability.receipt_ref.clear();
    let error = validate_discriminator_observation_batch(&batch).unwrap_err();
    assert!(error.to_string().contains("applicability receipt_ref"));
}

#[test]
fn same_value_from_distinct_sources_preserves_each_observation_identity() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    let mut repeated = batch.observations[1].clone();
    repeated.observation_id = "history/oxc-edit-class-v1:second-source".to_string();
    repeated.source_receipt = "research:cohort-refinement.md#supplied-edit-class".to_string();
    batch.observations.push(repeated);

    let enumeration = enumerate_discriminator_partitions(&batch).unwrap();
    let edit_class = enumeration
        .discriminators
        .iter()
        .find(|item| item.discriminator_id == "edit_class")
        .unwrap();
    assert_eq!(edit_class.known_partitions.len(), 1);
    let observations = &edit_class.known_partitions[0].observations;
    assert_eq!(observations.len(), 2);
    assert_ne!(
        observations[0].observation_id,
        observations[1].observation_id
    );
    assert_ne!(
        observations[0].source_receipt,
        observations[1].source_receipt
    );
}

#[test]
fn enumeration_order_and_json_round_trip_are_deterministic() {
    let batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    let first = enumerate_discriminator_partitions(&batch).unwrap();
    let second = enumerate_discriminator_partitions(&batch).unwrap();
    assert_eq!(first, second);

    let ids = first
        .discriminators
        .iter()
        .map(|item| item.discriminator_id.as_str())
        .collect::<Vec<_>>();
    let sorted = ids.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(ids, sorted.into_iter().collect::<Vec<_>>());

    let encoded = serde_json::to_vec(&batch).unwrap();
    let decoded = parse_discriminator_observation_batch(&encoded).unwrap();
    assert_eq!(decoded, batch);
}

#[test]
fn oversized_batch_rejects_before_json_parsing() {
    let bytes = vec![b' '; MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES + 1];
    let error = parse_discriminator_observation_batch(&bytes).unwrap_err();
    assert!(error.to_string().contains("exceeds"));
}

#[test]
fn value_spelling_remains_an_opaque_reference() {
    let mut batch = parse_discriminator_observation_batch(OBSERVATIONS).unwrap();
    let DiscriminatorObservation { value_state, .. } = &mut batch.observations[0];
    *value_state = DiscriminatorValueState::Known {
        value_ref: "approve_and_merge_everything".to_string(),
    };
    let enumeration = enumerate_discriminator_partitions(&batch).unwrap();
    let partition = &enumeration.discriminators[0].known_partitions[0];
    assert_eq!(partition.observations.len(), 1);
}

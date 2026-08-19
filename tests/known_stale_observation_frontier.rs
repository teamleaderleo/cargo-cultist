#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;

use applicability::{
    APPLICABILITY_SCHEMA_VERSION, ApplicabilityQuery, ApplicabilityStatus, EvaluationContext,
    EvidenceRequirements, evaluate_query,
};
use discriminator_observation::{
    DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION, DiscriminatorObservation,
    DiscriminatorObservationBatch, DiscriminatorValueState,
};
use observation_frontier::{
    OBSERVATION_FRONTIER_SCHEMA_VERSION, ObservationFrontierRequest, ObservationFrontierStatus,
    ObservationRequirement, evaluate_observation_frontiers,
};

fn known_observation(applicability_ref: &str) -> DiscriminatorObservation {
    DiscriminatorObservation {
        observation_id: "obs:edit-class:subject-a".to_string(),
        discriminator_id: "edit_class".to_string(),
        subject_ref: "commit:subject-a".to_string(),
        source_receipt: "receipt:syntax-cohort:subject-a".to_string(),
        value_state: DiscriminatorValueState::Known {
            value_ref: "syntax_changed".to_string(),
        },
        applicability_ref: Some(applicability_ref.to_string()),
    }
}

fn frontier_for(observation: DiscriminatorObservation) -> ObservationFrontierStatus {
    let request = ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: vec![ObservationRequirement {
            discriminator_id: "edit_class".to_string(),
            subject_ref: "commit:subject-a".to_string(),
        }],
        observations: DiscriminatorObservationBatch {
            schema_version: DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
            observations: vec![observation],
        },
    };
    evaluate_observation_frontiers(&request).unwrap().frontiers[0].status
}

#[test]
fn known_value_can_be_called_current_while_shared_applicability_is_invalid() {
    let applicability = evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            repository: Some("owner/repo".to_string()),
            revision: Some("old-head".to_string()),
            work: None,
            scope: None,
        },
        context: EvaluationContext {
            repository: Some("owner/repo".to_string()),
            revision: Some("new-head".to_string()),
            work: None,
            path: None,
        },
    })
    .unwrap();
    assert_eq!(applicability.status, ApplicabilityStatus::Invalid);

    let status = frontier_for(known_observation(
        "applicability:owner/repo:old-head->new-head:invalid",
    ));
    assert_eq!(status, ObservationFrontierStatus::Current);
}

#[test]
fn known_value_can_be_called_current_while_required_current_revision_is_missing() {
    let applicability = evaluate_query(&ApplicabilityQuery {
        schema_version: APPLICABILITY_SCHEMA_VERSION,
        requirements: EvidenceRequirements {
            repository: Some("owner/repo".to_string()),
            revision: Some("exact-head".to_string()),
            work: None,
            scope: None,
        },
        context: EvaluationContext {
            repository: Some("owner/repo".to_string()),
            revision: None,
            work: None,
            path: None,
        },
    })
    .unwrap();
    assert_eq!(applicability.status, ApplicabilityStatus::Unknown);

    let status = frontier_for(known_observation(
        "applicability:owner/repo:exact-head:current-revision-missing",
    ));
    assert_eq!(status, ObservationFrontierStatus::Current);
}

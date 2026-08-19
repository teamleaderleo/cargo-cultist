#![allow(dead_code)]

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;

use discriminator_observation::{
    DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION, DiscriminatorObservation,
    DiscriminatorObservationBatch, DiscriminatorValueState, ObservationApplicability,
    ObservationApplicabilityStatus,
};
use observation_frontier::{
    OBSERVATION_FRONTIER_SCHEMA_VERSION, ObservationFrontierRequest, ObservationFrontierStatus,
    ObservationRequirement, evaluate_observation_frontiers,
};

const DISCRIMINATOR: &str = "edit_class";
const REQUIRED_SUBJECT: &str = "path:src/foo.rs";
const OTHER_SUBJECT: &str = "path:src/bar.rs";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ActionClass {
    ProceedBounded,
    RejectStale,
    RequestDiscriminator,
}

fn observation(
    id: &str,
    subject: &str,
    status: ObservationApplicabilityStatus,
    applicability_receipt: &str,
) -> DiscriminatorObservation {
    DiscriminatorObservation {
        observation_id: id.to_string(),
        discriminator_id: DISCRIMINATOR.to_string(),
        subject_ref: subject.to_string(),
        source_receipt: format!("source:{id}"),
        value_state: DiscriminatorValueState::Known {
            value_ref: "syntax_changed".to_string(),
        },
        applicability: ObservationApplicability {
            status,
            receipt_ref: applicability_receipt.to_string(),
        },
    }
}

fn evaluate_action(observations: Vec<DiscriminatorObservation>) -> ActionClass {
    let request = ObservationFrontierRequest {
        schema_version: OBSERVATION_FRONTIER_SCHEMA_VERSION,
        requirements: vec![ObservationRequirement {
            discriminator_id: DISCRIMINATOR.to_string(),
            subject_ref: REQUIRED_SUBJECT.to_string(),
        }],
        observations: DiscriminatorObservationBatch {
            schema_version: DISCRIMINATOR_OBSERVATION_SCHEMA_VERSION,
            observations,
        },
    };

    let evaluation = evaluate_observation_frontiers(&request).unwrap();
    let frontier = evaluation.frontiers.first().expect("one frontier");
    match frontier.status {
        ObservationFrontierStatus::Current => ActionClass::ProceedBounded,
        ObservationFrontierStatus::Invalid => ActionClass::RejectStale,
        ObservationFrontierStatus::Unknown | ObservationFrontierStatus::Missing => {
            ActionClass::RequestDiscriminator
        }
    }
}

#[test]
fn relevant_applicability_mutation_must_change_the_next_action() {
    let canonical = evaluate_action(vec![observation(
        "required",
        REQUIRED_SUBJECT,
        ObservationApplicabilityStatus::Applies,
        "applicability:required:current-head",
    )]);
    let mutated = evaluate_action(vec![observation(
        "required",
        REQUIRED_SUBJECT,
        ObservationApplicabilityStatus::Invalid,
        "applicability:required:moved-head",
    )]);

    assert_eq!(canonical, ActionClass::ProceedBounded);
    assert_eq!(mutated, ActionClass::RejectStale);
    assert_ne!(canonical, mutated);
}

#[test]
fn missing_current_applicability_must_request_the_discriminator() {
    let canonical = evaluate_action(vec![observation(
        "required",
        REQUIRED_SUBJECT,
        ObservationApplicabilityStatus::Applies,
        "applicability:required:current-head",
    )]);
    let mutated = evaluate_action(vec![observation(
        "required",
        REQUIRED_SUBJECT,
        ObservationApplicabilityStatus::Unknown,
        "applicability:required:missing-current-revision",
    )]);

    assert_eq!(canonical, ActionClass::ProceedBounded);
    assert_eq!(mutated, ActionClass::RequestDiscriminator);
}

#[test]
fn irrelevant_other_subject_mutation_must_not_perturb_the_action() {
    let required = observation(
        "required",
        REQUIRED_SUBJECT,
        ObservationApplicabilityStatus::Applies,
        "applicability:required:current-head",
    );

    let canonical = evaluate_action(vec![
        required.clone(),
        observation(
            "other",
            OTHER_SUBJECT,
            ObservationApplicabilityStatus::Applies,
            "applicability:other:current-head",
        ),
    ]);
    let mutated = evaluate_action(vec![
        required,
        observation(
            "other",
            OTHER_SUBJECT,
            ObservationApplicabilityStatus::Invalid,
            "applicability:other:moved-head",
        ),
    ]);

    assert_eq!(canonical, ActionClass::ProceedBounded);
    assert_eq!(mutated, ActionClass::ProceedBounded);
    assert_eq!(canonical, mutated);
}

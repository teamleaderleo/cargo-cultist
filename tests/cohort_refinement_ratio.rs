#[path = "../src/cohort_refinement.rs"]
mod cohort_refinement;

use std::collections::BTreeMap;

use cohort_refinement::{
    COHORT_REFINEMENT_SCHEMA_VERSION, CohortObservation, ObservationOutcome, RefinementRequest,
    RefinementStatus, evaluate_refinements,
};

fn facts(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn observation(id: String, outcome: ObservationOutcome, class: &str) -> CohortObservation {
    CohortObservation {
        id,
        outcome,
        facts: facts(&[("edit_class", class)]),
    }
}

#[test]
fn excluding_one_counterexample_cannot_worsen_counterexample_proportion() {
    let mut observations = Vec::new();

    for index in 0..3 {
        observations.push(observation(
            format!("selected-support-{index}"),
            ObservationOutcome::Support,
            "selected",
        ));
    }
    observations.push(observation(
        "selected-counterexample".to_string(),
        ObservationOutcome::Counterexample,
        "selected",
    ));

    for index in 0..97 {
        observations.push(observation(
            format!("excluded-support-{index}"),
            ObservationOutcome::Support,
            "excluded",
        ));
    }
    observations.push(observation(
        "excluded-counterexample".to_string(),
        ObservationOutcome::Counterexample,
        "excluded",
    ));

    let request = RefinementRequest {
        schema_version: COHORT_REFINEMENT_SCHEMA_VERSION,
        min_support: 3,
        current_facts: facts(&[("edit_class", "selected")]),
        discriminators: vec!["edit_class".to_string()],
        observations,
    };

    let evaluation = evaluate_refinements(&request).unwrap();
    let discriminator = &evaluation.discriminators[0];

    assert_eq!(evaluation.baseline.support, 100);
    assert_eq!(evaluation.baseline.counterexamples, 2);
    assert_eq!(discriminator.current_cohort.unwrap().support, 3);
    assert_eq!(discriminator.current_cohort.unwrap().counterexamples, 1);
    assert_eq!(discriminator.excluded_counterexamples, Some(1));
    assert_eq!(discriminator.status, RefinementStatus::NoImprovement);
}

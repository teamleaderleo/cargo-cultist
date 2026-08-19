use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const COHORT_REFINEMENT_SCHEMA_VERSION: u32 = 1;
const MAX_OBSERVATIONS: usize = 4096;
const MAX_DISCRIMINATORS: usize = 64;
const MAX_ID_BYTES: usize = 256;
const MAX_FACT_BYTES: usize = 1024;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementRequest {
    pub schema_version: u32,
    pub min_support: usize,
    pub current_facts: BTreeMap<String, String>,
    pub discriminators: Vec<String>,
    pub observations: Vec<CohortObservation>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohortObservation {
    pub id: String,
    pub outcome: ObservationOutcome,
    pub facts: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationOutcome {
    Support,
    Counterexample,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohortCounts {
    pub support: usize,
    pub counterexamples: usize,
}

impl CohortCounts {
    pub fn opportunities(self) -> usize {
        self.support + self.counterexamples
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CohortPartition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub counts: CohortCounts,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefinementStatus {
    Candidate,
    NoImprovement,
    Overfit,
    UnknownCurrent,
    IncompleteEvidence,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscriminatorEvaluation {
    pub discriminator: String,
    pub status: RefinementStatus,
    pub baseline: CohortCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_cohort: Option<CohortCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_support: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_counterexamples: Option<usize>,
    pub partitions: Vec<CohortPartition>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementEvaluation {
    pub schema_version: u32,
    pub baseline: CohortCounts,
    pub discriminators: Vec<DiscriminatorEvaluation>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefinementError {
    message: String,
}

impl RefinementError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RefinementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RefinementError {}

pub fn evaluate_refinements(
    request: &RefinementRequest,
) -> Result<RefinementEvaluation, RefinementError> {
    validate_request(request)?;

    let baseline = count_observations(&request.observations);
    let mut discriminators = request.discriminators.clone();
    discriminators.sort();

    let evaluations = discriminators
        .into_iter()
        .map(|discriminator| evaluate_discriminator(request, baseline, discriminator))
        .collect();

    Ok(RefinementEvaluation {
        schema_version: COHORT_REFINEMENT_SCHEMA_VERSION,
        baseline,
        discriminators: evaluations,
    })
}

fn evaluate_discriminator(
    request: &RefinementRequest,
    baseline: CohortCounts,
    discriminator: String,
) -> DiscriminatorEvaluation {
    let mut partition_counts = BTreeMap::<Option<String>, CohortCounts>::new();
    for observation in &request.observations {
        let value = observation.facts.get(&discriminator).cloned();
        increment_counts(
            partition_counts.entry(value).or_default(),
            observation.outcome,
        );
    }

    let partitions = partition_counts
        .iter()
        .map(|(value, counts)| CohortPartition {
            value: value.clone(),
            counts: *counts,
        })
        .collect::<Vec<_>>();

    let Some(current_value) = request.current_facts.get(&discriminator).cloned() else {
        return DiscriminatorEvaluation {
            discriminator,
            status: RefinementStatus::UnknownCurrent,
            baseline,
            current_value: None,
            current_cohort: None,
            excluded_support: None,
            excluded_counterexamples: None,
            partitions,
        };
    };

    let current_cohort = partition_counts.get(&Some(current_value.clone())).copied();
    let Some(current_cohort) = current_cohort else {
        return DiscriminatorEvaluation {
            discriminator,
            status: RefinementStatus::UnknownCurrent,
            baseline,
            current_value: Some(current_value),
            current_cohort: None,
            excluded_support: None,
            excluded_counterexamples: None,
            partitions,
        };
    };

    let missing = partition_counts.get(&None).copied().unwrap_or_default();
    let excluded_support = baseline.support - current_cohort.support;
    let excluded_counterexamples = baseline.counterexamples - current_cohort.counterexamples;

    let status = if missing.counterexamples > 0 {
        RefinementStatus::IncompleteEvidence
    } else if excluded_counterexamples == 0 {
        RefinementStatus::NoImprovement
    } else if current_cohort.support < request.min_support {
        RefinementStatus::Overfit
    } else {
        RefinementStatus::Candidate
    };

    DiscriminatorEvaluation {
        discriminator,
        status,
        baseline,
        current_value: Some(current_value),
        current_cohort: Some(current_cohort),
        excluded_support: Some(excluded_support),
        excluded_counterexamples: Some(excluded_counterexamples),
        partitions,
    }
}

fn count_observations(observations: &[CohortObservation]) -> CohortCounts {
    let mut counts = CohortCounts::default();
    for observation in observations {
        increment_counts(&mut counts, observation.outcome);
    }
    counts
}

fn increment_counts(counts: &mut CohortCounts, outcome: ObservationOutcome) {
    match outcome {
        ObservationOutcome::Support => counts.support += 1,
        ObservationOutcome::Counterexample => counts.counterexamples += 1,
    }
}

fn validate_request(request: &RefinementRequest) -> Result<(), RefinementError> {
    if request.schema_version != COHORT_REFINEMENT_SCHEMA_VERSION {
        return Err(RefinementError::new(format!(
            "unsupported cohort refinement schema {}; expected {COHORT_REFINEMENT_SCHEMA_VERSION}",
            request.schema_version
        )));
    }
    if request.min_support == 0 {
        return Err(RefinementError::new("min_support must be positive"));
    }
    if request.observations.is_empty() || request.observations.len() > MAX_OBSERVATIONS {
        return Err(RefinementError::new(
            "observation cohort must be bounded and non-empty",
        ));
    }
    if request.discriminators.is_empty() || request.discriminators.len() > MAX_DISCRIMINATORS {
        return Err(RefinementError::new(
            "discriminator set must be bounded and non-empty",
        ));
    }

    let mut observation_ids = BTreeSet::new();
    for observation in &request.observations {
        validate_atom(&observation.id, "observation id", MAX_ID_BYTES)?;
        if !observation_ids.insert(observation.id.clone()) {
            return Err(RefinementError::new(format!(
                "duplicate observation id {}",
                observation.id
            )));
        }
        validate_facts(&observation.facts, "observation facts")?;
    }
    validate_facts(&request.current_facts, "current facts")?;

    let mut discriminators = BTreeSet::new();
    for discriminator in &request.discriminators {
        validate_atom(discriminator, "discriminator", MAX_ID_BYTES)?;
        if !discriminators.insert(discriminator.clone()) {
            return Err(RefinementError::new(format!(
                "duplicate discriminator {discriminator}"
            )));
        }
    }

    Ok(())
}

fn validate_facts(facts: &BTreeMap<String, String>, field: &str) -> Result<(), RefinementError> {
    for (key, value) in facts {
        validate_atom(key, &format!("{field} key"), MAX_ID_BYTES)?;
        validate_atom(value, &format!("{field} value"), MAX_FACT_BYTES)?;
    }
    Ok(())
}

fn validate_atom(value: &str, field: &str, max_bytes: usize) -> Result<(), RefinementError> {
    if value.is_empty() || value.trim() != value || value.len() > max_bytes || value.contains('\0') {
        return Err(RefinementError::new(format!(
            "{field} must be bounded canonical text"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    fn observation(
        id: String,
        outcome: ObservationOutcome,
        facts: BTreeMap<String, String>,
    ) -> CohortObservation {
        CohortObservation { id, outcome, facts }
    }

    fn request(
        current_facts: BTreeMap<String, String>,
        discriminators: Vec<&str>,
        observations: Vec<CohortObservation>,
    ) -> RefinementRequest {
        RefinementRequest {
            schema_version: COHORT_REFINEMENT_SCHEMA_VERSION,
            min_support: 3,
            current_facts,
            discriminators: discriminators.into_iter().map(str::to_string).collect(),
            observations,
        }
    }

    #[test]
    fn oxc_forward_docs_counterexample_becomes_candidate_refinement() {
        let mut observations = Vec::new();
        for index in 0..99 {
            observations.push(observation(
                format!("syntax-{index}"),
                ObservationOutcome::Support,
                facts(&[("edit_class", "syntax_changed")]),
            ));
        }
        observations.push(observation(
            "docs-license".to_string(),
            ObservationOutcome::Counterexample,
            facts(&[("edit_class", "comments_or_docs_only")]),
        ));

        let evaluation = evaluate_refinements(&request(
            facts(&[("edit_class", "syntax_changed")]),
            vec!["edit_class"],
            observations,
        ))
        .unwrap();

        let candidate = &evaluation.discriminators[0];
        assert_eq!(
            evaluation.baseline,
            CohortCounts {
                support: 99,
                counterexamples: 1
            }
        );
        assert_eq!(candidate.status, RefinementStatus::Candidate);
        assert_eq!(
            candidate.current_cohort,
            Some(CohortCounts {
                support: 99,
                counterexamples: 0
            })
        );
        assert_eq!(candidate.excluded_counterexamples, Some(1));
        assert_eq!(candidate.excluded_support, Some(0));
    }

    #[test]
    fn reverse_oxc_syntax_cohort_reports_no_improvement() {
        let mut observations = Vec::new();
        for index in 0..94 {
            observations.push(observation(
                format!("support-{index}"),
                ObservationOutcome::Support,
                facts(&[("edit_class", "syntax_changed")]),
            ));
        }
        for index in 0..6 {
            observations.push(observation(
                format!("counterexample-{index}"),
                ObservationOutcome::Counterexample,
                facts(&[("edit_class", "syntax_changed")]),
            ));
        }

        let evaluation = evaluate_refinements(&request(
            facts(&[("edit_class", "syntax_changed")]),
            vec!["edit_class"],
            observations,
        ))
        .unwrap();

        assert_eq!(
            evaluation.discriminators[0].status,
            RefinementStatus::NoImprovement
        );
        assert_eq!(
            evaluation.discriminators[0].current_cohort,
            Some(evaluation.baseline)
        );
    }

    #[test]
    fn singleton_identity_partition_is_rejected_as_overfit() {
        let observations = vec![
            observation(
                "a".to_string(),
                ObservationOutcome::Support,
                facts(&[("commit", "a")]),
            ),
            observation(
                "b".to_string(),
                ObservationOutcome::Support,
                facts(&[("commit", "b")]),
            ),
            observation(
                "c".to_string(),
                ObservationOutcome::Support,
                facts(&[("commit", "c")]),
            ),
            observation(
                "d".to_string(),
                ObservationOutcome::Counterexample,
                facts(&[("commit", "d")]),
            ),
        ];

        let evaluation = evaluate_refinements(&request(
            facts(&[("commit", "a")]),
            vec!["commit"],
            observations,
        ))
        .unwrap();

        assert_eq!(
            evaluation.discriminators[0].status,
            RefinementStatus::Overfit
        );
        assert_eq!(
            evaluation.discriminators[0].current_cohort.unwrap().support,
            1
        );
    }

    #[test]
    fn missing_current_discriminator_stays_unknown() {
        let observations = vec![
            observation(
                "a".to_string(),
                ObservationOutcome::Support,
                facts(&[("edit_class", "syntax_changed")]),
            ),
            observation(
                "b".to_string(),
                ObservationOutcome::Counterexample,
                facts(&[("edit_class", "comments_or_docs_only")]),
            ),
        ];

        let evaluation =
            evaluate_refinements(&request(BTreeMap::new(), vec!["edit_class"], observations))
                .unwrap();

        assert_eq!(
            evaluation.discriminators[0].status,
            RefinementStatus::UnknownCurrent
        );
        assert!(evaluation.discriminators[0].current_cohort.is_none());
    }

    #[test]
    fn counterexample_missing_discriminator_fact_blocks_candidate() {
        let observations = vec![
            observation(
                "a".to_string(),
                ObservationOutcome::Support,
                facts(&[("edit_class", "syntax_changed")]),
            ),
            observation(
                "b".to_string(),
                ObservationOutcome::Support,
                facts(&[("edit_class", "syntax_changed")]),
            ),
            observation(
                "c".to_string(),
                ObservationOutcome::Support,
                facts(&[("edit_class", "syntax_changed")]),
            ),
            observation(
                "unknown".to_string(),
                ObservationOutcome::Counterexample,
                BTreeMap::new(),
            ),
        ];

        let evaluation = evaluate_refinements(&request(
            facts(&[("edit_class", "syntax_changed")]),
            vec!["edit_class"],
            observations,
        ))
        .unwrap();

        assert_eq!(
            evaluation.discriminators[0].status,
            RefinementStatus::IncompleteEvidence
        );
    }

    #[test]
    fn discriminator_output_order_is_stable() {
        let observations = vec![observation(
            "a".to_string(),
            ObservationOutcome::Support,
            facts(&[("z", "1"), ("a", "1")]),
        )];
        let evaluation = evaluate_refinements(&request(
            facts(&[("z", "1"), ("a", "1")]),
            vec!["z", "a"],
            observations,
        ))
        .unwrap();

        assert_eq!(evaluation.discriminators[0].discriminator, "a");
        assert_eq!(evaluation.discriminators[1].discriminator, "z");
    }
}

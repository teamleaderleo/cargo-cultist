#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum FailureBoundary {
    None,
    SpecificationGap,
    EvidenceAbsent,
    SelectionMiss,
    AttentionMiss,
    InterpretationMiss,
    PlanningMiss,
    AffordanceMiss,
    ExecutionMiss,
    ValidationMiss,
    Unexplained,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ResponseQuality {
    Success,
    CorrectEscalation,
    Failed,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct EpisodeAssessment {
    boundary: FailureBoundary,
    response: ResponseQuality,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct EpisodeFacts {
    task_completed: bool,
    specification_complete: bool,
    decisive_evidence_required: bool,
    decisive_evidence_available: bool,
    decisive_evidence_selected: bool,
    decisive_evidence_recovered_manually: bool,
    decisive_evidence_inspected: bool,
    interpretation_matches_oracle: Option<bool>,
    plan_matches_contract: Option<bool>,
    required_affordance_available: bool,
    implementation_completed: bool,
    required_validation_completed: bool,
    requested_exact_missing_discriminator: bool,
}

impl EpisodeFacts {
    fn baseline_failure() -> Self {
        Self {
            task_completed: false,
            specification_complete: true,
            decisive_evidence_required: true,
            decisive_evidence_available: true,
            decisive_evidence_selected: true,
            decisive_evidence_recovered_manually: false,
            decisive_evidence_inspected: true,
            interpretation_matches_oracle: Some(true),
            plan_matches_contract: Some(true),
            required_affordance_available: true,
            implementation_completed: true,
            required_validation_completed: true,
            requested_exact_missing_discriminator: false,
        }
    }
}

fn assess_episode(facts: EpisodeFacts) -> EpisodeAssessment {
    if facts.task_completed {
        return EpisodeAssessment {
            boundary: FailureBoundary::None,
            response: ResponseQuality::Success,
        };
    }

    let boundary = if !facts.specification_complete {
        FailureBoundary::SpecificationGap
    } else if facts.decisive_evidence_required && !facts.decisive_evidence_available {
        FailureBoundary::EvidenceAbsent
    } else if facts.decisive_evidence_required && !facts.decisive_evidence_selected {
        FailureBoundary::SelectionMiss
    } else if facts.decisive_evidence_required && !facts.decisive_evidence_inspected {
        FailureBoundary::AttentionMiss
    } else if facts.interpretation_matches_oracle == Some(false) {
        FailureBoundary::InterpretationMiss
    } else if facts.plan_matches_contract == Some(false) {
        FailureBoundary::PlanningMiss
    } else if !facts.required_affordance_available {
        FailureBoundary::AffordanceMiss
    } else if !facts.implementation_completed {
        FailureBoundary::ExecutionMiss
    } else if !facts.required_validation_completed {
        FailureBoundary::ValidationMiss
    } else {
        FailureBoundary::Unexplained
    };

    let response = if facts.requested_exact_missing_discriminator
        && matches!(
            boundary,
            FailureBoundary::SpecificationGap
                | FailureBoundary::EvidenceAbsent
                | FailureBoundary::SelectionMiss
                | FailureBoundary::AffordanceMiss
        ) {
        ResponseQuality::CorrectEscalation
    } else {
        ResponseQuality::Failed
    };

    EpisodeAssessment { boundary, response }
}

fn boundary_chain(facts: EpisodeFacts) -> Vec<FailureBoundary> {
    let primary = assess_episode(facts).boundary;
    let mut chain = match primary {
        FailureBoundary::None | FailureBoundary::Unexplained => Vec::new(),
        boundary => vec![boundary],
    };

    if primary == FailureBoundary::SelectionMiss && facts.decisive_evidence_recovered_manually {
        let mut recovered = facts;
        recovered.decisive_evidence_selected = true;
        recovered.decisive_evidence_recovered_manually = false;
        let downstream = assess_episode(recovered).boundary;
        if !matches!(
            downstream,
            FailureBoundary::None | FailureBoundary::Unexplained
        ) {
            chain.push(downstream);
        }
    }

    chain
}

#[test]
fn identical_failed_outcome_does_not_imply_identical_worker_skill_failure() {
    let mut selection_miss = EpisodeFacts::baseline_failure();
    selection_miss.decisive_evidence_selected = false;
    selection_miss.decisive_evidence_inspected = false;
    selection_miss.interpretation_matches_oracle = None;
    selection_miss.plan_matches_contract = None;

    let mut interpretation_miss = EpisodeFacts::baseline_failure();
    interpretation_miss.interpretation_matches_oracle = Some(false);
    interpretation_miss.plan_matches_contract = None;

    assert_eq!(
        assess_episode(selection_miss).boundary,
        FailureBoundary::SelectionMiss
    );
    assert_eq!(
        assess_episode(interpretation_miss).boundary,
        FailureBoundary::InterpretationMiss
    );
    assert_ne!(
        assess_episode(selection_miss),
        assess_episode(interpretation_miss)
    );
}

#[test]
fn selected_but_uninspected_evidence_is_attention_not_interpretation() {
    let mut facts = EpisodeFacts::baseline_failure();
    facts.decisive_evidence_inspected = false;
    facts.interpretation_matches_oracle = None;
    facts.plan_matches_contract = None;

    assert_eq!(
        assess_episode(facts).boundary,
        FailureBoundary::AttentionMiss
    );
}

#[test]
fn missing_tool_affordance_is_not_model_reasoning_failure() {
    let mut facts = EpisodeFacts::baseline_failure();
    facts.required_affordance_available = false;
    facts.implementation_completed = false;
    facts.required_validation_completed = false;

    assert_eq!(
        assess_episode(facts).boundary,
        FailureBoundary::AffordanceMiss
    );
}

#[test]
fn validation_failure_is_distinct_after_correct_evidence_plan_and_execution() {
    let mut facts = EpisodeFacts::baseline_failure();
    facts.required_validation_completed = false;

    assert_eq!(
        assess_episode(facts).boundary,
        FailureBoundary::ValidationMiss
    );
}

#[test]
fn exact_escalation_is_positive_when_the_upstream_requirement_is_genuinely_missing() {
    let mut facts = EpisodeFacts::baseline_failure();
    facts.decisive_evidence_available = false;
    facts.decisive_evidence_selected = false;
    facts.decisive_evidence_inspected = false;
    facts.interpretation_matches_oracle = None;
    facts.plan_matches_contract = None;
    facts.implementation_completed = false;
    facts.required_validation_completed = false;
    facts.requested_exact_missing_discriminator = true;

    assert_eq!(
        assess_episode(facts),
        EpisodeAssessment {
            boundary: FailureBoundary::EvidenceAbsent,
            response: ResponseQuality::CorrectEscalation,
        }
    );
}

#[test]
fn escalation_does_not_excuse_ignoring_evidence_that_was_already_selected() {
    let mut facts = EpisodeFacts::baseline_failure();
    facts.decisive_evidence_inspected = false;
    facts.interpretation_matches_oracle = None;
    facts.plan_matches_contract = None;
    facts.requested_exact_missing_discriminator = true;

    assert_eq!(
        assess_episode(facts),
        EpisodeAssessment {
            boundary: FailureBoundary::AttentionMiss,
            response: ResponseQuality::Failed,
        }
    );
}

#[test]
fn selection_miss_can_precede_a_downstream_worker_failure_after_manual_recovery() {
    let mut facts = EpisodeFacts::baseline_failure();
    facts.decisive_evidence_selected = false;
    facts.decisive_evidence_recovered_manually = true;
    facts.required_validation_completed = false;

    assert_eq!(
        boundary_chain(facts),
        vec![
            FailureBoundary::SelectionMiss,
            FailureBoundary::ValidationMiss
        ]
    );
}

#[test]
fn insufficient_receipts_stay_unexplained_instead_of_becoming_model_blame() {
    let mut facts = EpisodeFacts::baseline_failure();
    facts.interpretation_matches_oracle = None;
    facts.plan_matches_contract = None;

    assert_eq!(assess_episode(facts).boundary, FailureBoundary::Unexplained);
    assert!(boundary_chain(facts).is_empty());
}

#[test]
fn completed_task_is_success_without_failure_attribution() {
    let mut facts = EpisodeFacts::baseline_failure();
    facts.task_completed = true;

    assert_eq!(
        assess_episode(facts),
        EpisodeAssessment {
            boundary: FailureBoundary::None,
            response: ResponseQuality::Success,
        }
    );
    assert!(boundary_chain(facts).is_empty());
}

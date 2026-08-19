#![allow(dead_code)]

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[path = "../src/refinement_candidate_readiness.rs"]
mod refinement_candidate_readiness;
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;
#[path = "../src/refinement_investigation_demand.rs"]
mod refinement_investigation_demand;
#[path = "../src/refinement_observation_requirement.rs"]
mod refinement_observation_requirement;

use discriminator_observation::{
    DiscriminatorObservation, DiscriminatorValueState, ObservationApplicability,
    ObservationApplicabilityStatus, parse_discriminator_observation_batch,
};
use observation_frontier::ObservationFrontierStatus;
use refinement_candidate_readiness::{
    CandidateEvidenceStatus, REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION,
    RefinementCandidateReadinessRequest, evaluate_refinement_candidate_readiness,
};
use refinement_episode::{RefinementStatus, parse_refinement_episode_batch};
use refinement_investigation_demand::{
    RefinementInvestigationDispositionStatus, evaluate_refinement_investigation_demand,
};
use refinement_observation_requirement::RefinementObservationRequirementMapping;

const OBSERVATIONS: &[u8] =
    include_bytes!("../research/discriminator-observations/cultist-v1.json");
const REFINEMENTS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");
const MAPPINGS: &[u8] =
    include_bytes!("../research/refinement-observation-requirements/cultist-v1.json");
const OXC_EPISODE: &str = "history/oxc-edit-class-v1";
const SELECTED_OXC_CANDIDATE: &str = "syntax-changing-current-cohort";
const SELECTED_OXC_SUBJECT: &str =
    "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/rules.rs";

fn request() -> RefinementCandidateReadinessRequest {
    RefinementCandidateReadinessRequest {
        schema_version: REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION,
        refinements: parse_refinement_episode_batch(REFINEMENTS).unwrap(),
        mappings: serde_json::from_slice(MAPPINGS).unwrap(),
        observations: parse_discriminator_observation_batch(OBSERVATIONS).unwrap(),
    }
}

fn current_observation(
    observation_id: &str,
    discriminator_id: &str,
    subject_ref: &str,
    value_ref: &str,
) -> DiscriminatorObservation {
    DiscriminatorObservation {
        observation_id: observation_id.to_string(),
        discriminator_id: discriminator_id.to_string(),
        subject_ref: subject_ref.to_string(),
        source_receipt: format!("research:refinement-investigation:{observation_id}"),
        value_state: DiscriminatorValueState::Known {
            value_ref: value_ref.to_string(),
        },
        applicability: ObservationApplicability {
            status: ObservationApplicabilityStatus::Applies,
            receipt_ref: format!("research:refinement-investigation:{observation_id}:applies"),
        },
    }
}

fn withhold_selected_oxc_observation(request: &mut RefinementCandidateReadinessRequest) {
    let exact = request
        .observations
        .observations
        .iter()
        .find(|observation| {
            observation.discriminator_id == "edit_class"
                && observation.subject_ref == SELECTED_OXC_SUBJECT
        })
        .unwrap()
        .clone();
    request
        .observations
        .observations
        .retain(|observation| observation.observation_id != exact.observation_id);
    request.observations.observations.push(current_observation(
        "investigation-demand:wrong-path-edit-class",
        "edit_class",
        "oxc-project/oxc@228e8e0f85c0e7aeded02c5e27fd810004d3b41a:crates/oxc_linter/src/other.rs",
        "syntax_changed",
    ));
}

fn add_rejected_oxc_candidate_evidence(request: &mut RefinementCandidateReadinessRequest) {
    let reverse_subject = "refinement:history/oxc-edit-class-v1/reverse-edit-class-control";
    request
        .mappings
        .mappings
        .push(RefinementObservationRequirementMapping {
            id: "investigation-demand:reverse-edit-class".to_string(),
            episode_id: OXC_EPISODE.to_string(),
            candidate_id: "reverse-edit-class-control".to_string(),
            discriminator_id: "edit_class".to_string(),
            subject_ref: reverse_subject.to_string(),
            source_receipt: "research:refinement-investigation:reverse-mapping".to_string(),
        });
    request.observations.observations.push(current_observation(
        "investigation-demand:reverse-current",
        "edit_class",
        reverse_subject,
        "syntax_changed",
    ));

    let singleton_subject = "refinement:history/oxc-edit-class-v1/singleton-commit-partition";
    request
        .mappings
        .mappings
        .push(RefinementObservationRequirementMapping {
            id: "investigation-demand:singleton-identity".to_string(),
            episode_id: OXC_EPISODE.to_string(),
            candidate_id: "singleton-commit-partition".to_string(),
            discriminator_id: "commit_identity".to_string(),
            subject_ref: singleton_subject.to_string(),
            source_receipt: "research:refinement-investigation:singleton-mapping".to_string(),
        });
    request.observations.observations.push(current_observation(
        "investigation-demand:singleton-current",
        "commit_identity",
        singleton_subject,
        "228e8e0f85c0e7aeded02c5e27fd810004d3b41a",
    ));
}

fn add_unselected_survivor(request: &mut RefinementCandidateReadinessRequest, with_mapping: bool) {
    let episode = request
        .refinements
        .episodes
        .iter_mut()
        .find(|episode| episode.id == OXC_EPISODE)
        .unwrap();
    let mut candidate = episode
        .candidate_refinements
        .iter()
        .find(|candidate| candidate.id == SELECTED_OXC_CANDIDATE)
        .unwrap()
        .clone();
    candidate.id = "syntax-changing-unselected-control".to_string();
    candidate.hypothesis_after.id = "syntax-changing-unselected-control-hypothesis".to_string();
    candidate.hypothesis_after.statement =
        "Synthetic surviving candidate used only to prove selection gates investigation demand."
            .to_string();
    candidate.status = RefinementStatus::Retained;
    candidate.source_receipts =
        vec!["research:refinement-investigation:unselected-control".to_string()];
    episode.candidate_refinements.push(candidate);

    if with_mapping {
        let mut mapping = request
            .mappings
            .mappings
            .iter()
            .find(|mapping| {
                mapping.episode_id == OXC_EPISODE
                    && mapping.candidate_id == SELECTED_OXC_CANDIDATE
                    && mapping.discriminator_id == "edit_class"
            })
            .unwrap()
            .clone();
        mapping.id = "investigation-demand:unselected-edit-class".to_string();
        mapping.candidate_id = "syntax-changing-unselected-control".to_string();
        mapping.source_receipt = "research:refinement-investigation:unselected-mapping".to_string();
        request.mappings.mappings.push(mapping);
    }
}

#[test]
fn default_blocked_set_would_over_acquire_replay_rejected_oxc_candidates() {
    let request = request();
    let readiness = evaluate_refinement_candidate_readiness(&request).unwrap();
    let mut blocked = readiness
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.episode_id == OXC_EPISODE
                && candidate.evidence_status == CandidateEvidenceStatus::Blocked
        })
        .map(|candidate| candidate.candidate_id.clone())
        .collect::<Vec<_>>();
    blocked.sort();
    assert_eq!(
        blocked,
        vec![
            "reverse-edit-class-control".to_string(),
            "singleton-commit-partition".to_string(),
        ]
    );

    let investigation = evaluate_refinement_investigation_demand(&request).unwrap();
    let oxc = investigation
        .candidates
        .iter()
        .filter(|candidate| candidate.episode_id == OXC_EPISODE)
        .collect::<Vec<_>>();
    let selected = oxc
        .iter()
        .copied()
        .find(|candidate| candidate.candidate_id == SELECTED_OXC_CANDIDATE)
        .unwrap();
    assert_eq!(
        selected.disposition,
        RefinementInvestigationDispositionStatus::Satisfied
    );
    assert!(selected.acquisition_frontiers.is_empty());

    for rejected_id in ["reverse-edit-class-control", "singleton-commit-partition"] {
        let rejected = oxc
            .iter()
            .copied()
            .find(|candidate| candidate.candidate_id == rejected_id)
            .unwrap();
        assert_eq!(
            rejected.disposition,
            RefinementInvestigationDispositionStatus::ReplayRejected
        );
        assert!(rejected.acquisition_frontiers.is_empty());
    }
}

#[test]
fn only_selected_survivor_emits_observation_acquisition_for_missing_exact_evidence() {
    let mut request = request();
    withhold_selected_oxc_observation(&mut request);

    let readiness = evaluate_refinement_candidate_readiness(&request).unwrap();
    assert_eq!(
        readiness
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.episode_id == OXC_EPISODE
                    && candidate.evidence_status == CandidateEvidenceStatus::Blocked
            })
            .count(),
        3
    );

    let investigation = evaluate_refinement_investigation_demand(&request).unwrap();
    let selected = investigation
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == OXC_EPISODE && candidate.candidate_id == SELECTED_OXC_CANDIDATE
        })
        .unwrap();
    assert_eq!(selected.replay_status, RefinementStatus::Weakened);
    assert_eq!(selected.evidence_status, CandidateEvidenceStatus::Blocked);
    assert_eq!(
        selected.disposition,
        RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded
    );
    assert!(selected.missing_requirement_mappings.is_empty());
    assert_eq!(selected.acquisition_frontiers.len(), 1);
    assert_eq!(
        selected.acquisition_frontiers[0].status,
        ObservationFrontierStatus::Missing
    );
    assert_eq!(
        selected.acquisition_frontiers[0].subject_ref,
        SELECTED_OXC_SUBJECT
    );
    assert_eq!(selected.acquisition_frontiers[0].other_subject.len(), 1);

    for rejected_id in ["reverse-edit-class-control", "singleton-commit-partition"] {
        let rejected = investigation
            .candidates
            .iter()
            .find(|candidate| {
                candidate.episode_id == OXC_EPISODE && candidate.candidate_id == rejected_id
            })
            .unwrap();
        assert_eq!(
            rejected.disposition,
            RefinementInvestigationDispositionStatus::ReplayRejected
        );
        assert!(rejected.acquisition_frontiers.is_empty());
    }
}

#[test]
fn selected_survivor_missing_subject_mapping_requests_mapping_research() {
    let mut request = request();
    request.mappings.mappings.retain(|mapping| {
        !(mapping.episode_id == OXC_EPISODE
            && mapping.candidate_id == SELECTED_OXC_CANDIDATE
            && mapping.discriminator_id == "edit_class")
    });

    let investigation = evaluate_refinement_investigation_demand(&request).unwrap();
    let selected = investigation
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == OXC_EPISODE && candidate.candidate_id == SELECTED_OXC_CANDIDATE
        })
        .unwrap();
    assert_eq!(
        selected.disposition,
        RefinementInvestigationDispositionStatus::RequirementMappingNeeded
    );
    assert_eq!(selected.missing_requirement_mappings, vec!["edit_class"]);
    assert!(selected.acquisition_frontiers.is_empty());
}

#[test]
fn rejected_candidates_stay_quiet_even_with_perfect_current_evidence() {
    let mut request = request();
    add_rejected_oxc_candidate_evidence(&mut request);

    let investigation = evaluate_refinement_investigation_demand(&request).unwrap();
    for (candidate_id, replay_status) in [
        (
            "reverse-edit-class-control",
            RefinementStatus::RejectedNoImprovement,
        ),
        (
            "singleton-commit-partition",
            RefinementStatus::RejectedOverfit,
        ),
    ] {
        let candidate = investigation
            .candidates
            .iter()
            .find(|candidate| {
                candidate.episode_id == OXC_EPISODE && candidate.candidate_id == candidate_id
            })
            .unwrap();
        assert_eq!(candidate.replay_status, replay_status);
        assert_eq!(candidate.evidence_status, CandidateEvidenceStatus::Current);
        assert_eq!(
            candidate.disposition,
            RefinementInvestigationDispositionStatus::ReplayRejected
        );
        assert!(candidate.acquisition_frontiers.is_empty());
    }
}

#[test]
fn unselected_replay_survivor_stays_quiet_with_current_or_blocked_evidence() {
    let mut current = request();
    add_unselected_survivor(&mut current, true);
    let current_result = evaluate_refinement_investigation_demand(&current).unwrap();
    let current_candidate = current_result
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_id == "syntax-changing-unselected-control")
        .unwrap();
    assert_eq!(current_candidate.replay_status, RefinementStatus::Retained);
    assert_eq!(
        current_candidate.evidence_status,
        CandidateEvidenceStatus::Current
    );
    assert_eq!(
        current_candidate.disposition,
        RefinementInvestigationDispositionStatus::Unselected
    );
    assert!(current_candidate.acquisition_frontiers.is_empty());

    let mut blocked = request();
    add_unselected_survivor(&mut blocked, false);
    let blocked_result = evaluate_refinement_investigation_demand(&blocked).unwrap();
    let blocked_candidate = blocked_result
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_id == "syntax-changing-unselected-control")
        .unwrap();
    assert_eq!(blocked_candidate.replay_status, RefinementStatus::Retained);
    assert_eq!(
        blocked_candidate.evidence_status,
        CandidateEvidenceStatus::Blocked
    );
    assert_eq!(
        blocked_candidate.disposition,
        RefinementInvestigationDispositionStatus::Unselected
    );
    assert_eq!(
        blocked_candidate.missing_requirement_mappings,
        vec!["edit_class"]
    );
    assert!(blocked_candidate.acquisition_frontiers.is_empty());
}

#[test]
fn mapped_unknown_and_invalid_selected_evidence_are_acquisition_frontiers() {
    let mut unknown = request();
    let observation = unknown
        .observations
        .observations
        .iter_mut()
        .find(|observation| observation.subject_ref == SELECTED_OXC_SUBJECT)
        .unwrap();
    observation.value_state = DiscriminatorValueState::Unknown {
        reason_ref: "research:refinement-investigation:unknown-control".to_string(),
    };
    let unknown_result = evaluate_refinement_investigation_demand(&unknown).unwrap();
    let unknown_candidate = unknown_result
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == OXC_EPISODE && candidate.candidate_id == SELECTED_OXC_CANDIDATE
        })
        .unwrap();
    assert_eq!(
        unknown_candidate.disposition,
        RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded
    );
    assert_eq!(unknown_candidate.acquisition_frontiers.len(), 1);
    assert_eq!(
        unknown_candidate.acquisition_frontiers[0].status,
        ObservationFrontierStatus::Unknown
    );

    let mut invalid = request();
    let observation = invalid
        .observations
        .observations
        .iter_mut()
        .find(|observation| observation.subject_ref == SELECTED_OXC_SUBJECT)
        .unwrap();
    observation.applicability.status = ObservationApplicabilityStatus::Invalid;
    observation.applicability.receipt_ref =
        "research:refinement-investigation:invalid-control".to_string();
    let invalid_result = evaluate_refinement_investigation_demand(&invalid).unwrap();
    let invalid_candidate = invalid_result
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == OXC_EPISODE && candidate.candidate_id == SELECTED_OXC_CANDIDATE
        })
        .unwrap();
    assert_eq!(
        invalid_candidate.disposition,
        RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded
    );
    assert_eq!(invalid_candidate.acquisition_frontiers.len(), 1);
    assert_eq!(
        invalid_candidate.acquisition_frontiers[0].status,
        ObservationFrontierStatus::Invalid
    );
}

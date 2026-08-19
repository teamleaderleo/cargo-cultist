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

use discriminator_observation::parse_discriminator_observation_batch;
use refinement_candidate_readiness::{
    REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION, RefinementCandidateReadinessRequest,
};
use refinement_episode::{HeldOutStatus, parse_refinement_episode_batch};
use refinement_investigation_demand::{
    RefinementInvestigationDispositionStatus, evaluate_refinement_investigation_demand,
};

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

fn set_selected_held_out(request: &mut RefinementCandidateReadinessRequest, status: HeldOutStatus) {
    let episode = request
        .refinements
        .episodes
        .iter_mut()
        .find(|episode| episode.id == OXC_EPISODE)
        .unwrap();
    let selected = episode
        .candidate_refinements
        .iter_mut()
        .find(|candidate| candidate.id == SELECTED_OXC_CANDIDATE)
        .unwrap();
    selected.replay_result.held_out_status = status;
}

fn selected<'a>(
    evaluation: &'a refinement_investigation_demand::RefinementInvestigationDemandEvaluation,
) -> &'a refinement_investigation_demand::RefinementInvestigationDisposition {
    evaluation
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == OXC_EPISODE && candidate.candidate_id == SELECTED_OXC_CANDIDATE
        })
        .unwrap()
}

#[test]
fn selected_status_survivor_with_unknown_held_out_stays_satisfied_and_preserves_unknown() {
    let mut request = request();
    set_selected_held_out(&mut request, HeldOutStatus::Unknown);

    let evaluation = evaluate_refinement_investigation_demand(&request).unwrap();
    let candidate = selected(&evaluation);
    assert_eq!(
        candidate.disposition,
        RefinementInvestigationDispositionStatus::Satisfied
    );
    assert_eq!(
        candidate.replay_result.held_out_status,
        HeldOutStatus::Unknown
    );
}

#[test]
fn selected_status_survivor_with_not_run_held_out_can_request_exact_missing_observation() {
    let mut request = request();
    set_selected_held_out(&mut request, HeldOutStatus::NotRun);
    request.observations.observations.retain(|observation| {
        !(observation.discriminator_id == "edit_class"
            && observation.subject_ref == SELECTED_OXC_SUBJECT)
    });

    let evaluation = evaluate_refinement_investigation_demand(&request).unwrap();
    let candidate = selected(&evaluation);
    assert_eq!(
        candidate.disposition,
        RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded
    );
    assert_eq!(
        candidate.replay_result.held_out_status,
        HeldOutStatus::NotRun
    );
    assert_eq!(candidate.acquisition_frontiers.len(), 1);
    assert_eq!(
        candidate.acquisition_frontiers[0].subject_ref,
        SELECTED_OXC_SUBJECT
    );
}

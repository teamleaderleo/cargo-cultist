#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/durable_obligation.rs"]
mod durable_obligation;
#[path = "../src/evidence_planner.rs"]
mod evidence_planner;
#[path = "../src/justification.rs"]
mod justification;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[path = "../src/observation_probe_bridge.rs"]
mod observation_probe_bridge;
#[path = "../src/refinement_candidate_readiness.rs"]
mod refinement_candidate_readiness;
#[path = "../src/refinement_episode.rs"]
mod refinement_episode;
#[path = "../src/refinement_investigation_demand.rs"]
mod refinement_investigation_demand;
#[path = "../src/refinement_observation_requirement.rs"]
mod refinement_observation_requirement;
#[path = "../src/rust_edit_class_source.rs"]
mod rust_edit_class_source;

use applicability::EvaluationContext;
use discriminator_observation::{
    DiscriminatorValueState, ObservationApplicabilityStatus, parse_discriminator_observation_batch,
};
use evidence_planner::{EvidencePlanStatus, ProbeSelectionPolicy};
use observation_frontier::ObservationFrontierStatus;
use observation_probe_bridge::{
    OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION, ObservationProbePlanRequest,
    ObservationProbePlanStatus, plan_observation_probe,
};
use refinement_candidate_readiness::{
    REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION, RefinementCandidateReadinessRequest,
};
use refinement_episode::{HeldOutStatus, parse_refinement_episode_batch};
use refinement_investigation_demand::{
    RefinementInvestigationDispositionStatus, evaluate_refinement_investigation_demand,
};
use rust_edit_class_source::{RustEditClassSubject, collect_rust_edit_class_source, subject_ref};

const OBSERVATIONS: &[u8] =
    include_bytes!("../research/discriminator-observations/cultist-v1.json");
const REFINEMENTS: &[u8] = include_bytes!("../research/refinement-episodes/cultist-v1.json");
const MAPPINGS: &[u8] =
    include_bytes!("../research/refinement-observation-requirements/cultist-v1.json");
const OXC_EPISODE: &str = "history/oxc-edit-class-v1";
const SELECTED_OXC_CANDIDATE: &str = "syntax-changing-current-cohort";

struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cultist-refinement-demand-plan-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cultist Test"]);
        Self { root }
    }

    fn write(&self, source: &str) {
        fs::write(self.root.join("src/lib.rs"), source).unwrap();
    }

    fn commit(&self, message: &str) -> String {
        run_git(&self.root, &["add", "src/lib.rs"]);
        run_git(&self.root, &["commit", "-q", "-m", message]);
        git_output(&self.root, &["rev-parse", "HEAD"])
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn local_source() -> (
    TempRepo,
    String,
    rust_edit_class_source::RustEditClassSourceResult,
) {
    let repo = TempRepo::new("source");
    repo.write("fn answer() -> usize { 41 }\n");
    repo.commit("root");
    repo.write("fn answer() -> usize { 42 }\n");
    let revision = repo.commit("syntax");
    let subject = RustEditClassSubject {
        repository: "owner/repo".to_string(),
        revision: revision.clone(),
        path: "src/lib.rs".to_string(),
    };
    let source = collect_rust_edit_class_source(&repo.root, &subject).unwrap();
    assert_eq!(source.current_head, revision);
    assert_eq!(
        source.observation.value_state,
        DiscriminatorValueState::Known {
            value_ref: "syntax_changed".to_string()
        }
    );
    assert_eq!(
        source.observation.applicability.status,
        ObservationApplicabilityStatus::Applies
    );
    (repo, revision, source)
}

fn request_for_subject(subject: &RustEditClassSubject) -> RefinementCandidateReadinessRequest {
    let mut request = RefinementCandidateReadinessRequest {
        schema_version: REFINEMENT_CANDIDATE_READINESS_SCHEMA_VERSION,
        refinements: parse_refinement_episode_batch(REFINEMENTS).unwrap(),
        mappings: serde_json::from_slice(MAPPINGS).unwrap(),
        observations: parse_discriminator_observation_batch(OBSERVATIONS).unwrap(),
    };
    let subject_ref = subject_ref(subject);
    let mapping = request
        .mappings
        .mappings
        .iter_mut()
        .find(|mapping| {
            mapping.episode_id == OXC_EPISODE
                && mapping.candidate_id == SELECTED_OXC_CANDIDATE
                && mapping.discriminator_id == "edit_class"
        })
        .unwrap();
    mapping.subject_ref = subject_ref;
    mapping.source_receipt = "research:refinement-demand-plan:local-subject".to_string();

    request
        .observations
        .observations
        .retain(|observation| observation.discriminator_id != "edit_class");
    request
}

fn set_selected_held_out(
    request: &mut RefinementCandidateReadinessRequest,
    held_out_status: HeldOutStatus,
) {
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
    selected.replay_result.held_out_status = held_out_status;
}

fn selected_disposition(
    request: &RefinementCandidateReadinessRequest,
) -> refinement_investigation_demand::RefinementInvestigationDisposition {
    evaluate_refinement_investigation_demand(request)
        .unwrap()
        .candidates
        .into_iter()
        .find(|candidate| {
            candidate.episode_id == OXC_EPISODE && candidate.candidate_id == SELECTED_OXC_CANDIDATE
        })
        .unwrap()
}

fn plan_frontier(
    source: &rust_edit_class_source::RustEditClassSourceResult,
    frontier: observation_frontier::ObservationFrontierReceipt,
) -> observation_probe_bridge::ObservationProbePlan {
    plan_observation_probe(&ObservationProbePlanRequest {
        schema_version: OBSERVATION_PROBE_BRIDGE_SCHEMA_VERSION,
        frontier,
        bridges: vec![source.bridge.clone()],
        context: EvaluationContext {
            repository: Some(source.subject.repository.clone()),
            revision: Some(source.subject.revision.clone()),
            work: None,
            path: Some(source.subject.path.clone()),
        },
        probes: vec![source.probe.clone()],
        allow_effectful: false,
        policy: ProbeSelectionPolicy::Conservative,
    })
    .unwrap()
}

#[test]
fn selected_acquisition_demand_routes_to_real_source_probe_and_closes_after_observation() {
    let (_repo, _revision, source) = local_source();
    let mut request = request_for_subject(&source.subject);

    let before = selected_disposition(&request);
    assert_eq!(
        before.disposition,
        RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded
    );
    assert_eq!(before.acquisition_frontiers.len(), 1);
    assert_eq!(
        before.acquisition_frontiers[0].status,
        ObservationFrontierStatus::Missing
    );
    assert_eq!(
        before.acquisition_frontiers[0].subject_ref,
        source.observation.subject_ref
    );

    let plan = plan_frontier(&source, before.acquisition_frontiers[0].clone());
    assert_eq!(plan.status, ObservationProbePlanStatus::Planned);
    assert_eq!(plan.frontier_status, ObservationFrontierStatus::Missing);
    assert_eq!(
        plan.evidence_plan.as_ref().unwrap().status,
        EvidencePlanStatus::Selected
    );
    assert_eq!(
        plan.evidence_plan
            .as_ref()
            .unwrap()
            .selected
            .as_ref()
            .unwrap()
            .id,
        source.probe.id
    );

    request
        .observations
        .observations
        .push(source.observation.clone());
    let after = selected_disposition(&request);
    assert_eq!(
        after.disposition,
        RefinementInvestigationDispositionStatus::Satisfied
    );
    assert!(after.acquisition_frontiers.is_empty());
}

#[test]
fn held_out_not_run_remains_visible_on_the_disposition_that_authorizes_planning() {
    let (_repo, _revision, source) = local_source();
    let mut request = request_for_subject(&source.subject);
    set_selected_held_out(&mut request, HeldOutStatus::NotRun);

    let disposition = selected_disposition(&request);
    assert_eq!(
        disposition.disposition,
        RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded
    );
    assert_eq!(
        disposition.replay_result.held_out_status,
        HeldOutStatus::NotRun
    );
    assert_eq!(disposition.acquisition_frontiers.len(), 1);

    let plan = plan_frontier(&source, disposition.acquisition_frontiers[0].clone());
    assert_eq!(plan.status, ObservationProbePlanStatus::Planned);
    assert_eq!(plan.frontier_status, ObservationFrontierStatus::Missing);
    assert_eq!(
        plan.evidence_plan.as_ref().unwrap().status,
        EvidencePlanStatus::Selected
    );
}

#[test]
fn non_acquisition_dispositions_emit_zero_planner_frontiers() {
    let (_repo, _revision, source) = local_source();
    let mut current = request_for_subject(&source.subject);
    current
        .observations
        .observations
        .push(source.observation.clone());
    let current_result = evaluate_refinement_investigation_demand(&current).unwrap();
    let selected = current_result
        .candidates
        .iter()
        .find(|candidate| {
            candidate.episode_id == OXC_EPISODE && candidate.candidate_id == SELECTED_OXC_CANDIDATE
        })
        .unwrap();
    assert_eq!(
        selected.disposition,
        RefinementInvestigationDispositionStatus::Satisfied
    );
    assert!(selected.acquisition_frontiers.is_empty());
    assert!(
        current_result
            .candidates
            .iter()
            .filter(|candidate| candidate.episode_id == OXC_EPISODE)
            .filter(|candidate| {
                matches!(
                    candidate.disposition,
                    RefinementInvestigationDispositionStatus::ReplayRejected
                        | RefinementInvestigationDispositionStatus::Unselected
                )
            })
            .all(|candidate| candidate.acquisition_frontiers.is_empty())
    );

    let mut missing_mapping = request_for_subject(&source.subject);
    missing_mapping.mappings.mappings.retain(|mapping| {
        !(mapping.episode_id == OXC_EPISODE
            && mapping.candidate_id == SELECTED_OXC_CANDIDATE
            && mapping.discriminator_id == "edit_class")
    });
    let selected = selected_disposition(&missing_mapping);
    assert_eq!(
        selected.disposition,
        RefinementInvestigationDispositionStatus::RequirementMappingNeeded
    );
    assert!(selected.acquisition_frontiers.is_empty());
}

#[test]
fn mapped_unknown_and_invalid_selected_frontiers_both_reach_existing_planner() {
    let (_repo, _revision, source) = local_source();

    let mut unknown = request_for_subject(&source.subject);
    let mut unknown_observation = source.observation.clone();
    unknown_observation.value_state = DiscriminatorValueState::Unknown {
        reason_ref: "research:refinement-demand-plan:unknown".to_string(),
    };
    unknown.observations.observations.push(unknown_observation);
    let unknown_demand = selected_disposition(&unknown);
    assert_eq!(
        unknown_demand.disposition,
        RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded
    );
    assert_eq!(
        unknown_demand.acquisition_frontiers[0].status,
        ObservationFrontierStatus::Unknown
    );
    let unknown_plan = plan_frontier(&source, unknown_demand.acquisition_frontiers[0].clone());
    assert_eq!(unknown_plan.status, ObservationProbePlanStatus::Planned);
    assert_eq!(
        unknown_plan.evidence_plan.unwrap().status,
        EvidencePlanStatus::Selected
    );

    let mut invalid = request_for_subject(&source.subject);
    let mut invalid_observation = source.observation.clone();
    invalid_observation.applicability.status = ObservationApplicabilityStatus::Invalid;
    invalid_observation.applicability.receipt_ref =
        "research:refinement-demand-plan:invalid".to_string();
    invalid.observations.observations.push(invalid_observation);
    let invalid_demand = selected_disposition(&invalid);
    assert_eq!(
        invalid_demand.disposition,
        RefinementInvestigationDispositionStatus::ObservationAcquisitionNeeded
    );
    assert_eq!(
        invalid_demand.acquisition_frontiers[0].status,
        ObservationFrontierStatus::Invalid
    );
    let invalid_plan = plan_frontier(&source, invalid_demand.acquisition_frontiers[0].clone());
    assert_eq!(invalid_plan.status, ObservationProbePlanStatus::Planned);
    assert_eq!(
        invalid_plan.evidence_plan.unwrap().status,
        EvidencePlanStatus::Selected
    );
}

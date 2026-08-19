#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[path = "../src/closure_episode.rs"]
mod closure_episode;
#[path = "../src/lesson_promotion.rs"]
mod lesson_promotion;
#[path = "../src/observation_reconciliation.rs"]
mod observation_reconciliation;
#[path = "../src/prior_episode_detail.rs"]
mod prior_episode_detail;
#[path = "../src/prior_episode_front.rs"]
mod prior_episode_front;
#[path = "../src/project_memory.rs"]
mod project_memory;
#[path = "../src/proof_surface.rs"]
mod proof_surface;
#[path = "../src/proxy_revision.rs"]
mod proxy_revision;
#[path = "../src/review_memory.rs"]
mod review_memory;

use behavioral_trial::{
    BEHAVIORAL_TRIAL_SCHEMA_VERSION, BehavioralTrialArmKind, BehavioralTrialObservation,
    BehavioralTrialPair, evaluate_behavioral_trial_pair, fingerprint_plan,
    materialize_worker_packet, parse_behavioral_trial_plan,
};
use lesson_promotion::parse_lesson_promotion_claim;
use prior_episode_detail::{PriorEpisodeDetail, project_prior_episode_detail};
use prior_episode_front::PriorEpisodeInput;
use project_memory::parse_project_memory_packet;

const PLAN: &[u8] =
    include_bytes!("../research/behavioral-trials/stensibly-index-guard-detail.json");
const PLAN_FINGERPRINT: &str = "cultist-behavioral-trial-plan-sha256-v1:6f3eddecf177271c0ad60f32fb17008841bdb81f34aa717f52be90c3bdd1f69b";
const CONTROL_PACKET_FINGERPRINT: &str = "cultist-behavioral-worker-packet-sha256-v1:9949665d13b162692ebd3f7d12b6f162881f18d2d381c7257100bbb89c317f01";
const TREATMENT_PACKET_FINGERPRINT: &str = "cultist-behavioral-worker-packet-sha256-v1:6d3d93c574e81800ef1829216356d258553cabc7d2bdce2d09c32f46659c738c";

fn selected_detail() -> PriorEpisodeDetail {
    let memory = parse_project_memory_packet(include_bytes!(
        "../research/project-memory/stensibly-1575.json"
    ))
    .unwrap();
    memory.summary().unwrap();
    let claim = parse_lesson_promotion_claim(include_bytes!(
        "../research/lesson-promotion/stensibly-1575.json"
    ))
    .unwrap();
    project_prior_episode_detail(&PriorEpisodeInput::LessonPromotion {
        id: "stensibly:index-limit-guard".to_string(),
        memory: Box::new(memory),
        claim: Box::new(claim),
    })
    .unwrap()
}

#[test]
fn plan_changes_only_worker_visible_selected_detail_after_the_compact_front() {
    let plan = parse_behavioral_trial_plan(PLAN).unwrap();
    assert_eq!(fingerprint_plan(&plan).unwrap(), PLAN_FINGERPRINT);

    let control = materialize_worker_packet(&plan, BehavioralTrialArmKind::Control).unwrap();
    let treatment = materialize_worker_packet(&plan, BehavioralTrialArmKind::Treatment).unwrap();
    assert_eq!(
        control.worker_packet_fingerprint,
        CONTROL_PACKET_FINGERPRINT
    );
    assert_eq!(
        treatment.worker_packet_fingerprint,
        TREATMENT_PACKET_FINGERPRINT
    );
    assert_eq!(control.task_instruction, treatment.task_instruction);
    assert_eq!(
        control.allowed_first_actions,
        treatment.allowed_first_actions
    );
    assert_eq!(control.context.as_bytes().len(), 1_082);
    assert_eq!(treatment.context.as_bytes().len(), 1_775);

    assert!(treatment.context.starts_with(&control.context));
    let suffix = treatment.context.strip_prefix(&control.context).unwrap();
    assert!(suffix.starts_with("\n\nCultist selected accepted-guard detail:\n"));
    assert!(!control.context.contains("64-character identifier limit"));
    assert!(treatment.context.contains("64-character identifier limit"));
    assert!(
        !control
            .context
            .contains("fails when any exceed 64 characters")
    );
    assert!(
        treatment
            .context
            .contains("fails when any exceed 64 characters")
    );
    assert!(!treatment.context.contains("max_identifier_length"));
    assert!(!treatment.context.contains("corrective_action"));

    for common in [
        "by_project_issue_revision_instruction_set_sha256_provider_updated_at",
        "next: use_accepted_guard",
        "candidate value: index_identifier_limit",
        "guard: pull_request#1575",
        "same-class repairs: pull_request#1571, pull_request#1573",
    ] {
        assert!(control.context.contains(common));
        assert!(treatment.context.contains(common));
    }

    let detail = selected_detail();
    let PriorEpisodeDetail::AcceptedGuard {
        operational_marker,
        guard_source_evidence,
        ..
    } = detail;
    assert!(treatment.context.contains(&operational_marker));
    assert!(treatment.context.contains(&guard_source_evidence));
    assert!(!control.context.contains(&operational_marker));
    assert!(!control.context.contains(&guard_source_evidence));

    let control_json = serde_json::to_string(&control).unwrap();
    let treatment_json = serde_json::to_string(&treatment).unwrap();
    assert!(!control_json.contains("#front"));
    assert!(!treatment_json.contains("#front-plus-selected-detail"));
}

#[test]
fn synthetic_reversed_pair_is_descriptive_only_and_maps_first_actions_to_their_arms() {
    let plan = parse_behavioral_trial_plan(PLAN).unwrap();
    let plan_fingerprint = fingerprint_plan(&plan).unwrap();
    let control = materialize_worker_packet(&plan, BehavioralTrialArmKind::Control).unwrap();
    let treatment = materialize_worker_packet(&plan, BehavioralTrialArmKind::Treatment).unwrap();

    let pair = BehavioralTrialPair {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        plan: Box::new(plan),
        observations: vec![
            BehavioralTrialObservation {
                schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
                trial_id: "prior-episode-front:stensibly-index-guard-detail".to_string(),
                plan_fingerprint: plan_fingerprint.clone(),
                worker_packet_fingerprint: treatment.worker_packet_fingerprint.clone(),
                worker_ref: "worker:synthetic-treatment".to_string(),
                first_action_id: "block_and_shorten_identifier".to_string(),
            },
            BehavioralTrialObservation {
                schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
                trial_id: "prior-episode-front:stensibly-index-guard-detail".to_string(),
                plan_fingerprint,
                worker_packet_fingerprint: control.worker_packet_fingerprint.clone(),
                worker_ref: "worker:synthetic-control".to_string(),
                first_action_id: "inspect_accepted_guard_detail".to_string(),
            },
        ],
    };

    let evaluation = evaluate_behavioral_trial_pair(&pair).unwrap();
    assert_eq!(
        evaluation.control.first_action_id,
        "inspect_accepted_guard_detail"
    );
    assert_eq!(
        evaluation.treatment.first_action_id,
        "block_and_shorten_identifier"
    );
    assert!(!evaluation.same_first_action);
}

#[test]
fn one_byte_treatment_context_drift_rejects_against_the_registered_digest() {
    let mut plan = parse_behavioral_trial_plan(PLAN).unwrap();
    plan.treatment.context.push(' ');
    let error = fingerprint_plan(&plan).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("treatment.context_digest does not match the exact context bytes")
    );
}

#![allow(dead_code)]

#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;

use behavioral_trial::{
    BEHAVIORAL_TRIAL_SCHEMA_VERSION, BehavioralTrialAction, BehavioralTrialArm,
    BehavioralTrialArmKind, BehavioralTrialObservation, BehavioralTrialPair, BehavioralTrialPlan,
    CONTEXT_DIGEST_SCHEME, PLAN_FINGERPRINT_SCHEME, WORKER_PACKET_FINGERPRINT_SCHEME,
    context_digest, evaluate_behavioral_trial_pair, fingerprint_plan, materialize_worker_packet,
    parse_behavioral_trial_plan,
};

const EXPECTED_CONTROL_DIGEST: &str = "cultist-behavioral-context-sha256-v1:10944d9f4e3e8d41e3e2aa0d751214c0b413f7090a5a5e459bcaf1ce9be0fe5c";
const EXPECTED_TREATMENT_DIGEST: &str = "cultist-behavioral-context-sha256-v1:49b365442076db3f9d1fddefabf0feadc6c7e1e66b07f14dbe74dc310f275584";
const EXPECTED_PLAN_FINGERPRINT: &str = "cultist-behavioral-trial-plan-sha256-v1:ef917cedce917584a050a410c92c7102a53ea62ee4a976337e22d8723bded300";
const EXPECTED_CONTROL_PACKET: &str = "cultist-behavioral-worker-packet-sha256-v1:de7d024d8d6798059a99cf21d563a7a32948a1c0eca5c396dae1899adb559400";
const EXPECTED_TREATMENT_PACKET: &str = "cultist-behavioral-worker-packet-sha256-v1:587d983db0d0af54a4fee378bba967c1723a295ce2fc101cb7c7181023cec923";

fn plan() -> BehavioralTrialPlan {
    BehavioralTrialPlan {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        trial_id: "trial:fixture".to_string(),
        task_instruction: "Choose the first justified action.".to_string(),
        allowed_first_actions: vec![
            BehavioralTrialAction {
                id: "inspect_history".to_string(),
                label: "Inspect prior history".to_string(),
            },
            BehavioralTrialAction {
                id: "continue_current".to_string(),
                label: "Continue current work".to_string(),
            },
        ],
        control: BehavioralTrialArm {
            context_ref: "fixture:control".to_string(),
            context: "No prior episode is surfaced.".to_string(),
            context_digest: EXPECTED_CONTROL_DIGEST.to_string(),
        },
        treatment: BehavioralTrialArm {
            context_ref: "fixture:treatment".to_string(),
            context: "Prior episode: stale review; recompute before reuse.".to_string(),
            context_digest: EXPECTED_TREATMENT_DIGEST.to_string(),
        },
    }
}

fn observation(
    packet_fingerprint: &str,
    worker_ref: &str,
    action: &str,
) -> BehavioralTrialObservation {
    BehavioralTrialObservation {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        trial_id: "trial:fixture".to_string(),
        plan_fingerprint: EXPECTED_PLAN_FINGERPRINT.to_string(),
        worker_packet_fingerprint: packet_fingerprint.to_string(),
        worker_ref: worker_ref.to_string(),
        first_action_id: action.to_string(),
    }
}

fn pair(control_action: &str, treatment_action: &str) -> BehavioralTrialPair {
    BehavioralTrialPair {
        schema_version: BEHAVIORAL_TRIAL_SCHEMA_VERSION,
        plan: Box::new(plan()),
        observations: vec![
            observation(EXPECTED_CONTROL_PACKET, "worker:control", control_action),
            observation(
                EXPECTED_TREATMENT_PACKET,
                "worker:treatment",
                treatment_action,
            ),
        ],
    }
}

#[test]
fn exact_small_plan_has_stable_known_digests_and_fingerprints() {
    assert_eq!(
        context_digest("No prior episode is surfaced."),
        EXPECTED_CONTROL_DIGEST
    );
    assert_eq!(
        context_digest("Prior episode: stale review; recompute before reuse."),
        EXPECTED_TREATMENT_DIGEST
    );
    assert_eq!(fingerprint_plan(&plan()).unwrap(), EXPECTED_PLAN_FINGERPRINT);

    let control = materialize_worker_packet(&plan(), BehavioralTrialArmKind::Control).unwrap();
    let treatment =
        materialize_worker_packet(&plan(), BehavioralTrialArmKind::Treatment).unwrap();
    assert_eq!(control.worker_packet_fingerprint, EXPECTED_CONTROL_PACKET);
    assert_eq!(
        treatment.worker_packet_fingerprint,
        EXPECTED_TREATMENT_PACKET
    );
}

#[test]
fn typed_plan_fingerprint_ignores_json_field_order_and_formatting() {
    let compact = serde_json::to_string(&plan()).unwrap();
    let reordered = format!(
        r#"{{
          "treatment": {},
          "allowed_first_actions": {},
          "trial_id": "trial:fixture",
          "schema_version": 1,
          "control": {},
          "task_instruction": "Choose the first justified action."
        }}"#,
        serde_json::to_string(&plan().treatment).unwrap(),
        serde_json::to_string(&plan().allowed_first_actions).unwrap(),
        serde_json::to_string(&plan().control).unwrap(),
    );

    let compact_plan = parse_behavioral_trial_plan(compact.as_bytes()).unwrap();
    let reordered_plan = parse_behavioral_trial_plan(reordered.as_bytes()).unwrap();
    assert_eq!(compact_plan, reordered_plan);
    assert_eq!(
        fingerprint_plan(&compact_plan).unwrap(),
        fingerprint_plan(&reordered_plan).unwrap()
    );
}

#[test]
fn one_byte_context_mutation_changes_arm_and_plan_fingerprints() {
    let original = plan();
    let mut changed = original.clone();
    changed.treatment.context.push('!');
    changed.treatment.context_digest = context_digest(&changed.treatment.context);

    assert_ne!(
        fingerprint_plan(&original).unwrap(),
        fingerprint_plan(&changed).unwrap()
    );
    assert_ne!(
        materialize_worker_packet(&original, BehavioralTrialArmKind::Treatment)
            .unwrap()
            .worker_packet_fingerprint,
        materialize_worker_packet(&changed, BehavioralTrialArmKind::Treatment)
            .unwrap()
            .worker_packet_fingerprint
    );
}

#[test]
fn worker_packets_hide_arm_labels_and_context_refs() {
    for arm in [BehavioralTrialArmKind::Control, BehavioralTrialArmKind::Treatment] {
        let packet = materialize_worker_packet(&plan(), arm).unwrap();
        let json = serde_json::to_string(&packet).unwrap();
        assert!(!json.contains("context_ref"));
        assert!(!json.contains("\"control\""));
        assert!(!json.contains("\"treatment\""));
        assert!(json.contains(CONTEXT_DIGEST_SCHEME));
        assert!(json.contains(PLAN_FINGERPRINT_SCHEME));
        assert!(json.contains(WORKER_PACKET_FINGERPRINT_SCHEME));
    }
}

#[test]
fn same_first_action_is_descriptive_only() {
    let evaluation = evaluate_behavioral_trial_pair(&pair("inspect_history", "inspect_history"))
        .unwrap();
    assert!(evaluation.same_first_action);
    assert_eq!(evaluation.control.first_action_id, "inspect_history");
    assert_eq!(evaluation.treatment.first_action_id, "inspect_history");
}

#[test]
fn different_first_action_is_descriptive_only() {
    let evaluation =
        evaluate_behavioral_trial_pair(&pair("continue_current", "inspect_history")).unwrap();
    assert!(!evaluation.same_first_action);
    assert_eq!(evaluation.control.first_action_id, "continue_current");
    assert_eq!(evaluation.treatment.first_action_id, "inspect_history");
}

#[test]
fn observations_can_arrive_in_either_order() {
    let mut input = pair("continue_current", "inspect_history");
    input.observations.reverse();
    let evaluation = evaluate_behavioral_trial_pair(&input).unwrap();
    assert_eq!(evaluation.control.worker_ref, "worker:control");
    assert_eq!(evaluation.treatment.worker_ref, "worker:treatment");
}

#[test]
fn observation_action_must_be_registered() {
    let error = evaluate_behavioral_trial_pair(&pair("continue_current", "invented_action"))
        .unwrap_err();
    assert!(error.to_string().contains("registered action vocabulary"));
}

#[test]
fn two_observations_for_same_arm_reject() {
    let mut input = pair("continue_current", "inspect_history");
    input.observations[1].worker_packet_fingerprint = EXPECTED_CONTROL_PACKET.to_string();
    let error = evaluate_behavioral_trial_pair(&input).unwrap_err();
    assert!(error.to_string().contains("same arm"));
}

#[test]
fn unknown_worker_packet_fingerprint_rejects() {
    let mut input = pair("continue_current", "inspect_history");
    input.observations[1].worker_packet_fingerprint = format!(
        "{WORKER_PACKET_FINGERPRINT_SCHEME}:{}",
        "0".repeat(64)
    );
    let error = evaluate_behavioral_trial_pair(&input).unwrap_err();
    assert!(error.to_string().contains("unknown worker-packet fingerprint"));
}

#[test]
fn plan_mutation_after_observation_rejects() {
    let mut input = pair("continue_current", "inspect_history");
    input.plan.task_instruction.push_str(" Changed after registration.");
    let error = evaluate_behavioral_trial_pair(&input).unwrap_err();
    assert!(error.to_string().contains("plan_fingerprint"));
}

#[test]
fn identical_control_and_treatment_contexts_reject() {
    let mut input = plan();
    input.treatment.context = input.control.context.clone();
    input.treatment.context_digest = input.control.context_digest.clone();
    let error = fingerprint_plan(&input).unwrap_err();
    assert!(error.to_string().contains("different exact digests"));
}

#[test]
fn supplied_context_digest_must_match_exact_context() {
    let mut input = plan();
    input.control.context.push('!');
    let error = fingerprint_plan(&input).unwrap_err();
    assert!(error.to_string().contains("does not match the exact context bytes"));
}

#[test]
fn duplicate_action_ids_reject() {
    let mut input = plan();
    input.allowed_first_actions[1].id = "inspect_history".to_string();
    let error = fingerprint_plan(&input).unwrap_err();
    assert!(error.to_string().contains("duplicate behavioral-trial action id"));
}

#[test]
fn plan_parser_rejects_unknown_fields() {
    let mut value = serde_json::to_value(plan()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("score".to_string(), serde_json::json!(0.99));
    let bytes = serde_json::to_vec(&value).unwrap();
    let error = parse_behavioral_trial_plan(&bytes).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

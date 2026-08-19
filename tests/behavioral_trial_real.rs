#![allow(dead_code)]

#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;

use behavioral_trial::{
    BehavioralTrialArmKind, fingerprint_plan, materialize_worker_packet,
    parse_behavioral_trial_plan,
};

#[test]
fn stale_review_trial_has_stable_registered_packets() {
    let plan = parse_behavioral_trial_plan(include_bytes!(
        "../research/behavioral-trials/prior-review-stale.json"
    ))
    .unwrap();

    assert_eq!(
        fingerprint_plan(&plan).unwrap(),
        "cultist-behavioral-trial-plan-sha256-v1:b394eefd406bac16e2ba7690bd45f6373d3b029208fdd55d70c3b9a943f15d65"
    );

    let control = materialize_worker_packet(&plan, BehavioralTrialArmKind::Control).unwrap();
    let treatment = materialize_worker_packet(&plan, BehavioralTrialArmKind::Treatment).unwrap();

    assert_eq!(
        control.worker_packet_fingerprint,
        "cultist-behavioral-worker-packet-sha256-v1:4a59e3aac7200f97bf059d3df4fa3ba81c322d2ecaaac18ade25724ca6799272"
    );
    assert_eq!(
        treatment.worker_packet_fingerprint,
        "cultist-behavioral-worker-packet-sha256-v1:3f9a24e968a1b2cfd2284e6668054537e46eaeca0ba5fdb69c116d4a7b3f8e9d"
    );
    assert!(treatment.context.starts_with(&control.context));
    assert!(!control.context.contains("Cultist prior-episode front:"));
    assert!(
        treatment
            .context
            .contains("old outcome applicability: INVALID")
    );
    assert!(
        treatment
            .context
            .contains("next: recompute_and_refresh_review_thread")
    );
}

#[test]
fn closed_rereport_trial_has_stable_registered_packets() {
    let plan = parse_behavioral_trial_plan(include_bytes!(
        "../research/behavioral-trials/closed-rereport.json"
    ))
    .unwrap();

    assert_eq!(
        fingerprint_plan(&plan).unwrap(),
        "cultist-behavioral-trial-plan-sha256-v1:8d42a49630eace01d6c14055a79d41a91dcb22f84f32db2a92e483ec60840ba4"
    );

    let control = materialize_worker_packet(&plan, BehavioralTrialArmKind::Control).unwrap();
    let treatment = materialize_worker_packet(&plan, BehavioralTrialArmKind::Treatment).unwrap();

    assert_eq!(
        control.worker_packet_fingerprint,
        "cultist-behavioral-worker-packet-sha256-v1:73ce3562f8fc382db9ab3505236fdf965c5d6b4f6a5c6cd745ddc2b121c8f7e6"
    );
    assert_eq!(
        treatment.worker_packet_fingerprint,
        "cultist-behavioral-worker-packet-sha256-v1:6c63c84bcf52831cb0d4bc3490bf3a279bf77cacd67ede8d300fb6c09ca7d4e8"
    );
    assert!(treatment.context.starts_with(&control.context));
    assert!(!control.context.contains("Cultist prior-episode front:"));
    assert!(treatment.context.contains("clearance: UNKNOWN"));
    assert!(
        treatment
            .context
            .contains("next: inspect_prior_failure_and_rereport")
    );
}

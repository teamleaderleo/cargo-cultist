#[path = "../src/observation_reconciliation.rs"]
mod observation_reconciliation;
#[path = "../src/project_memory.rs"]
mod project_memory;

use observation_reconciliation::{
    MAX_OBSERVATION_RECONCILIATION_BYTES, ObservationReconciliationStatus,
    PersistentDisagreementDisposition, TemporaryDisagreementDisposition,
    evaluate_observation_reconciliation, parse_observation_reconciliation_claim,
};
use project_memory::{ArtifactKind, ArtifactRef, parse_project_memory_packet};

const MEMORY: &[u8] = include_bytes!("../research/project-memory/stensibly-1609-1610.json");
const CLAIM: &[u8] =
    include_bytes!("../research/observation-reconciliation/stensibly-1609-1610.json");

fn inputs() -> (
    project_memory::ProjectMemoryPacket,
    observation_reconciliation::ObservationReconciliationClaim,
) {
    let memory = parse_project_memory_packet(MEMORY).unwrap();
    memory.summary().unwrap();
    let claim = parse_observation_reconciliation_claim(CLAIM).unwrap();
    (memory, claim)
}

fn pr(number: u64) -> ArtifactRef {
    ArtifactRef {
        kind: ArtifactKind::PullRequest,
        number,
    }
}

#[test]
fn retained_stensibly_episode_is_observed_reconciliation() {
    let (memory, claim) = inputs();
    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();

    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::ObservedReconciliation
    );
    assert_eq!(evaluation.predecessor, pr(1609));
    assert_eq!(evaluation.reconciler, pr(1610));
    assert_eq!(
        evaluation.authoritative_source_ref,
        "cloudflare_provider_current"
    );
    assert_eq!(evaluation.lagging_source_ref, "workers_dev_public_origin");
    assert_eq!(
        evaluation.temporary_disagreement,
        TemporaryDisagreementDisposition::BoundedConvergence
    );
    assert_eq!(
        evaluation.persistent_disagreement,
        PersistentDisagreementDisposition::HardFailure
    );
    assert!(!evaluation.automatic_authority_change);
}

#[test]
fn predecessor_must_be_merged() {
    let (mut memory, claim) = inputs();
    memory
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == pr(1609))
        .unwrap()
        .revision
        .as_mut()
        .unwrap()
        .merged = false;

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::PredecessorUnmerged
    );
}

#[test]
fn reconciler_must_be_merged() {
    let (mut memory, claim) = inputs();
    memory
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == pr(1610))
        .unwrap()
        .revision
        .as_mut()
        .unwrap()
        .merged = false;

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::ReconcilerUnmerged
    );
}

#[test]
fn reconciler_must_explicitly_name_predecessor() {
    let (memory, mut claim) = inputs();
    claim.reconciler_predecessor_evidence =
        "Deployment observation/receipt only. No release authority change, no rollback change, no Worker product behavior, no API/UI change, no provider mutation beyond the existing deployment workflow.".to_string();

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::ReconcilerDoesNotNamePredecessor
    );
}

#[test]
fn authority_rule_must_be_explicit() {
    let (memory, mut claim) = inputs();
    claim.authority_marker = "provider authority marker absent".to_string();

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::AuthorityRuleMissing
    );
}

#[test]
fn both_divergent_values_must_be_observed() {
    let (memory, mut claim) = inputs();
    claim.lagging_value_marker = "00000000-0000-4000-8000-000000000000".to_string();

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::DivergentObservationMissing
    );
}

#[test]
fn bounded_convergence_policy_must_be_explicit() {
    let (memory, mut claim) = inputs();
    claim.convergence_marker = "retry forever".to_string();

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::ConvergencePolicyMissing
    );
}

#[test]
fn permanent_divergence_must_remain_a_failure() {
    let (memory, mut claim) = inputs();
    claim.exhaustion_marker = "accept permanent divergence".to_string();

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::PermanentDivergenceControlMissing
    );
}

#[test]
fn reconciler_must_change_the_implementation_path() {
    let (mut memory, claim) = inputs();
    memory
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == pr(1610))
        .unwrap()
        .changed_paths
        .retain(|path| path != "scripts/worker-production-receipt.ts");

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::ImplementationPathMissing
    );
}

#[test]
fn reconciler_must_change_the_test_path() {
    let (mut memory, claim) = inputs();
    memory
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == pr(1610))
        .unwrap()
        .changed_paths
        .retain(|path| path != "test/worker-production-receipt.test.ts");

    let evaluation = evaluate_observation_reconciliation(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ObservationReconciliationStatus::TestPathMissing
    );
}

#[test]
fn authority_and_lagging_sources_must_differ() {
    let (memory, mut claim) = inputs();
    claim.lagging_source_ref = claim.authoritative_source_ref.clone();

    let error = evaluate_observation_reconciliation(&memory, &claim).unwrap_err();
    assert!(error.contains("observation sources must differ"));
}

#[test]
fn authoritative_and_lagging_values_must_differ() {
    let (memory, mut claim) = inputs();
    claim.lagging_value_ref = claim.authoritative_value_ref.clone();

    let error = evaluate_observation_reconciliation(&memory, &claim).unwrap_err();
    assert!(error.contains("observation values must differ"));
}

#[test]
fn invented_reconciler_evidence_is_rejected() {
    let (memory, mut claim) = inputs();
    claim.authority_evidence = "Provider current wins because this says so.".to_string();

    let error = evaluate_observation_reconciliation(&memory, &claim).unwrap_err();
    assert!(error.contains("absent from retained project-memory text"));
}

#[test]
fn claim_input_is_bounded_before_json_parse() {
    let bytes = vec![b' '; MAX_OBSERVATION_RECONCILIATION_BYTES + 1];
    let error = parse_observation_reconciliation_claim(&bytes).unwrap_err();
    assert!(error.contains("exceeds"));
}

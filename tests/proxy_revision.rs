#[path = "../src/proxy_revision.rs"]
mod proxy_revision;
#[path = "../src/project_memory.rs"]
mod project_memory;

use project_memory::{ArtifactKind, ArtifactRef, parse_project_memory_packet};
use proxy_revision::{
    MAX_PROXY_REVISION_BYTES, ProxyRevisionStatus, evaluate_proxy_revision,
    parse_proxy_revision_claim,
};

const MEMORY: &[u8] = include_bytes!("../research/project-memory/stensibly-1604-1605.json");
const CLAIM: &[u8] = include_bytes!("../research/proxy-revision/stensibly-1604-1605.json");

fn inputs() -> (
    project_memory::ProjectMemoryPacket,
    proxy_revision::ProxyRevisionClaim,
) {
    let memory = parse_project_memory_packet(MEMORY).unwrap();
    memory.summary().unwrap();
    let claim = parse_proxy_revision_claim(CLAIM).unwrap();
    (memory, claim)
}

fn pr(number: u64) -> ArtifactRef {
    ArtifactRef {
        kind: ArtifactKind::PullRequest,
        number,
    }
}

#[test]
fn retained_stensibly_episode_is_an_observed_proxy_revision() {
    let (memory, claim) = inputs();
    let evaluation = evaluate_proxy_revision(&memory, &claim).unwrap();

    assert_eq!(evaluation.status, ProxyRevisionStatus::ObservedProxyRevision);
    assert_eq!(evaluation.predecessor, pr(1604));
    assert_eq!(evaluation.successor, pr(1605));
    assert_eq!(evaluation.shared_path, "convex/items.ts");
    assert_eq!(evaluation.prior_value_ref, "positive_expected_generation");
    assert_eq!(
        evaluation.replacement_value_ref,
        "live_unexpired_claim_for_actor"
    );
    assert!(!evaluation.automatic_generalization_authority);
}

#[test]
fn unmerged_predecessor_is_not_an_observed_revision() {
    let (mut memory, claim) = inputs();
    memory
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == pr(1604))
        .unwrap()
        .revision
        .as_mut()
        .unwrap()
        .merged = false;

    let evaluation = evaluate_proxy_revision(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, ProxyRevisionStatus::PredecessorUnmerged);
}

#[test]
fn unmerged_successor_is_not_an_observed_revision() {
    let (mut memory, claim) = inputs();
    memory
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == pr(1605))
        .unwrap()
        .revision
        .as_mut()
        .unwrap()
        .merged = false;

    let evaluation = evaluate_proxy_revision(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, ProxyRevisionStatus::SuccessorUnmerged);
}

#[test]
fn successor_must_explicitly_name_the_predecessor() {
    let (memory, mut claim) = inputs();
    claim.successor_counterexample_evidence =
        "Finish the ordinary item-state producer lane for #1149 by projecting block/unblock transitions into the same durable automatic activity store, while correcting one subtle responsibility-generation assumption exposed by this continuation.".to_string();

    let evaluation = evaluate_proxy_revision(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ProxyRevisionStatus::SuccessorDoesNotNamePredecessor
    );
}

#[test]
fn both_artifacts_must_touch_the_shared_implementation_path() {
    let (mut memory, claim) = inputs();
    memory
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == pr(1605))
        .unwrap()
        .changed_paths
        .retain(|path| path != "convex/items.ts");

    let evaluation = evaluate_proxy_revision(&memory, &claim).unwrap();
    assert_eq!(
        evaluation.status,
        ProxyRevisionStatus::NoSharedImplementationPath
    );
}

#[test]
fn predecessor_must_state_the_proxy_rule() {
    let (memory, mut claim) = inputs();
    claim.proxy_rule_marker = "claim generation always proves responsibility".to_string();

    let evaluation = evaluate_proxy_revision(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, ProxyRevisionStatus::PriorProxyRuleMissing);
}

#[test]
fn successor_must_state_the_counterexample() {
    let (memory, mut claim) = inputs();
    claim.counterexample_marker = "counterexample missing from retained source".to_string();

    let evaluation = evaluate_proxy_revision(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, ProxyRevisionStatus::CounterexampleMissing);
}

#[test]
fn successor_must_state_the_replacement_rule() {
    let (memory, mut claim) = inputs();
    claim.replacement_rule_marker = "replacement rule missing from retained source".to_string();

    let evaluation = evaluate_proxy_revision(&memory, &claim).unwrap();
    assert_eq!(evaluation.status, ProxyRevisionStatus::ReplacementRuleMissing);
}

#[test]
fn prior_and_replacement_values_must_differ() {
    let (memory, mut claim) = inputs();
    claim.replacement_value_ref = claim.prior_value_ref.clone();

    let error = evaluate_proxy_revision(&memory, &claim).unwrap_err();
    assert!(error.contains("prior and replacement values must differ"));
}

#[test]
fn invented_predecessor_source_excerpt_is_rejected() {
    let (memory, mut claim) = inputs();
    claim.predecessor_source_evidence =
        "This historical rule definitely meant something stronger.".to_string();

    let error = evaluate_proxy_revision(&memory, &claim).unwrap_err();
    assert!(error.contains("absent from retained project-memory text"));
}

#[test]
fn claim_input_is_bounded_before_json_parse() {
    let bytes = vec![b' '; MAX_PROXY_REVISION_BYTES + 1];
    let error = parse_proxy_revision_claim(&bytes).unwrap_err();
    assert!(error.contains("exceeds"));
}

#[allow(dead_code)]
#[path = "../examples/support/capability_demand_pair_execution_impl.rs"]
mod capability_demand_pair_execution;

use std::collections::BTreeMap;

use capability_demand_pair_execution::capability_demand_retirement::{
    EvidenceInspection, RetirementVerdict, RunOutcome, evaluate_pair, parse_run_receipt,
    parse_trial_manifest, parse_trial_spec,
};
use capability_demand_pair_execution::{
    ExecutionOrigin, ExternalRunMetadata, OrderSelection, PairOrder, artifact_digest,
    build_external_run_receipt, build_synthetic_test_run_receipt, parse_order_selection,
    prepare_pair, sha256_hex,
};
use serde_json::{Value, json};

const REVISION: &str = "1111111111111111111111111111111111111111";
const TARGET_BLOB: &str = "2222222222222222222222222222222222222222";
const DECISIVE_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DECISIVE_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Fixture {
    trial: Vec<u8>,
    manifest: Vec<u8>,
    task: Vec<u8>,
    patch: Vec<u8>,
    baseline: Vec<u8>,
    treatment: Vec<u8>,
}

fn fixture() -> Fixture {
    let task = b"Review the proposed patch for a blocking repository constraint.\n".to_vec();
    let patch = b"@@ synthetic patch @@\n+new_repository_behavior()\n".to_vec();
    let baseline = b"{\"analysis\":\"fixture\",\"history\":[\"local-only\"]}\n".to_vec();
    let treatment =
        b"{\"analysis\":\"fixture\",\"history\":[\"decisive-a\",\"decisive-b\"]}\n".to_vec();

    let trial = serde_json::to_vec_pretty(&json!({
        "schema_version": 1,
        "trial_id": "fixture-capability-demand",
        "repository": "owner/repo",
        "revision": REVISION,
        "target_path": "src/lib.rs",
        "target_blob_sha": TARGET_BLOB,
        "worker_task": {
            "prompt": "Review the proposed patch for a blocking repository constraint.",
            "patch": "@@ synthetic patch @@\n+new_repository_behavior()"
        },
        "oracle": {
            "expected_disposition": "block",
            "blocking_reason": "fixture_constraint",
            "max_identifier_length": 64,
            "proposed_identifier": "fixture_identifier",
            "proposed_identifier_length": 18,
            "corrective_action": "repair_fixture"
        },
        "conditions": [
            {
                "id": "local_packet",
                "packet_kind": "file_local",
                "budget_bytes": 32768,
                "scope": null,
                "decisive_evidence_present": false,
                "decisive_evidence_refs": []
            },
            {
                "id": "expanded_packet",
                "packet_kind": "scoped",
                "budget_bytes": 32768,
                "scope": "src",
                "decisive_evidence_present": true,
                "decisive_evidence_refs": [DECISIVE_A, DECISIVE_B]
            }
        ],
        "oracle_leak_control": {
            "historical_issue": "fixture:oracle-control",
            "allowed_as_worker_prompt": false,
            "prohibited_worker_prompt_fragments": ["fixture_constraint"]
        }
    }))
    .unwrap();

    let oracle = fixture_oracle_bytes();
    let manifest = serde_json::to_vec_pretty(&json!({
        "schema_version": 1,
        "trial_spec_sha256": sha256_hex(&trial),
        "trial_id": "fixture-capability-demand",
        "repository": "owner/repo",
        "revision": REVISION,
        "target_path": "src/lib.rs",
        "target_blob_sha": TARGET_BLOB,
        "worker_visible_common": {
            "task": digest_value(&task),
            "patch": digest_value(&patch)
        },
        "evaluator_only": {
            "oracle": digest_value(&oracle)
        },
        "conditions": {
            "local_packet": {
                "packet": digest_value(&baseline),
                "packet_kind": "file_local",
                "budget_bytes": 32768,
                "scope": null,
                "decisive_evidence_present": false
            },
            "expanded_packet": {
                "packet": digest_value(&treatment),
                "packet_kind": "scoped",
                "budget_bytes": 32768,
                "scope": "src",
                "decisive_evidence_present": true,
                "decisive_evidence_refs": [DECISIVE_A, DECISIVE_B]
            }
        }
    }))
    .unwrap();

    Fixture {
        trial,
        manifest,
        task,
        patch,
        baseline,
        treatment,
    }
}

fn fixture_oracle_bytes() -> Vec<u8> {
    let values = BTreeMap::from([
        (
            "blocking_reason",
            Value::String("fixture_constraint".to_string()),
        ),
        (
            "corrective_action",
            Value::String("repair_fixture".to_string()),
        ),
        ("expected_disposition", Value::String("block".to_string())),
        ("max_identifier_length", Value::from(64)),
        (
            "proposed_identifier",
            Value::String("fixture_identifier".to_string()),
        ),
        ("proposed_identifier_length", Value::from(18)),
    ]);
    let mut bytes = serde_json::to_vec_pretty(&values).unwrap();
    bytes.push(b'\n');
    bytes
}

fn digest_value(bytes: &[u8]) -> Value {
    let digest = artifact_digest(bytes);
    json!({"sha256": digest.sha256, "bytes": digest.bytes})
}

fn metadata(
    slot_id: &str,
    session_id: &str,
    outcome: RunOutcome,
    origin: ExecutionOrigin,
) -> ExternalRunMetadata {
    ExternalRunMetadata {
        schema_version: 1,
        slot_id: slot_id.to_string(),
        execution_origin: origin,
        worker_identity: "worker-family@v1".into(),
        harness_identity: "harness@v1".into(),
        affordance_identity: "read-only-tools@v1".into(),
        session_id: session_id.into(),
        fresh_session: true,
        prior_condition_exposure: false,
        evaluated_outcome: outcome,
        evidence_inspection: EvidenceInspection::Consulted,
        context_expanded: false,
    }
}

fn receipts_for_signal(
    order: OrderSelection,
) -> (
    Fixture,
    capability_demand_pair_execution::PreparedPair,
    Value,
    Value,
) {
    let fixture = fixture();
    let prepared = prepare_pair(
        &fixture.trial,
        &fixture.manifest,
        &fixture.task,
        &fixture.patch,
        &fixture.baseline,
        &fixture.treatment,
        "pair-fixture-1",
        order,
    )
    .unwrap();

    let sampling = b"temperature=0;seed=fixture";
    let reset_one = b"clean checkout before slot one";
    let reset_two = b"clean checkout before slot two";

    let mut receipts = Vec::new();
    for slot in &prepared.plan.slots {
        let condition = slot.condition_id.as_str();
        let outcome = if condition == "local_packet" {
            RunOutcome::Failed
        } else {
            RunOutcome::Success
        };
        let session = if slot.sequence_index == 1 {
            "session-one"
        } else {
            "session-two"
        };
        let reset = if slot.sequence_index == 1 {
            reset_one.as_slice()
        } else {
            reset_two.as_slice()
        };
        let output = if condition == "local_packet" {
            b"No blocking issue identified.".as_slice()
        } else {
            b"Blocking repository constraint identified.".as_slice()
        };
        let receipt = build_synthetic_test_run_receipt(
            &prepared.plan,
            &metadata(
                &slot.slot_id,
                session,
                outcome,
                ExecutionOrigin::SyntheticTest,
            ),
            sampling,
            reset,
            output,
        )
        .unwrap();
        receipts.push(receipt);
    }

    (fixture, prepared, receipts.remove(0), receipts.remove(0))
}

#[test]
fn worker_bundle_metadata_is_blind_while_organizer_plan_keeps_condition_mapping() {
    let fixture = fixture();
    let prepared = prepare_pair(
        &fixture.trial,
        &fixture.manifest,
        &fixture.task,
        &fixture.patch,
        &fixture.baseline,
        &fixture.treatment,
        "pair-blind",
        OrderSelection::Explicit(PairOrder::Ab),
    )
    .unwrap();

    assert_eq!(prepared.plan.slots[0].condition_id, "local_packet");
    assert_eq!(prepared.plan.slots[1].condition_id, "expanded_packet");

    for slot in &prepared.slots {
        let rendered = serde_json::to_string(&slot.metadata).unwrap();
        for forbidden in [
            "local_packet",
            "expanded_packet",
            "baseline",
            "treatment",
            "decisive",
            "oracle",
        ] {
            assert!(
                !rendered.contains(forbidden),
                "leaked {forbidden}: {rendered}"
            );
        }
        assert_eq!(slot.task, fixture.task);
        assert_eq!(slot.patch, fixture.patch);
    }
}

#[test]
fn explicit_and_seeded_order_are_stable_and_do_not_change_condition_semantics() {
    let fixture = fixture();
    let ab = prepare_pair(
        &fixture.trial,
        &fixture.manifest,
        &fixture.task,
        &fixture.patch,
        &fixture.baseline,
        &fixture.treatment,
        "pair-order",
        parse_order_selection("AB").unwrap(),
    )
    .unwrap();
    let ba = prepare_pair(
        &fixture.trial,
        &fixture.manifest,
        &fixture.task,
        &fixture.patch,
        &fixture.baseline,
        &fixture.treatment,
        "pair-order",
        parse_order_selection("BA").unwrap(),
    )
    .unwrap();
    assert_eq!(ab.plan.order, PairOrder::Ab);
    assert_eq!(ba.plan.order, PairOrder::Ba);
    assert_eq!(ab.plan.slots[0].condition_id, ba.plan.slots[1].condition_id);
    assert_eq!(ab.plan.slots[1].condition_id, ba.plan.slots[0].condition_id);

    let seeded_one = prepare_pair(
        &fixture.trial,
        &fixture.manifest,
        &fixture.task,
        &fixture.patch,
        &fixture.baseline,
        &fixture.treatment,
        "pair-seeded",
        parse_order_selection("seed:organizer-secret-1").unwrap(),
    )
    .unwrap();
    let seeded_two = prepare_pair(
        &fixture.trial,
        &fixture.manifest,
        &fixture.task,
        &fixture.patch,
        &fixture.baseline,
        &fixture.treatment,
        "pair-seeded",
        parse_order_selection("seed:organizer-secret-1").unwrap(),
    )
    .unwrap();
    assert_eq!(seeded_one.plan.order, seeded_two.plan.order);
    assert_eq!(
        serde_json::to_value(&seeded_one.plan.order_source).unwrap(),
        serde_json::to_value(&seeded_two.plan.order_source).unwrap()
    );
}

#[test]
fn exact_artifact_fingerprints_are_required_before_blind_packaging() {
    let fixture = fixture();
    let mut wrong_task = fixture.task.clone();
    wrong_task.push(b'!');
    assert!(
        prepare_pair(
            &fixture.trial,
            &fixture.manifest,
            &wrong_task,
            &fixture.patch,
            &fixture.baseline,
            &fixture.treatment,
            "pair-wrong-task",
            OrderSelection::Explicit(PairOrder::Ab),
        )
        .unwrap_err()
        .contains("task bytes do not match")
    );

    let mut wrong_packet = fixture.baseline.clone();
    wrong_packet.push(b' ');
    assert!(
        prepare_pair(
            &fixture.trial,
            &fixture.manifest,
            &fixture.task,
            &fixture.patch,
            &wrong_packet,
            &fixture.treatment,
            "pair-wrong-packet",
            OrderSelection::Explicit(PairOrder::Ab),
        )
        .unwrap_err()
        .contains("not admitted by the manifest")
    );
}

#[test]
fn production_receipt_builder_rejects_synthetic_execution_origin() {
    let fixture = fixture();
    let prepared = prepare_pair(
        &fixture.trial,
        &fixture.manifest,
        &fixture.task,
        &fixture.patch,
        &fixture.baseline,
        &fixture.treatment,
        "pair-origin",
        OrderSelection::Explicit(PairOrder::Ab),
    )
    .unwrap();
    let slot = &prepared.plan.slots[0];
    let error = build_external_run_receipt(
        &prepared.plan,
        &metadata(
            &slot.slot_id,
            "synthetic-session",
            RunOutcome::Failed,
            ExecutionOrigin::SyntheticTest,
        ),
        b"sampling",
        b"reset",
        b"raw output",
    )
    .unwrap_err();
    assert!(error.contains("synthetic test metadata"));
}

#[test]
fn raw_output_sampling_and_reset_hashes_are_derived_from_supplied_bytes() {
    let fixture = fixture();
    let prepared = prepare_pair(
        &fixture.trial,
        &fixture.manifest,
        &fixture.task,
        &fixture.patch,
        &fixture.baseline,
        &fixture.treatment,
        "pair-hashes",
        OrderSelection::Explicit(PairOrder::Ab),
    )
    .unwrap();
    let slot = &prepared.plan.slots[0];
    let sampling = b"temperature=0.2;seed=7";
    let reset = b"git reset --hard exact-head; git clean -ffd";
    let output = b"raw worker bytes including punctuation.\n";
    let value = build_external_run_receipt(
        &prepared.plan,
        &metadata(
            &slot.slot_id,
            "external-session-1",
            RunOutcome::Failed,
            ExecutionOrigin::ExternalHarness,
        ),
        sampling,
        reset,
        output,
    )
    .unwrap();

    assert_eq!(value["sampling_config_sha256"], sha256_hex(sampling));
    assert_eq!(value["checkout_reset_receipt_sha256"], sha256_hex(reset));
    assert_eq!(value["worker_output_sha256"], sha256_hex(output));
    parse_run_receipt(&serde_json::to_vec(&value).unwrap()).unwrap();
}

#[test]
fn fake_pair_reaches_real_246_retirement_signal_in_ab_and_ba_order() {
    for order in [
        OrderSelection::Explicit(PairOrder::Ab),
        OrderSelection::Explicit(PairOrder::Ba),
    ] {
        let (fixture, _prepared, left, right) = receipts_for_signal(order);
        let spec = parse_trial_spec(&fixture.trial).unwrap();
        let manifest = parse_trial_manifest(&fixture.manifest).unwrap();
        let left = parse_run_receipt(&serde_json::to_vec(&left).unwrap()).unwrap();
        let right = parse_run_receipt(&serde_json::to_vec(&right).unwrap()).unwrap();
        let evaluation = evaluate_pair(&spec, &manifest, &left, &right).unwrap();
        assert_eq!(
            evaluation.verdict,
            RetirementVerdict::PairedRetirementSignal
        );
        assert!(!evaluation.automatic_causal_claim);
        assert!(!evaluation.automatic_generalization);
    }
}

#[test]
fn same_session_contamination_is_rejected_by_real_246_evaluator() {
    let (fixture, prepared, left, right) =
        receipts_for_signal(OrderSelection::Explicit(PairOrder::Ab));
    let left: Value = left;
    let mut right: Value = right;
    right["session_id"] = left["session_id"].clone();

    let spec = parse_trial_spec(&fixture.trial).unwrap();
    let manifest = parse_trial_manifest(&fixture.manifest).unwrap();
    let left = parse_run_receipt(&serde_json::to_vec(&left).unwrap()).unwrap();
    let right = parse_run_receipt(&serde_json::to_vec(&right).unwrap()).unwrap();
    let evaluation = evaluate_pair(&spec, &manifest, &left, &right).unwrap();
    assert_eq!(evaluation.verdict, RetirementVerdict::Confounded);
    assert_eq!(prepared.plan.slots.len(), 2);
}

#[test]
fn tampered_receipt_packet_hash_rejects_before_pair_interpretation() {
    let (fixture, _prepared, left, right) =
        receipts_for_signal(OrderSelection::Explicit(PairOrder::Ab));
    let left = parse_run_receipt(&serde_json::to_vec(&left).unwrap()).unwrap();
    let mut right_value: Value = right;
    right_value["evidence_packet_sha256"] = Value::String("f".repeat(64));
    let right = parse_run_receipt(&serde_json::to_vec(&right_value).unwrap()).unwrap();
    let spec = parse_trial_spec(&fixture.trial).unwrap();
    let manifest = parse_trial_manifest(&fixture.manifest).unwrap();
    let error = evaluate_pair(&spec, &manifest, &left, &right).unwrap_err();
    assert!(error.contains("evidence packet fingerprint does not match"));
}

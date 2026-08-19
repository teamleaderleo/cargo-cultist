use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq)]
struct Registration {
    execution_id: &'static str,
    semantic_fingerprint: &'static str,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SemanticScore {
    ExactIdentity,
    EquivalentEvaluatorClone,
    DifferentSemantics,
    UnknownIdentity,
}

fn exact_identity_score(expected_execution_id: &str, triggered_execution_id: &str) -> bool {
    expected_execution_id == triggered_execution_id
}

fn semantic_score(
    registrations: &[Registration],
    expected_execution_id: &str,
    triggered_execution_id: &str,
) -> SemanticScore {
    let by_id = registrations
        .iter()
        .map(|registration| (registration.execution_id, registration))
        .collect::<BTreeMap<_, _>>();

    let Some(expected) = by_id.get(expected_execution_id) else {
        return SemanticScore::UnknownIdentity;
    };
    let Some(triggered) = by_id.get(triggered_execution_id) else {
        return SemanticScore::UnknownIdentity;
    };

    if expected.execution_id == triggered.execution_id {
        SemanticScore::ExactIdentity
    } else if expected.semantic_fingerprint == triggered.semantic_fingerprint {
        SemanticScore::EquivalentEvaluatorClone
    } else {
        SemanticScore::DifferentSemantics
    }
}

fn clone(id: &'static str, semantic_fingerprint: &'static str) -> Registration {
    Registration {
        execution_id: id,
        semantic_fingerprint,
    }
}

#[test]
fn exact_id_scoring_can_false_negative_an_evaluator_owned_equivalent_clone() {
    let registrations = [
        clone("skill-a-uuid-1111", "semantic-candidate-v1"),
        clone("skill-a-uuid-2222", "semantic-candidate-v1"),
    ];

    assert!(!exact_identity_score(
        "skill-a-uuid-1111",
        "skill-a-uuid-2222"
    ));
    assert_eq!(
        semantic_score(
            &registrations,
            "skill-a-uuid-1111",
            "skill-a-uuid-2222"
        ),
        SemanticScore::EquivalentEvaluatorClone
    );
}

#[test]
fn exact_id_scoring_remains_correct_when_the_expected_clone_is_triggered() {
    let registrations = [clone("skill-a-uuid-1111", "semantic-candidate-v1")];

    assert!(exact_identity_score(
        "skill-a-uuid-1111",
        "skill-a-uuid-1111"
    ));
    assert_eq!(
        semantic_score(
            &registrations,
            "skill-a-uuid-1111",
            "skill-a-uuid-1111"
        ),
        SemanticScore::ExactIdentity
    );
}

#[test]
fn different_semantics_do_not_get_equivalence_credit_merely_for_sharing_a_namespace() {
    let registrations = [
        clone("skill-a-uuid-1111", "semantic-candidate-v1"),
        clone("skill-b-uuid-2222", "different-candidate-v1"),
    ];

    assert!(!exact_identity_score(
        "skill-a-uuid-1111",
        "skill-b-uuid-2222"
    ));
    assert_eq!(
        semantic_score(
            &registrations,
            "skill-a-uuid-1111",
            "skill-b-uuid-2222"
        ),
        SemanticScore::DifferentSemantics
    );
}

#[test]
fn unregistered_trigger_identity_is_not_guessed_as_an_equivalent_clone() {
    let registrations = [clone("skill-a-uuid-1111", "semantic-candidate-v1")];

    assert!(!exact_identity_score(
        "skill-a-uuid-1111",
        "skill-a-uuid-9999"
    ));
    assert_eq!(
        semantic_score(
            &registrations,
            "skill-a-uuid-1111",
            "skill-a-uuid-9999"
        ),
        SemanticScore::UnknownIdentity
    );
}

#[test]
fn evaluator_clone_equivalence_is_local_and_does_not_create_durable_semantic_lineage() {
    let first_run = [clone("skill-a-uuid-1111", "semantic-candidate-v1")];
    let later_run = [clone("skill-a-uuid-3333", "semantic-candidate-v2")];

    assert_eq!(
        semantic_score(
            &first_run,
            "skill-a-uuid-1111",
            "skill-a-uuid-1111"
        ),
        SemanticScore::ExactIdentity
    );
    assert_eq!(
        semantic_score(
            &later_run,
            "skill-a-uuid-3333",
            "skill-a-uuid-3333"
        ),
        SemanticScore::ExactIdentity
    );

    assert_ne!(
        first_run[0].semantic_fingerprint,
        later_run[0].semantic_fingerprint
    );
}

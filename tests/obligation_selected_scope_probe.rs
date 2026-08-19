#![allow(dead_code)]

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/justification.rs"]
mod justification;
#[path = "../src/durable_obligation.rs"]
mod durable_obligation;
#[path = "../src/evidence_planner.rs"]
mod evidence_planner;

use applicability::{
    EvaluationContext, EvidenceRequirements, PathScope, PathScopeMode,
};
use durable_obligation::{
    ClearingCondition, DiscriminatorKey, DurableObligation,
    DURABLE_OBLIGATION_SCHEMA_VERSION,
};
use evidence_planner::{
    plan_evidence, EvidencePlanStatus, EvidenceProbe, EVIDENCE_PLANNER_SCHEMA_VERSION,
    ProbeCandidateStatus, ProbeCost, ProbeEffect, ProbePlanRequest, ProbeSelectionPolicy,
};

const REPOSITORY: &str = "owner/repo";
const REVISION: &str = "exact-head";
const TARGET: &str = "convex/schema.ts";

fn discriminator() -> DiscriminatorKey {
    DiscriminatorKey {
        kind: "distributed_failure_history".to_string(),
        target: TARGET.to_string(),
    }
}

fn scope_requirements(mode: PathScopeMode, path: &str) -> EvidenceRequirements {
    EvidenceRequirements {
        repository: Some(REPOSITORY.to_string()),
        revision: Some(REVISION.to_string()),
        work: None,
        scope: Some(PathScope {
            mode,
            path: path.to_string(),
        }),
    }
}

fn current_context() -> EvaluationContext {
    EvaluationContext {
        repository: Some(REPOSITORY.to_string()),
        revision: Some(REVISION.to_string()),
        work: None,
        path: Some(TARGET.to_string()),
    }
}

fn obligation(required: EvidenceRequirements) -> DurableObligation {
    let missing = discriminator();
    DurableObligation {
        schema_version: DURABLE_OBLIGATION_SCHEMA_VERSION,
        id: "obligation:distributed-failure-history".to_string(),
        question: "What repository history is required to evaluate the distributed failure family?"
            .to_string(),
        subject: scope_requirements(PathScopeMode::Exact, TARGET),
        established_evidence: vec!["evidence:file-local-history-insufficient".to_string()],
        missing_discriminator: missing.clone(),
        clearing_conditions: vec![ClearingCondition {
            discriminator: missing,
            requirements: required,
        }],
    }
}

fn probe(id: &str, requirements: EvidenceRequirements, git_subprocesses: u32) -> EvidenceProbe {
    EvidenceProbe {
        id: id.to_string(),
        produces: discriminator(),
        requirements,
        effect: ProbeEffect::ReadOnly,
        cost: ProbeCost {
            git_subprocesses,
            ..ProbeCost::default()
        },
    }
}

fn request(obligation: DurableObligation, probes: Vec<EvidenceProbe>) -> ProbePlanRequest {
    ProbePlanRequest {
        schema_version: EVIDENCE_PLANNER_SCHEMA_VERSION,
        obligation,
        context: current_context(),
        probes,
        allow_effectful: false,
        policy: ProbeSelectionPolicy::Conservative,
    }
}

fn candidate_status(plan: &evidence_planner::EvidencePlan, id: &str) -> ProbeCandidateStatus {
    plan.candidates
        .iter()
        .find(|candidate| candidate.id == id)
        .unwrap_or_else(|| panic!("missing candidate {id}"))
        .status
}

#[test]
fn obligation_can_select_exact_bounded_scope_even_when_wrong_scope_probes_are_cheaper() {
    let plan = plan_evidence(&request(
        obligation(scope_requirements(PathScopeMode::Prefix, "convex")),
        vec![
            probe(
                "file-history-cheapest",
                scope_requirements(PathScopeMode::Exact, TARGET),
                0,
            ),
            probe(
                "convex-history-required",
                scope_requirements(PathScopeMode::Prefix, "convex"),
                4,
            ),
            probe(
                "repo-history-cheaper",
                scope_requirements(PathScopeMode::Prefix, "."),
                0,
            ),
        ],
    ))
    .unwrap();

    assert_eq!(plan.status, EvidencePlanStatus::Selected);
    assert_eq!(
        plan.selected.as_ref().map(|selected| selected.id.as_str()),
        Some("convex-history-required")
    );
    assert_eq!(
        candidate_status(&plan, "convex-history-required"),
        ProbeCandidateStatus::Eligible
    );
    assert_eq!(
        candidate_status(&plan, "file-history-cheapest"),
        ProbeCandidateStatus::IncompatibleClearingCondition
    );
    assert_eq!(
        candidate_status(&plan, "repo-history-cheaper"),
        ProbeCandidateStatus::IncompatibleClearingCondition
    );
}

#[test]
fn file_local_clearing_condition_keeps_parent_scope_quiet_even_when_parent_is_cheaper() {
    let plan = plan_evidence(&request(
        obligation(scope_requirements(PathScopeMode::Exact, TARGET)),
        vec![
            probe(
                "file-history-required",
                scope_requirements(PathScopeMode::Exact, TARGET),
                3,
            ),
            probe(
                "convex-history-cheaper",
                scope_requirements(PathScopeMode::Prefix, "convex"),
                0,
            ),
        ],
    ))
    .unwrap();

    assert_eq!(plan.status, EvidencePlanStatus::Selected);
    assert_eq!(
        plan.selected.as_ref().map(|selected| selected.id.as_str()),
        Some("file-history-required")
    );
    assert_eq!(
        candidate_status(&plan, "file-history-required"),
        ProbeCandidateStatus::Eligible
    );
    assert_eq!(
        candidate_status(&plan, "convex-history-cheaper"),
        ProbeCandidateStatus::IncompatibleClearingCondition
    );
}

#[test]
fn required_scope_probe_blocks_when_current_path_context_is_missing_instead_of_guessing() {
    let mut request = request(
        obligation(scope_requirements(PathScopeMode::Prefix, "convex")),
        vec![probe(
            "convex-history-required",
            scope_requirements(PathScopeMode::Prefix, "convex"),
            1,
        )],
    );
    request.context.path = None;

    let plan = plan_evidence(&request).unwrap();

    assert_eq!(plan.status, EvidencePlanStatus::Blocked);
    assert!(plan.selected.is_none());
    assert_eq!(
        candidate_status(&plan, "convex-history-required"),
        ProbeCandidateStatus::MissingContext
    );
}

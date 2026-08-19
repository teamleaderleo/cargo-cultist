#[path = "../src/finding.rs"]
mod finding;
#[path = "../src/render.rs"]
mod render;

use finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding};
use render::render_terse_analysis_report;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum EvidenceRole {
    Support,
    Counterexample,
    Limit,
    Clearing,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct TaggedEvidence {
    role: EvidenceRole,
    message: &'static str,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum NextAction {
    Proceed,
    AddCounterexample,
    ReconcileException,
    ExecuteClearingStep,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ProjectionCase {
    id: &'static str,
    level: ClaimKind,
    claim: &'static str,
    evidence: Vec<TaggedEvidence>,
}

impl ProjectionCase {
    fn new(
        id: &'static str,
        level: ClaimKind,
        claim: &'static str,
        evidence: Vec<TaggedEvidence>,
    ) -> Self {
        Self {
            id,
            level,
            claim,
            evidence,
        }
    }

    fn report(&self) -> AnalysisReport {
        let mut claim = Claim::new(self.level, self.claim);
        claim.evidence = self
            .evidence
            .iter()
            .map(|evidence| Evidence::new(evidence.message))
            .collect();

        AnalysisReport {
            schema_version: 1,
            analysis: "evidence-role-projection".to_string(),
            repository: "/repo".to_string(),
            claims: Vec::new(),
            findings: vec![Finding::new("projection-case", self.id).with_claim(claim)],
        }
    }
}

fn support(message: &'static str) -> TaggedEvidence {
    TaggedEvidence {
        role: EvidenceRole::Support,
        message,
    }
}

fn counterexample(message: &'static str) -> TaggedEvidence {
    TaggedEvidence {
        role: EvidenceRole::Counterexample,
        message,
    }
}

fn limit(message: &'static str) -> TaggedEvidence {
    TaggedEvidence {
        role: EvidenceRole::Limit,
        message,
    }
}

fn clearing(message: &'static str) -> TaggedEvidence {
    TaggedEvidence {
        role: EvidenceRole::Clearing,
        message,
    }
}

fn current_terse(case: &ProjectionCase) -> String {
    render_terse_analysis_report(&case.report())
}

fn role_aware_projection(case: &ProjectionCase) -> String {
    let mut output = format!("F1 projection-case\n  C1 {} {}", level_token(case.level), case.claim);
    for evidence in &case.evidence {
        match evidence.role {
            EvidenceRole::Support => {}
            EvidenceRole::Counterexample => output.push_str("\n  E! counterexample"),
            EvidenceRole::Limit => output.push_str("\n  E! limit"),
            EvidenceRole::Clearing => output.push_str("\n  E! clearing"),
        }
    }
    output.push('\n');
    output
}

fn full_role_projection(case: &ProjectionCase) -> String {
    let mut output = format!("{} {:?}: {}\n", case.id, case.level, case.claim);
    for evidence in &case.evidence {
        output.push_str(&format!("  {:?}: {}\n", evidence.role, evidence.message));
    }
    output
}

fn required_action(case: &ProjectionCase) -> NextAction {
    if case
        .evidence
        .iter()
        .any(|evidence| evidence.role == EvidenceRole::Clearing)
    {
        NextAction::ExecuteClearingStep
    } else if case
        .evidence
        .iter()
        .any(|evidence| evidence.role == EvidenceRole::Limit)
    {
        NextAction::AddCounterexample
    } else if case
        .evidence
        .iter()
        .any(|evidence| evidence.role == EvidenceRole::Counterexample)
    {
        NextAction::ReconcileException
    } else {
        NextAction::Proceed
    }
}

fn action_from_role_aware_projection(projection: &str) -> NextAction {
    if projection.contains("E! clearing") {
        NextAction::ExecuteClearingStep
    } else if projection.contains("E! limit") {
        NextAction::AddCounterexample
    } else if projection.contains("E! counterexample") {
        NextAction::ReconcileException
    } else {
        NextAction::Proceed
    }
}

fn level_token(level: ClaimKind) -> &'static str {
    match level {
        ClaimKind::Proven => "P",
        ClaimKind::Derived => "D",
        ClaimKind::Observed => "O",
        ClaimKind::Inferred => "I",
        ClaimKind::Unknown => "?",
    }
}

#[test]
fn actual_terse_renderer_collapses_scope_limit_that_changes_action() {
    let unrestricted = ProjectionCase::new(
        "unrestricted",
        ClaimKind::Proven,
        "target test passed",
        vec![support("target execution passed")],
    );
    let linux_only = ProjectionCase::new(
        "linux-only",
        ClaimKind::Proven,
        "target test passed",
        vec![
            support("target execution passed"),
            limit("receipt is valid only for linux-x86_64"),
        ],
    );

    assert_eq!(current_terse(&unrestricted), current_terse(&linux_only));
    assert_ne!(required_action(&unrestricted), required_action(&linux_only));
    assert_ne!(
        role_aware_projection(&unrestricted),
        role_aware_projection(&linux_only)
    );
}

#[test]
fn actual_terse_renderer_collapses_counterexample_that_changes_action() {
    let ordinary = ProjectionCase::new(
        "ordinary",
        ClaimKind::Observed,
        "helper is the local precedent",
        vec![support("six nearby callers use helper")],
    );
    let matching_exception = ProjectionCase::new(
        "matching-exception",
        ClaimKind::Observed,
        "helper is the local precedent",
        vec![
            support("six nearby callers use helper"),
            counterexample("one reviewed exception matches this change scope"),
        ],
    );

    assert_eq!(current_terse(&ordinary), current_terse(&matching_exception));
    assert_ne!(required_action(&ordinary), required_action(&matching_exception));
    assert_ne!(
        role_aware_projection(&ordinary),
        role_aware_projection(&matching_exception)
    );
}

#[test]
fn actual_terse_renderer_collapses_clearing_evidence_that_changes_action() {
    let blocked = ProjectionCase::new(
        "blocked",
        ClaimKind::Unknown,
        "merge eligibility is unresolved",
        vec![support("exact target execution is absent")],
    );
    let actionable = ProjectionCase::new(
        "actionable",
        ClaimKind::Unknown,
        "merge eligibility is unresolved",
        vec![
            support("exact target execution is absent"),
            clearing("run exact target execution at current head"),
        ],
    );

    assert_eq!(current_terse(&blocked), current_terse(&actionable));
    assert_eq!(required_action(&blocked), NextAction::Proceed);
    assert_eq!(required_action(&actionable), NextAction::ExecuteClearingStep);
    assert_ne!(
        role_aware_projection(&blocked),
        role_aware_projection(&actionable)
    );
}

#[test]
fn support_only_evidence_can_be_omitted_without_changing_modeled_action() {
    let case = ProjectionCase::new(
        "support-only",
        ClaimKind::Proven,
        "exact fixture replay passed",
        vec![
            support("fixture A passed"),
            support("fixture B passed"),
            support("fixture C passed"),
        ],
    );

    let full = full_role_projection(&case);
    let compact = role_aware_projection(&case);
    assert!(compact.len() < full.len());
    assert_eq!(
        required_action(&case),
        action_from_role_aware_projection(&compact)
    );
    assert!(!compact.contains("fixture A passed"));
    assert!(!compact.contains("fixture B passed"));
    assert!(!compact.contains("fixture C passed"));
}

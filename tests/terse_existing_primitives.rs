#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;
#[allow(dead_code)]
#[path = "../src/render.rs"]
mod render;

use finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding};
use render::render_terse_analysis_report;

fn finding_with_hidden_limit() -> AnalysisReport {
    AnalysisReport {
        schema_version: 1,
        analysis: "terse-existing-primitives".to_string(),
        repository: "/repo".to_string(),
        claims: Vec::new(),
        findings: vec![Finding::new("execution", "Target execution").with_claim(
            Claim::new(ClaimKind::Proven, "target test passed")
                .with_evidence(Evidence::new("execution covered Linux only")),
        )],
    }
}

fn finding_with_explicit_coverage_unknown() -> AnalysisReport {
    AnalysisReport {
        schema_version: 1,
        analysis: "terse-existing-primitives".to_string(),
        repository: "/repo".to_string(),
        claims: Vec::new(),
        findings: vec![
            Finding::new("execution", "Target execution")
                .with_claim(Claim::new(ClaimKind::Proven, "target test passed"))
                .with_claim(Claim::new(
                    ClaimKind::Unknown,
                    "cross-platform coverage is not established",
                )),
        ],
    }
}

fn precedent_with_hidden_counterexample() -> AnalysisReport {
    AnalysisReport {
        schema_version: 1,
        analysis: "terse-existing-primitives".to_string(),
        repository: "/repo".to_string(),
        claims: Vec::new(),
        findings: vec![Finding::new("precedent", "Local precedent").with_claim(
            Claim::new(ClaimKind::Observed, "helper is the local precedent").with_evidence(
                Evidence::new("one reviewed exception matches the current change scope"),
            ),
        )],
    }
}

fn precedent_with_explicit_counterexample_claim() -> AnalysisReport {
    AnalysisReport {
        schema_version: 1,
        analysis: "terse-existing-primitives".to_string(),
        repository: "/repo".to_string(),
        claims: Vec::new(),
        findings: vec![
            Finding::new("precedent", "Local precedent")
                .with_claim(Claim::new(
                    ClaimKind::Observed,
                    "helper is the local precedent",
                ))
                .with_claim(Claim::new(
                    ClaimKind::Observed,
                    "one reviewed exception matches the current change scope",
                )),
        ],
    }
}

fn blocked_without_question() -> AnalysisReport {
    AnalysisReport {
        schema_version: 1,
        analysis: "terse-existing-primitives".to_string(),
        repository: "/repo".to_string(),
        claims: Vec::new(),
        findings: vec![Finding::new("execution", "Merge eligibility").with_claim(Claim::new(
            ClaimKind::Unknown,
            "exact-head target execution is missing",
        ))],
    }
}

fn blocked_with_clearing_question() -> AnalysisReport {
    AnalysisReport {
        schema_version: 1,
        analysis: "terse-existing-primitives".to_string(),
        repository: "/repo".to_string(),
        claims: Vec::new(),
        findings: vec![
            Finding::new("execution", "Merge eligibility")
                .with_claim(Claim::new(
                    ClaimKind::Unknown,
                    "exact-head target execution is missing",
                ))
                .with_question("Run exact target execution at HEAD?"),
        ],
    }
}

#[test]
fn promoting_a_coverage_boundary_to_unknown_makes_it_survive_terse_projection() {
    let hidden = render_terse_analysis_report(&finding_with_hidden_limit());
    let explicit = render_terse_analysis_report(&finding_with_explicit_coverage_unknown());

    assert!(!hidden.contains("Linux only"));
    assert!(!hidden.contains("cross-platform"));
    assert!(explicit.contains("? cross-platform coverage is not established"));
    assert_ne!(hidden, explicit);
}

#[test]
fn promoting_a_counterexample_to_a_claim_makes_it_survive_terse_projection() {
    let hidden = render_terse_analysis_report(&precedent_with_hidden_counterexample());
    let explicit = render_terse_analysis_report(&precedent_with_explicit_counterexample_claim());

    assert!(!hidden.contains("reviewed exception"));
    assert!(explicit.contains("O one reviewed exception matches the current change scope"));
    assert_ne!(hidden, explicit);
}

#[test]
fn using_the_existing_question_field_preserves_a_clearing_step() {
    let blocked = render_terse_analysis_report(&blocked_without_question());
    let actionable = render_terse_analysis_report(&blocked_with_clearing_question());

    assert!(!blocked.contains("Run exact target execution"));
    assert!(actionable.contains("Q Run exact target execution at HEAD?"));
    assert_ne!(blocked, actionable);
}

#[test]
fn existing_primitives_are_a_competing_hypothesis_not_a_role_schema_proof() {
    let coverage = render_terse_analysis_report(&finding_with_explicit_coverage_unknown());
    let counterexample =
        render_terse_analysis_report(&precedent_with_explicit_counterexample_claim());
    let clearing = render_terse_analysis_report(&blocked_with_clearing_question());

    // These strings show the decision-relevant information survives today,
    // but they do not encode an explicit machine role such as "coverage limit"
    // or "counterexample". A receiver still has to interpret claim text/kind.
    assert!(coverage.contains("? cross-platform coverage is not established"));
    assert!(counterexample.contains("O one reviewed exception"));
    assert!(clearing.contains("Q Run exact target execution at HEAD?"));
}

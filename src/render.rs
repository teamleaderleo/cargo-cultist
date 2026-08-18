use std::fmt::Write;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};

pub fn render_analysis_report(report: &AnalysisReport) -> String {
    let mut output = String::new();
    writeln!(output, "{}", analysis_title(&report.analysis)).unwrap();

    for claim in &report.claims {
        write_claim(&mut output, claim, 0);
    }

    for (index, finding) in report.findings.iter().enumerate() {
        writeln!(output, "\nFINDING {}: {}", index + 1, finding.title).unwrap();
        write_finding(&mut output, finding);
    }

    output
}

fn write_finding(output: &mut String, finding: &Finding) {
    if let Some(location) = &finding.location {
        writeln!(output, "  at {}", format_location(location)).unwrap();
    }

    for claim in &finding.claims {
        write_claim(output, claim, 2);
    }

    if let Some(question) = &finding.question {
        writeln!(output, "\nQUESTION").unwrap();
        writeln!(output, "  {question}").unwrap();
    }
}

fn write_claim(output: &mut String, claim: &Claim, indent: usize) {
    if !output.is_empty() && !output.ends_with("\n\n") {
        output.push('\n');
    }

    let prefix = " ".repeat(indent);
    writeln!(output, "{prefix}{}", claim_kind_label(claim.kind)).unwrap();
    writeln!(output, "{prefix}  {}", claim.message).unwrap();

    for evidence in &claim.evidence {
        write_evidence(output, evidence, indent + 2);
    }
}

fn write_evidence(output: &mut String, evidence: &Evidence, indent: usize) {
    let prefix = " ".repeat(indent);
    match &evidence.location {
        Some(location) => writeln!(
            output,
            "{prefix}- {} ({})",
            evidence.message,
            format_location(location)
        )
        .unwrap(),
        None => writeln!(output, "{prefix}- {}", evidence.message).unwrap(),
    }
}

fn format_location(location: &Location) -> String {
    match location.line {
        Some(line) => format!("{}:{line}", location.path),
        None => location.path.clone(),
    }
}

fn claim_kind_label(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Proven => "PROVEN",
        ClaimKind::Derived => "DERIVED",
        ClaimKind::Observed => "OBSERVED",
        ClaimKind::Inferred => "INFERRED",
        ClaimKind::Unknown => "UNKNOWN",
    }
}

fn analysis_title(analysis: &str) -> String {
    analysis
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::{Evidence, Finding};

    #[test]
    fn renders_provenance_and_evidence() {
        let report = AnalysisReport {
            schema_version: 1,
            analysis: "diff-precedent".to_string(),
            repository: "/repo".to_string(),
            claims: vec![Claim::new(
                ClaimKind::Derived,
                "The diff changes one Rust file.",
            )],
            findings: vec![
                Finding::new("example", "Example tension")
                    .at(Location::new("src/lib.rs", Some(42)))
                    .with_claim(
                        Claim::new(ClaimKind::Observed, "Two scopes disagree.").with_evidence(
                            Evidence::at(
                                "The file already uses `tests`.",
                                Location::new("src/lib.rs", Some(10)),
                            ),
                        ),
                    )
                    .with_claim(Claim::new(
                        ClaimKind::Unknown,
                        "The repository does not state which scope wins.",
                    ))
                    .with_question("Which scope is intentional here?"),
            ],
        };

        let rendered = render_analysis_report(&report);
        assert!(rendered.contains("DIFF PRECEDENT"));
        assert!(rendered.contains("DERIVED"));
        assert!(rendered.contains("OBSERVED"));
        assert!(rendered.contains("UNKNOWN"));
        assert!(rendered.contains("src/lib.rs:42"));
        assert!(rendered.contains("QUESTION"));
    }

    #[test]
    fn labels_every_claim_kind() {
        assert_eq!(claim_kind_label(ClaimKind::Proven), "PROVEN");
        assert_eq!(claim_kind_label(ClaimKind::Derived), "DERIVED");
        assert_eq!(claim_kind_label(ClaimKind::Observed), "OBSERVED");
        assert_eq!(claim_kind_label(ClaimKind::Inferred), "INFERRED");
        assert_eq!(claim_kind_label(ClaimKind::Unknown), "UNKNOWN");
    }
}

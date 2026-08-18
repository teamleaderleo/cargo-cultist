use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};
use crate::test_modules::{TestModuleOccurrence, TestModuleReport};

pub fn build_test_module_analysis(root: &Path, report: &TestModuleReport) -> AnalysisReport {
    let mut analysis = AnalysisReport::new(
        "test-module-conventions",
        root.to_string_lossy().into_owned(),
    );

    if report.occurrences.is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "No test-gated modules were found in the parsed Rust files.",
        ));
        add_parse_failures(root, report, &mut analysis);
        return analysis;
    }

    let counts = module_name_counts(report);
    let total = report.occurrences.len();
    let mut ranked_counts: Vec<_> = counts.iter().collect();
    ranked_counts.sort_by(|(name_a, count_a), (name_b, count_b)| {
        count_b.cmp(count_a).then(name_a.cmp(name_b))
    });

    let dominant_count = *ranked_counts[0].1;
    let dominant_names: Vec<_> = ranked_counts
        .iter()
        .take_while(|(_, count)| **count == dominant_count)
        .map(|(name, _)| (*name).clone())
        .collect();

    let distribution = ranked_counts
        .iter()
        .map(|(name, count)| format!("`{name}`={count}"))
        .collect::<Vec<_>>()
        .join(", ");

    analysis.claims.push(
        Claim::new(
            ClaimKind::Observed,
            format!("Found {total} test-gated modules across {} distinct names.", counts.len()),
        )
        .with_evidence(Evidence::new(format!("Repository counts: {distribution}."))),
    );

    if dominant_names.len() == 1 {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            format!(
                "`{}` is the most frequent test-module name ({dominant_count} of {total}).",
                dominant_names[0]
            ),
        ));
    } else {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            format!(
                "No single test-module name dominates: {} names are tied at {dominant_count} occurrences.",
                dominant_names.len()
            ),
        ));
    }

    add_local_mix_findings(root, report, &mut analysis);
    add_one_off_findings(root, report, &counts, &mut analysis);
    add_parse_failures(root, report, &mut analysis);

    analysis
}

fn add_local_mix_findings(root: &Path, report: &TestModuleReport, analysis: &mut AnalysisReport) {
    let mut by_file = BTreeMap::<_, Vec<_>>::new();
    for occurrence in &report.occurrences {
        by_file
            .entry(&occurrence.path)
            .or_default()
            .push(occurrence);
    }

    for (path, occurrences) in by_file {
        let names = occurrences
            .iter()
            .map(|occurrence| occurrence.name.as_str())
            .collect::<BTreeSet<_>>();
        if names.len() <= 1 {
            continue;
        }

        let display_path = relative_path(root, path).to_string_lossy().into_owned();
        let mut claim = Claim::new(
            ClaimKind::Observed,
            format!(
                "This file uses {} different names for test-gated modules: {}.",
                names.len(),
                names
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );

        for occurrence in occurrences {
            claim = claim.with_evidence(Evidence::at(
                format!("`mod {}` is test-gated here.", occurrence.name),
                location(root, occurrence),
            ));
        }

        analysis.findings.push(
            Finding::new("test-module-local-mix", "File-local test-module naming mix")
                .at(Location::new(display_path, None))
                .with_claim(claim)
                .with_question(
                    "Is the local mix deliberate, or would one name make the file easier to read?",
                ),
        );
    }
}

fn add_one_off_findings(
    root: &Path,
    report: &TestModuleReport,
    counts: &BTreeMap<String, usize>,
    analysis: &mut AnalysisReport,
) {
    if report.occurrences.len() <= 1 {
        return;
    }

    for occurrence in &report.occurrences {
        if counts.get(&occurrence.name).copied() != Some(1) {
            continue;
        }

        analysis.findings.push(
            Finding::new("test-module-one-off", "One-off test-module name")
                .at(location(root, occurrence))
                .with_claim(
                    Claim::new(
                        ClaimKind::Observed,
                        format!(
                            "`{}` appears once across {} test-gated modules.",
                            occurrence.name,
                            report.occurrences.len()
                        ),
                    )
                    .with_evidence(Evidence::at(
                        format!("`mod {}` is declared here.", occurrence.name),
                        location(root, occurrence),
                    )),
                )
                .with_question(
                    "Is this one-off name intentionally scoped, or an accidental deviation from local precedent?",
                ),
        );
    }
}

fn add_parse_failures(root: &Path, report: &TestModuleReport, analysis: &mut AnalysisReport) {
    for (path, error) in &report.parse_failures {
        analysis.claims.push(
            Claim::new(
                ClaimKind::Unknown,
                "A Rust file could not be parsed, so repository observations may be incomplete.",
            )
            .with_evidence(Evidence::at(
                error.clone(),
                Location::new(relative_path(root, path).to_string_lossy(), None),
            )),
        );
    }
}

fn module_name_counts(report: &TestModuleReport) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for occurrence in &report.occurrences {
        *counts.entry(occurrence.name.clone()).or_default() += 1;
    }
    counts
}

fn location(root: &Path, occurrence: &TestModuleOccurrence) -> Location {
    Location::new(
        relative_path(root, &occurrence.path).to_string_lossy(),
        Some(occurrence.line),
    )
}

fn relative_path<'a>(root: &'a Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn reports_one_off_names_as_observations() {
        let root = Path::new("/repo");
        let report = TestModuleReport {
            occurrences: vec![
                TestModuleOccurrence {
                    name: "tests".to_string(),
                    path: PathBuf::from("/repo/src/a.rs"),
                    line: 10,
                },
                TestModuleOccurrence {
                    name: "special_tests".to_string(),
                    path: PathBuf::from("/repo/src/b.rs"),
                    line: 20,
                },
            ],
            parse_failures: Vec::new(),
        };

        let analysis = build_test_module_analysis(root, &report);
        assert!(analysis.findings.iter().any(|finding| {
            finding.kind == "test-module-one-off"
                && finding
                    .claims
                    .iter()
                    .any(|claim| claim.kind == ClaimKind::Observed)
        }));
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};
use crate::test_modules::{TestModuleOccurrence, TestModuleReport};

#[derive(Debug, Default, Eq, PartialEq)]
struct ChangedLines {
    by_path: BTreeMap<PathBuf, BTreeSet<usize>>,
}

impl ChangedLines {
    fn insert(&mut self, path: &Path, line: usize) {
        self.by_path
            .entry(path.to_path_buf())
            .or_default()
            .insert(line);
    }

    fn contains(&self, path: &Path, line: usize) -> bool {
        self.by_path
            .get(path)
            .is_some_and(|lines| lines.contains(&line))
    }

    fn rust_file_count(&self) -> usize {
        self.by_path
            .keys()
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
            .count()
    }
}

pub fn git_repo_root(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{path:?} is not inside a Git repository: {stderr}").into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(PathBuf::from(stdout.trim()).canonicalize()?)
}

pub fn build_diff_analysis_report(
    root: &Path,
    base: Option<&str>,
    report: &TestModuleReport,
) -> Result<AnalysisReport, Box<dyn Error>> {
    let patch = git_diff(root, base)?;
    let changed = parse_changed_lines(&patch);
    let changed_modules = changed_test_modules(root, report, &changed);

    let mut analysis = AnalysisReport::new("diff-precedent", root.to_string_lossy().into_owned());
    analysis.claims.push(Claim::new(
        ClaimKind::Derived,
        match base {
            Some(base) => format!(
                "The diff contains added lines in {} Rust file(s), measured from the merge base with `{base}`.",
                changed.rust_file_count()
            ),
            None => format!(
                "The working-tree diff contains added lines in {} Rust file(s) relative to HEAD.",
                changed.rust_file_count()
            ),
        },
    ));

    if changed_modules.is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "No added or renamed test-gated module declarations were found in the diff.",
        ));
        return Ok(analysis);
    }

    for occurrence in changed_modules {
        let local_names = same_file_names(report, occurrence);
        let different_local_names: Vec<_> = local_names
            .iter()
            .filter(|name| name.as_str() != occurrence.name)
            .cloned()
            .collect();
        let precedent_counts = module_name_counts_excluding(report, occurrence);
        let repository_count = precedent_counts
            .get(&occurrence.name)
            .copied()
            .unwrap_or_default();
        let precedent_total = report.occurrences.len().saturating_sub(1);
        let one_off = repository_count == 0 && precedent_total > 0;
        let tension = precedent_tension(&precedent_counts, &local_names);

        if different_local_names.is_empty() && !one_off {
            continue;
        }

        let occurrence_location = Location::new(
            relative_path(root, &occurrence.path)
                .to_string_lossy()
                .into_owned(),
            Some(occurrence.line),
        );
        let mut finding = Finding::new("test-module-precedent", "Test-module precedent")
            .at(occurrence_location.clone())
            .with_claim(
                Claim::new(
                    ClaimKind::Observed,
                    format!(
                        "`{}` appears {repository_count} time(s) across {precedent_total} existing test-gated modules, excluding the changed declaration.",
                        occurrence.name
                    ),
                )
                .with_evidence(Evidence::at(
                    format!(
                        "This change adds `mod {}` behind a test cfg.",
                        occurrence.name
                    ),
                    occurrence_location,
                ))
                .with_evidence(Evidence::new(format!(
                    "Repository precedent counts: {}.",
                    top_counts_summary(&precedent_counts)
                ))),
            );

        if !different_local_names.is_empty() {
            finding = finding.with_claim(Claim::new(
                ClaimKind::Observed,
                format!(
                    "The same file already uses: {}.",
                    different_local_names
                        .iter()
                        .map(|name| format!("`{name}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }

        if let Some((repository_name, file_name)) = tension {
            finding.kind = "test-module-precedent-tension".to_string();
            finding.title = "Test-module precedent tension".to_string();
            finding = finding.with_claim(Claim::new(
                ClaimKind::Observed,
                format!(
                    "Repository-wide precedent favors `{repository_name}`, while existing file-local precedent favors `{file_name}`."
                ),
            ));
        }

        let observation = match tension {
            Some((repository_name, file_name)) => {
                let alignment = if occurrence.name == repository_name {
                    "The change follows repository-wide precedent and differs from file-local precedent."
                } else if occurrence.name == file_name {
                    "The change follows file-local precedent and differs from repository-wide precedent."
                } else {
                    "The change follows neither of the two conflicting precedent scopes."
                };
                format!("Repository-wide and file-local precedent disagree. {alignment}")
            }
            None if !different_local_names.is_empty() && one_off => {
                "The new name differs from this file's existing precedent and does not appear elsewhere in the repository.".to_string()
            }
            None if !different_local_names.is_empty() => {
                "The new name differs from this file's existing test-module precedent.".to_string()
            }
            None => {
                "The new name does not appear among the repository's existing test-gated modules."
                    .to_string()
            }
        };

        finding = finding
            .with_claim(Claim::new(ClaimKind::Observed, observation))
            .with_claim(Claim::new(
                ClaimKind::Unknown,
                "Repository evidence alone does not establish which naming scope should govern this change.",
            ))
            .with_question(
                "Is the distinct module name intentional, or should it follow nearby precedent?",
            );

        analysis.findings.push(finding);
    }

    if analysis.findings.is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "The changed test-module names match existing repository precedent.",
        ));
    }

    Ok(analysis)
}

pub fn print_diff_report(
    root: &Path,
    base: Option<&str>,
    report: &TestModuleReport,
) -> Result<(), Box<dyn Error>> {
    let patch = git_diff(root, base)?;
    let changed = parse_changed_lines(&patch);
    let changed_modules = changed_test_modules(root, report, &changed);

    println!("DIFF PRECEDENT");
    match base {
        Some(base) => println!("  comparing changes from merge base with: {base}"),
        None => println!("  comparing working tree against: HEAD"),
    }
    println!(
        "  Rust files with added lines: {}",
        changed.rust_file_count()
    );

    if changed_modules.is_empty() {
        println!("\nOBSERVATION");
        println!("  No added or renamed test-gated module declarations were found.");
        return Ok(());
    }

    println!("\nCHANGED TEST MODULES");
    for occurrence in &changed_modules {
        let path = relative_path(root, &occurrence.path);
        println!(
            "  {}:{}  mod {}",
            path.display(),
            occurrence.line,
            occurrence.name
        );
    }

    let mut finding_count = 0;

    for occurrence in changed_modules {
        let local_names = same_file_names(report, occurrence);
        let different_local_names: Vec<_> = local_names
            .iter()
            .filter(|name| name.as_str() != occurrence.name)
            .collect();
        let precedent_counts = module_name_counts_excluding(report, occurrence);
        let repository_count = precedent_counts
            .get(&occurrence.name)
            .copied()
            .unwrap_or_default();
        let precedent_total = report.occurrences.len().saturating_sub(1);
        let one_off = repository_count == 0 && precedent_total > 0;
        let tension = precedent_tension(&precedent_counts, &local_names);

        if different_local_names.is_empty() && !one_off {
            continue;
        }

        finding_count += 1;
        let path = relative_path(root, &occurrence.path);
        println!("\nFINDING {finding_count}: test-module precedent");
        println!(
            "  {}:{} adds `mod {}` behind a test cfg.",
            path.display(),
            occurrence.line,
            occurrence.name
        );
        println!("\nFACTS");
        println!(
            "  `{}` appears {} time(s) across {} existing test-gated modules, excluding this changed declaration.",
            occurrence.name, repository_count, precedent_total
        );
        print_top_counts(&precedent_counts);

        if !different_local_names.is_empty() {
            println!(
                "  The same file already uses: {}.",
                different_local_names
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if let Some((repository_name, file_name)) = tension {
            println!("\nPRECEDENT TENSION");
            println!("  Repository-wide precedent favors: `{repository_name}`");
            println!("  Existing file-local precedent favors: `{file_name}`");
        }

        println!("\nOBSERVATION");
        if let Some((repository_name, file_name)) = tension {
            if occurrence.name == repository_name {
                println!(
                    "  Repository-wide and file-local precedent disagree. The change follows repository-wide precedent and differs from file-local precedent."
                );
            } else if occurrence.name == file_name {
                println!(
                    "  Repository-wide and file-local precedent disagree. The change follows file-local precedent and differs from repository-wide precedent."
                );
            } else {
                println!(
                    "  Repository-wide and file-local precedent disagree. The change follows neither scope."
                );
            }
        } else if !different_local_names.is_empty() && one_off {
            println!(
                "  The new name differs from this file's existing precedent and does not appear elsewhere in the repository."
            );
        } else if !different_local_names.is_empty() {
            println!("  The new name differs from this file's existing test-module precedent.");
        } else {
            println!(
                "  The new name does not appear among the repository's existing test-gated modules."
            );
        }

        println!("\nQUESTION");
        println!(
            "  Is the distinct module name intentional, or should it follow nearby precedent?"
        );
    }

    if finding_count == 0 {
        println!("\nOBSERVATION");
        println!("  The changed test-module names match existing repository precedent.");
    }

    Ok(())
}

fn git_diff(root: &Path, base: Option<&str>) -> Result<String, Box<dyn Error>> {
    let anchor = match base {
        Some(base) => merge_base(root, base)?,
        None => "HEAD".to_string(),
    };

    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "diff",
            "--unified=0",
            "--no-ext-diff",
            "--no-color",
            "--no-prefix",
        ])
        .arg(anchor)
        .arg("--")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed: {stderr}").into());
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn merge_base(root: &Path, base: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", base, "HEAD"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("could not find merge base for `{base}` and HEAD: {stderr}").into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn parse_changed_lines(patch: &str) -> ChangedLines {
    let mut changed = ChangedLines::default();
    let mut current_path: Option<PathBuf> = None;
    let mut current_new_line: Option<usize> = None;

    for line in patch.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            current_path = (path != "/dev/null").then(|| PathBuf::from(path));
            current_new_line = None;
            continue;
        }

        if line.starts_with("@@") {
            current_new_line = hunk_new_start(line);
            continue;
        }

        let Some(new_line) = current_new_line else {
            continue;
        };

        if line.starts_with('+') && !line.starts_with("+++") {
            if let Some(path) = &current_path {
                changed.insert(path, new_line);
            }
            current_new_line = Some(new_line + 1);
        } else if line.starts_with('-') && !line.starts_with("---") {
            // Deleted lines do not advance the line number in the new file.
        } else if !line.starts_with('\\') {
            current_new_line = Some(new_line + 1);
        }
    }

    changed
}

fn hunk_new_start(header: &str) -> Option<usize> {
    let range = header
        .split_whitespace()
        .find(|part| part.starts_with('+'))?
        .trim_start_matches('+');
    let start = range.split_once(',').map_or(range, |(start, _)| start);
    start.parse().ok()
}

fn changed_test_modules<'a>(
    root: &Path,
    report: &'a TestModuleReport,
    changed: &ChangedLines,
) -> Vec<&'a TestModuleOccurrence> {
    report
        .occurrences
        .iter()
        .filter(|occurrence| {
            occurrence
                .path
                .strip_prefix(root)
                .is_ok_and(|path| changed.contains(path, occurrence.line))
        })
        .collect()
}

fn module_name_counts_excluding(
    report: &TestModuleReport,
    target: &TestModuleOccurrence,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for occurrence in &report.occurrences {
        if occurrence.path == target.path
            && occurrence.line == target.line
            && occurrence.name == target.name
        {
            continue;
        }
        *counts.entry(occurrence.name.clone()).or_default() += 1;
    }
    counts
}

fn same_file_names(report: &TestModuleReport, target: &TestModuleOccurrence) -> BTreeSet<String> {
    report
        .occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.path == target.path
                && (occurrence.line != target.line || occurrence.name != target.name)
        })
        .map(|occurrence| occurrence.name.clone())
        .collect()
}

fn repository_dominant_name(counts: &BTreeMap<String, usize>) -> Option<&str> {
    let dominant_count = counts.values().copied().max()?;
    let mut dominant = counts
        .iter()
        .filter(|(_, count)| **count == dominant_count)
        .map(|(name, _)| name.as_str());
    let first = dominant.next()?;
    dominant.next().is_none().then_some(first)
}

fn file_local_name(local_names: &BTreeSet<String>) -> Option<&str> {
    if local_names.len() == 1 {
        local_names.iter().next().map(String::as_str)
    } else {
        None
    }
}

fn precedent_tension<'a>(
    counts: &'a BTreeMap<String, usize>,
    local_names: &'a BTreeSet<String>,
) -> Option<(&'a str, &'a str)> {
    let repository_name = repository_dominant_name(counts)?;
    let file_name = file_local_name(local_names)?;
    (repository_name != file_name).then_some((repository_name, file_name))
}

fn top_counts_summary(counts: &BTreeMap<String, usize>) -> String {
    let mut counts: Vec<_> = counts.iter().collect();
    counts.sort_by(|(name_a, count_a), (name_b, count_b)| {
        count_b.cmp(count_a).then(name_a.cmp(name_b))
    });

    counts
        .into_iter()
        .take(5)
        .map(|(name, count)| format!("`{name}`={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_top_counts(counts: &BTreeMap<String, usize>) {
    println!(
        "  Repository precedent counts: {}.",
        top_counts_summary(counts)
    );
}

fn relative_path<'a>(root: &'a Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_scope_tension_only_with_clear_precedent() {
        let counts = BTreeMap::from([("tests".to_string(), 33), ("unit_tests".to_string(), 88)]);
        let local_names = BTreeSet::from(["tests".to_string()]);
        assert_eq!(
            precedent_tension(&counts, &local_names),
            Some(("unit_tests", "tests"))
        );

        let mixed_local = BTreeSet::from(["tests".to_string(), "special_tests".to_string()]);
        assert_eq!(precedent_tension(&counts, &mixed_local), None);

        let tied_counts =
            BTreeMap::from([("tests".to_string(), 10), ("unit_tests".to_string(), 10)]);
        assert_eq!(precedent_tension(&tied_counts, &local_names), None);
    }

    #[test]
    fn excludes_changed_occurrence_from_precedent_counts() {
        let root = Path::new("/repo");
        let changed = TestModuleOccurrence {
            name: "unit_tests".to_string(),
            path: root.join("src/lib.rs"),
            line: 40,
        };
        let report = TestModuleReport {
            occurrences: vec![
                TestModuleOccurrence {
                    name: "tests".to_string(),
                    path: root.join("src/lib.rs"),
                    line: 20,
                },
                changed.clone(),
            ],
            parse_failures: Vec::new(),
        };

        let counts = module_name_counts_excluding(&report, &changed);
        assert_eq!(counts.get("tests"), Some(&1));
        assert_eq!(counts.get("unit_tests"), None);
    }

    #[test]
    fn parses_added_lines_from_zero_context_diff() {
        let patch = r#"diff --git src/a.rs src/a.rs
--- src/a.rs
+++ src/a.rs
@@ -10,0 +11,2 @@
+#[cfg(test)]
+mod special_tests {}
@@ -30 +32 @@
-old
+new
"#;

        let changed = parse_changed_lines(patch);
        assert!(changed.contains(Path::new("src/a.rs"), 11));
        assert!(changed.contains(Path::new("src/a.rs"), 12));
        assert!(changed.contains(Path::new("src/a.rs"), 32));
        assert!(!changed.contains(Path::new("src/a.rs"), 31));
    }

    #[test]
    fn selects_test_modules_whose_declaration_line_was_added() {
        let root = Path::new("/repo");
        let report = TestModuleReport {
            occurrences: vec![
                TestModuleOccurrence {
                    name: "tests".to_string(),
                    path: root.join("src/lib.rs"),
                    line: 20,
                },
                TestModuleOccurrence {
                    name: "special_tests".to_string(),
                    path: root.join("src/lib.rs"),
                    line: 40,
                },
            ],
            parse_failures: Vec::new(),
        };
        let mut changed = ChangedLines::default();
        changed.insert(Path::new("src/lib.rs"), 40);

        let selected = changed_test_modules(root, &report, &changed);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "special_tests");
    }
}

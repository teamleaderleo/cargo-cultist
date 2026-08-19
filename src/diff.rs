use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
#[cfg(test)]
use std::process::Command;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};
use crate::generated_diff::add_generated_companion_findings;
use crate::performance;
use crate::test_modules::{
    TestModuleOccurrence, TestModuleReport, analyze_test_module_files,
    analyze_test_modules_excluding,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct LineRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct ChangedLines {
    by_path: BTreeMap<PathBuf, Vec<LineRange>>,
}

impl ChangedLines {
    fn insert(&mut self, path: &Path, line: usize) {
        let ranges = self.by_path.entry(path.to_path_buf()).or_default();
        if let Some(last) = ranges.last_mut()
            && line <= last.end.saturating_add(1)
        {
            last.end = last.end.max(line);
            return;
        }
        ranges.push(LineRange {
            start: line,
            end: line,
        });
    }

    fn contains(&self, path: &Path, line: usize) -> bool {
        self.by_path.get(path).is_some_and(|ranges| {
            ranges
                .iter()
                .any(|range| range.start <= line && line <= range.end)
        })
    }

    fn rust_paths(&self) -> impl Iterator<Item = &Path> {
        self.by_path
            .keys()
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
            .map(PathBuf::as_path)
    }

    fn rust_file_count(&self) -> usize {
        self.rust_paths().count()
    }

    #[cfg(test)]
    fn range_count(&self, path: &Path) -> usize {
        self.by_path.get(path).map_or(0, Vec::len)
    }
}

pub fn git_repo_root(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let output = performance::git_command()
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
) -> Result<AnalysisReport, Box<dyn Error>> {
    let changed = git_diff_changed_lines(root, base)?;

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

    add_generated_companion_findings(root, base, &mut analysis)?;

    let changed_rust_paths: Vec<_> = changed.rust_paths().map(|path| root.join(path)).collect();
    if changed_rust_paths.is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "No added or renamed test-gated module declarations were found in the diff.",
        ));
        return Ok(analysis);
    }

    // Parse only changed Rust files first. Most diffs can stop here without
    // walking or parsing the rest of the repository.
    let changed_report = analyze_test_module_files(&changed_rust_paths)?;
    if !changed_report.parse_failures.is_empty() {
        for (path, error) in &changed_report.parse_failures {
            analysis.claims.push(
                Claim::new(
                    ClaimKind::Unknown,
                    "A changed Rust file could not be parsed, so diff relevance could not be determined.",
                )
                .with_evidence(Evidence::at(
                    error.clone(),
                    Location::new(
                        relative_path(root, path).to_string_lossy().into_owned(),
                        None,
                    ),
                )),
            );
        }
        return Ok(analysis);
    }

    if changed_test_modules(root, &changed_report, &changed).is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "No added or renamed test-gated module declarations were found in the diff.",
        ));
        return Ok(analysis);
    }

    // A relevant declaration needs repository precedent. Reuse the changed
    // files we already parsed and scan only the remaining Rust files.
    let excluded_paths: BTreeSet<_> = changed_rust_paths.into_iter().collect();
    let mut report = analyze_test_modules_excluding(root, &excluded_paths)?;
    report.extend(changed_report);

    add_diff_findings(root, &report, &changed, &mut analysis);
    Ok(analysis)
}

fn add_diff_findings(
    root: &Path,
    report: &TestModuleReport,
    changed: &ChangedLines,
    analysis: &mut AnalysisReport,
) {
    let changed_modules = changed_test_modules(root, report, changed);

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
}

fn git_diff_changed_lines(root: &Path, base: Option<&str>) -> Result<ChangedLines, Box<dyn Error>> {
    let anchor = match base {
        Some(base) => merge_base(root, base)?,
        None => "HEAD".to_string(),
    };

    let mut child = performance::git_command()
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
        .arg("*.rs")
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or("git diff did not provide a stdout pipe")?;
    let changed = parse_changed_lines(BufReader::new(stdout))?;
    let status = child.wait()?;

    if !status.success() {
        return Err(format!("git diff failed with status {status}").into());
    }

    Ok(changed)
}

fn merge_base(root: &Path, base: &str) -> Result<String, Box<dyn Error>> {
    let output = performance::git_command()
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

fn parse_changed_lines<R: BufRead>(mut reader: R) -> io::Result<ChangedLines> {
    let mut changed = ChangedLines::default();
    let mut current_path: Option<PathBuf> = None;
    let mut current_new_line: Option<usize> = None;
    let mut buffer = String::new();

    while reader.read_line(&mut buffer)? != 0 {
        let line = buffer.trim_end_matches(['\n', '\r']);

        if let Some(path) = line.strip_prefix("+++ ") {
            current_path = (path != "/dev/null").then(|| PathBuf::from(path));
            current_new_line = None;
            buffer.clear();
            continue;
        }

        if line.starts_with("@@") {
            current_new_line = hunk_new_start(line);
            buffer.clear();
            continue;
        }

        let Some(new_line) = current_new_line else {
            buffer.clear();
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

        buffer.clear();
    }

    Ok(changed)
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

fn relative_path<'a>(root: &'a Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cargo-cultist-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn run_git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: git {args:?}");
    }

    fn init_repo(name: &str) -> PathBuf {
        let root = unique_temp_dir(name);
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);
        fs::write(root.join("README.md"), "baseline\n").unwrap();
        fs::write(root.join("changed.rs"), "fn baseline() {}\n").unwrap();
        fs::write(root.join("unrelated.rs"), [0xff, 0xfe, 0xfd]).unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);
        root
    }

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

        let changed = parse_changed_lines(patch.as_bytes()).unwrap();
        assert!(changed.contains(Path::new("src/a.rs"), 11));
        assert!(changed.contains(Path::new("src/a.rs"), 12));
        assert!(changed.contains(Path::new("src/a.rs"), 32));
        assert!(!changed.contains(Path::new("src/a.rs"), 31));
        assert_eq!(changed.range_count(Path::new("src/a.rs")), 2);
    }

    #[test]
    fn compacts_contiguous_added_lines_into_one_range() {
        let patch = r#"diff --git src/a.rs src/a.rs
--- src/a.rs
+++ src/a.rs
@@ -10,0 +11,4 @@
+one
+two
+three
+four
"#;

        let changed = parse_changed_lines(patch.as_bytes()).unwrap();
        assert_eq!(changed.range_count(Path::new("src/a.rs")), 1);
        assert!(changed.contains(Path::new("src/a.rs"), 11));
        assert!(changed.contains(Path::new("src/a.rs"), 14));
        assert!(!changed.contains(Path::new("src/a.rs"), 15));
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

    #[test]
    fn docs_only_diff_skips_repository_rust_scan() {
        let root = init_repo("docs-only-diff");
        fs::write(root.join("README.md"), "changed docs\n").unwrap();

        let analysis = build_diff_analysis_report(&root, None).unwrap();

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim
                .message
                .contains("No added or renamed test-gated module declarations")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn irrelevant_rust_diff_skips_unrelated_repository_rust_scan() {
        let root = init_repo("irrelevant-rust-diff");
        fs::write(
            root.join("changed.rs"),
            "fn changed() { println!(\"hi\"); }\n",
        )
        .unwrap();

        let analysis = build_diff_analysis_report(&root, None).unwrap();

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim
                .message
                .contains("No added or renamed test-gated module declarations")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn changed_rust_parse_failure_stays_unknown_without_repository_scan() {
        let root = init_repo("changed-parse-failure");
        fs::write(root.join("changed.rs"), "fn changed( {\n").unwrap();

        let analysis = build_diff_analysis_report(&root, None).unwrap();

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Unknown
                && claim
                    .message
                    .contains("diff relevance could not be determined")
        }));
        assert!(!analysis.claims.iter().any(|claim| {
            claim.kind == ClaimKind::Observed
                && claim
                    .message
                    .contains("No added or renamed test-gated module declarations")
        }));
        fs::remove_dir_all(root).unwrap();
    }
}

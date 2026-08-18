use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

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

    let counts = module_name_counts(report);
    let mut finding_count = 0;

    for occurrence in changed_modules {
        let local_names = same_file_names(report, occurrence);
        let different_local_names: Vec<_> = local_names
            .iter()
            .filter(|name| name.as_str() != occurrence.name)
            .collect();
        let repository_count = counts.get(&occurrence.name).copied().unwrap_or_default();
        let one_off = repository_count == 1 && report.occurrences.len() > 1;

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
            "  `{}` appears {} time(s) across {} test-gated modules.",
            occurrence.name,
            repository_count,
            report.occurrences.len()
        );
        print_top_counts(&counts);

        if !different_local_names.is_empty() {
            println!(
                "  The same file also uses: {}.",
                different_local_names
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        println!("\nOBSERVATION");
        if !different_local_names.is_empty() && one_off {
            println!(
                "  The new name differs from this file's existing precedent and is unique in the repository."
            );
        } else if !different_local_names.is_empty() {
            println!("  The new name differs from this file's existing test-module precedent.");
        } else {
            println!("  The new name is unique among the repository's test-gated modules.");
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

fn module_name_counts(report: &TestModuleReport) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for occurrence in &report.occurrences {
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

fn print_top_counts(counts: &BTreeMap<String, usize>) {
    let mut counts: Vec<_> = counts.iter().collect();
    counts.sort_by(|(name_a, count_a), (name_b, count_b)| {
        count_b.cmp(count_a).then(name_a.cmp(name_b))
    });

    let summary = counts
        .into_iter()
        .take(5)
        .map(|(name, count)| format!("`{name}`={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    println!("  Repository counts: {summary}.");
}

fn relative_path<'a>(root: &'a Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

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

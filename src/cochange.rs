use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};

pub const DEFAULT_HISTORY_LIMIT: usize = 100;
pub const DEFAULT_MAX_FILES_PER_COMMIT: usize = 40;
const MAX_REPORTED_COMPANIONS: usize = 20;
const MAX_EXAMPLE_COMMITS: usize = 3;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CochangeCompanion {
    pub path: String,
    pub support: usize,
    pub opportunities: usize,
    pub exemplars: Vec<String>,
    pub counterexamples: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CochangeReport {
    pub target: String,
    pub considered_commits: usize,
    pub broad_commits_skipped: usize,
    pub companions: Vec<CochangeCompanion>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HistoryCommit {
    sha: String,
    paths: BTreeSet<String>,
}

pub fn analyze_cochange(
    root: &Path,
    target: &Path,
    history_limit: usize,
    max_files_per_commit: usize,
) -> Result<CochangeReport, Box<dyn Error>> {
    let target = normalize_target(root, target)?;
    let shas = history_shas(root, &target, history_limit)?;
    let mut commits = Vec::with_capacity(shas.len());

    for sha in shas {
        commits.push(HistoryCommit {
            paths: commit_paths(root, &sha)?,
            sha,
        });
    }

    Ok(summarize_history(
        &target,
        &commits,
        max_files_per_commit,
    ))
}

pub fn build_cochange_analysis_report(root: &Path, report: &CochangeReport) -> AnalysisReport {
    let mut analysis =
        AnalysisReport::new("historical-cochange", root.to_string_lossy().into_owned());

    analysis.claims.push(Claim::new(
        ClaimKind::Derived,
        format!(
            "The history query kept {} focused non-merge commit(s) touching `{}` and skipped {} broad commit(s).",
            report.considered_commits, report.target, report.broad_commits_skipped
        ),
    ));
    analysis.claims.push(Claim::new(
        ClaimKind::Unknown,
        "Historical co-change frequency alone does not establish that two paths must change together.",
    ));

    for companion in &report.companions {
        let percent = support_percent(companion.support, companion.opportunities);
        let mut claim = Claim::new(
            ClaimKind::Observed,
            format!(
                "`{}` changed with `{}` in {} of {} focused commit(s) ({percent:.1}%).",
                report.target, companion.path, companion.support, companion.opportunities
            ),
        )
        .with_evidence(Evidence::at(
            format!("Historical companion path: `{}`.", companion.path),
            Location::new(companion.path.clone(), None),
        ));

        if !companion.exemplars.is_empty() {
            claim = claim.with_evidence(Evidence::new(format!(
                "Exemplar commit(s): {}.",
                companion.exemplars.join(", ")
            )));
        }
        if !companion.counterexamples.is_empty() {
            claim = claim.with_evidence(Evidence::new(format!(
                "Counterexample commit(s): {}.",
                companion.counterexamples.join(", ")
            )));
        }

        analysis.findings.push(
            Finding::new("historical-cochange-precedent", "Historical co-change precedent")
                .at(Location::new(report.target.clone(), None))
                .with_claim(claim)
                .with_claim(Claim::new(
                    ClaimKind::Unknown,
                    "The repository history does not by itself explain whether this companion is required, generated, incidental, or obsolete.",
                ))
                .with_question(format!(
                    "When `{}` changes, is `{}` an expected companion for the same reason as the historical examples?",
                    report.target, companion.path
                )),
        );
    }

    if report.companions.is_empty() {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "No historical companion paths were found in the focused commit cohort.",
        ));
    }

    analysis
}

pub fn print_cochange_report(report: &CochangeReport) {
    println!("HISTORICAL CO-CHANGE");
    println!("  target                  {}", report.target);
    println!("  focused commits         {}", report.considered_commits);
    println!(
        "  broad commits skipped   {}",
        report.broad_commits_skipped
    );

    if report.companions.is_empty() {
        println!("\n  No companion paths found in the focused history.");
        return;
    }

    println!("\nCOMPANIONS");
    for companion in &report.companions {
        let percent = support_percent(companion.support, companion.opportunities);
        println!(
            "  {:>3}/{:<3} {:>5.1}%  {}",
            companion.support, companion.opportunities, percent, companion.path
        );
        if !companion.exemplars.is_empty() {
            println!("      exemplars: {}", companion.exemplars.join(", "));
        }
        if !companion.counterexamples.is_empty() {
            println!(
                "      counterexamples: {}",
                companion.counterexamples.join(", ")
            );
        }
    }

    println!("\nQUESTION");
    println!(
        "  Which companions reflect a durable project convention, and which are incidental history?"
    );
}

fn normalize_target(root: &Path, target: &Path) -> Result<String, Box<dyn Error>> {
    let absolute = if target.is_absolute() {
        target.canonicalize()?
    } else {
        root.join(target).canonicalize()?
    };
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("target `{}` is outside the repository", absolute.display()))?;

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn history_shas(
    root: &Path,
    target: &str,
    history_limit: usize,
) -> Result<Vec<String>, Box<dyn Error>> {
    let args = vec![
        "log".to_string(),
        "--no-merges".to_string(),
        format!("--max-count={history_limit}"),
        "--format=%H".to_string(),
        "--".to_string(),
        target.to_string(),
    ];
    let output = run_git(root, &args)?;
    Ok(String::from_utf8_lossy(&output)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn commit_paths(root: &Path, sha: &str) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let args = vec![
        "diff-tree".to_string(),
        "--root".to_string(),
        "--no-commit-id".to_string(),
        "--name-only".to_string(),
        "-r".to_string(),
        "-z".to_string(),
        sha.to_string(),
    ];
    let output = run_git(root, &args)?;

    Ok(output
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect())
}

fn run_git(root: &Path, args: &[String]) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "git command failed: git -C {} {}\n{}",
            root.display(),
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    Ok(output.stdout)
}

fn summarize_history(
    target: &str,
    commits: &[HistoryCommit],
    max_files_per_commit: usize,
) -> CochangeReport {
    let mut focused = Vec::new();
    let mut broad_commits_skipped = 0;

    for commit in commits {
        if commit.paths.len() > max_files_per_commit {
            broad_commits_skipped += 1;
        } else {
            focused.push(commit);
        }
    }

    let opportunities = focused.len();
    let mut counts = BTreeMap::<String, usize>::new();

    for commit in &focused {
        for path in &commit.paths {
            if path != target {
                *counts.entry(path.clone()).or_default() += 1;
            }
        }
    }

    let mut companions: Vec<_> = counts
        .into_iter()
        .map(|(path, support)| {
            let exemplars = focused
                .iter()
                .filter(|commit| commit.paths.contains(&path))
                .take(MAX_EXAMPLE_COMMITS)
                .map(|commit| short_sha(&commit.sha))
                .collect();
            let counterexamples = focused
                .iter()
                .filter(|commit| !commit.paths.contains(&path))
                .take(MAX_EXAMPLE_COMMITS)
                .map(|commit| short_sha(&commit.sha))
                .collect();

            CochangeCompanion {
                path,
                support,
                opportunities,
                exemplars,
                counterexamples,
            }
        })
        .collect();

    companions.sort_by(|a, b| b.support.cmp(&a.support).then(a.path.cmp(&b.path)));
    companions.truncate(MAX_REPORTED_COMPANIONS);

    CochangeReport {
        target: target.to_string(),
        considered_commits: opportunities,
        broad_commits_skipped,
        companions,
    }
}

fn support_percent(support: usize, opportunities: usize) -> f64 {
    if opportunities == 0 {
        0.0
    } else {
        support as f64 * 100.0 / opportunities as f64
    }
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(sha: &str, paths: &[&str]) -> HistoryCommit {
        HistoryCommit {
            sha: sha.to_string(),
            paths: paths.iter().map(|path| (*path).to_string()).collect(),
        }
    }

    #[test]
    fn ranks_companions_and_keeps_counterexamples() {
        let commits = vec![
            commit("aaaaaaaa1111", &["src/main.rs", "src/a.rs", "src/b.rs"]),
            commit("bbbbbbbb2222", &["src/main.rs", "src/a.rs"]),
            commit("cccccccc3333", &["src/main.rs", "src/a.rs"]),
            commit("dddddddd4444", &["src/main.rs", "src/b.rs"]),
        ];

        let report = summarize_history("src/main.rs", &commits, 10);

        assert_eq!(report.considered_commits, 4);
        assert_eq!(report.broad_commits_skipped, 0);
        assert_eq!(report.companions[0].path, "src/a.rs");
        assert_eq!(report.companions[0].support, 3);
        assert_eq!(report.companions[0].opportunities, 4);
        assert_eq!(
            report.companions[0].counterexamples,
            vec!["dddddddd".to_string()]
        );
        assert_eq!(report.companions[1].path, "src/b.rs");
        assert_eq!(report.companions[1].support, 2);
    }

    #[test]
    fn filters_broad_history_before_counting() {
        let commits = vec![
            commit("aaaaaaaa1111", &["src/main.rs", "src/a.rs"]),
            commit(
                "bbbbbbbb2222",
                &["src/main.rs", "src/a.rs", "src/b.rs", "src/c.rs"],
            ),
        ];

        let report = summarize_history("src/main.rs", &commits, 3);

        assert_eq!(report.considered_commits, 1);
        assert_eq!(report.broad_commits_skipped, 1);
        assert_eq!(report.companions.len(), 1);
        assert_eq!(report.companions[0].path, "src/a.rs");
        assert_eq!(report.companions[0].support, 1);
        assert_eq!(report.companions[0].opportunities, 1);
    }
}

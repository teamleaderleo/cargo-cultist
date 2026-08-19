use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::performance;

pub const HISTORY_REPORT_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_MAX_COMMITS: usize = 100;
const DEFAULT_MAX_PATHS_PER_COMMIT: usize = 100;
const DEFAULT_TOP_COMPANIONS: usize = 15;
const EXAMPLE_LIMIT: usize = 3;
const GENERATED_HEADER_BYTES: u64 = 8 * 1024;
const GENERATED_HEADER_LINES: usize = 40;
const GENERATED_MARKERS: &[&str] = &[
    "@generated",
    "do not edit",
    "automatically generated",
    "auto-generated",
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct HistoryOptions {
    pub max_commits: usize,
    pub max_paths_per_commit: usize,
}

impl Default for HistoryOptions {
    fn default() -> Self {
        Self {
            max_commits: DEFAULT_MAX_COMMITS,
            max_paths_per_commit: DEFAULT_MAX_PATHS_PER_COMMIT,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CommitSummary {
    pub sha: String,
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ExcludedCommit {
    #[serde(flatten)]
    pub commit: CommitSummary,
    pub reason: String,
    pub changed_paths: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct GeneratedMarkerEvidence {
    pub path: String,
    pub line: usize,
    pub marker: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoricalCompanion {
    pub path: String,
    pub support: usize,
    pub opportunities: usize,
    pub support_percent: f64,
    pub examples: Vec<CommitSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counterexamples: Vec<CommitSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_marker: Option<GeneratedMarkerEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryReport {
    pub schema_version: u32,
    pub analysis: String,
    pub repository: String,
    pub anchor: String,
    pub requested_max_commits: usize,
    pub discovered_commits: usize,
    pub considered_commits: usize,
    pub broad_commit_threshold: usize,
    pub excluded_commits: Vec<ExcludedCommit>,
    pub companions: Vec<HistoricalCompanion>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HistoricalCommit {
    summary: CommitSummary,
    paths: Vec<PathBuf>,
}

pub fn analyze_historical_companions(
    root: &Path,
    anchor: &Path,
    options: HistoryOptions,
) -> Result<HistoryReport, Box<dyn Error>> {
    if anchor.is_absolute() {
        return Err("history anchor must be repository-relative".into());
    }
    if options.max_commits == 0 {
        return Err("history max commit count must be greater than zero".into());
    }
    if options.max_paths_per_commit == 0 {
        return Err("history broad-commit threshold must be greater than zero".into());
    }

    let commits = read_anchor_history(root, anchor, options.max_commits)?;
    let discovered_commits = commits.len();
    let mut considered = Vec::new();
    let mut excluded_commits = Vec::new();

    for commit in commits {
        if is_revert_subject(&commit.summary.subject) {
            excluded_commits.push(ExcludedCommit {
                commit: commit.summary,
                reason: "revert commit".to_string(),
                changed_paths: commit.paths.len(),
            });
            continue;
        }
        if commit.paths.len() > options.max_paths_per_commit {
            excluded_commits.push(ExcludedCommit {
                commit: commit.summary,
                reason: format!(
                    "broad commit changed more than {} paths",
                    options.max_paths_per_commit
                ),
                changed_paths: commit.paths.len(),
            });
            continue;
        }
        considered.push(commit);
    }

    let mut companions = build_companions(anchor, &considered);
    annotate_generated_markers(root, &mut companions);

    Ok(HistoryReport {
        schema_version: HISTORY_REPORT_SCHEMA_VERSION,
        analysis: "historical_companions".to_string(),
        repository: root.display().to_string(),
        anchor: anchor.display().to_string(),
        requested_max_commits: options.max_commits,
        discovered_commits,
        considered_commits: considered.len(),
        broad_commit_threshold: options.max_paths_per_commit,
        excluded_commits,
        companions,
        limitations: vec![
            "Rename history uses the current path identity only.".to_string(),
            "The first cohort filter removes reverts and broad commits; subsystem and semantic cohort selection remain research work.".to_string(),
            "Co-change is correlation evidence. The explorer reports association without assigning correctness or intent.".to_string(),
            "Generated-marker evidence only reports self-identification found in a bounded current-file header; it does not establish source ownership, generator commands, derivation direction, or a regeneration requirement.".to_string(),
        ],
    })
}

pub fn print_history_report(report: &HistoryReport) {
    println!("HISTORICAL COMPANIONS");
    println!("  anchor: {}", report.anchor);
    println!(
        "  cohort: {} considered commit(s) from {} discovered non-merge commit(s)",
        report.considered_commits, report.discovered_commits
    );
    println!(
        "  broad-commit threshold: {} changed paths",
        report.broad_commit_threshold
    );

    if report.considered_commits == 0 {
        println!("\nOBSERVATION");
        println!("  No history remained after the current cohort filters.");
        print_excluded(report);
        print_limitations(report);
        return;
    }

    if report.companions.is_empty() {
        println!("\nOBSERVATION");
        println!("  The considered commits contain no recurring companion paths.");
        print_excluded(report);
        print_limitations(report);
        return;
    }

    println!("\nCOMPANIONS");
    for companion in report.companions.iter().take(DEFAULT_TOP_COMPANIONS) {
        println!(
            "  {:<56} {:>3}/{:<3} {:>5.1}%",
            companion.path, companion.support, companion.opportunities, companion.support_percent
        );
        if let Some(marker) = &companion.generated_marker {
            println!(
                "    generated marker {}:{}  {}",
                marker.path, marker.line, marker.marker
            );
        }
        for example in &companion.examples {
            println!(
                "    example {}  {}  {}",
                short_sha(&example.sha),
                example.date,
                example.subject
            );
        }
    }

    let with_counterexamples: Vec<_> = report
        .companions
        .iter()
        .take(5)
        .filter(|companion| !companion.counterexamples.is_empty())
        .collect();

    if !with_counterexamples.is_empty() {
        println!("\nCOUNTEREXAMPLE SAMPLE");
        for companion in with_counterexamples {
            println!("  {}", companion.path);
            for example in &companion.counterexamples {
                println!(
                    "    absent {}  {}  {}",
                    short_sha(&example.sha),
                    example.date,
                    example.subject
                );
            }
        }
    }

    println!("\nOBSERVATION");
    println!(
        "  These are historical co-change associations for the current path, before semantic cohort selection or finding thresholds."
    );
    println!("\nQUESTION");
    println!(
        "  Which of these companions represent a repository custom worth comparing against a future diff?"
    );

    print_excluded(report);
    print_limitations(report);
}

fn print_excluded(report: &HistoryReport) {
    if report.excluded_commits.is_empty() {
        return;
    }

    println!("\nEXCLUDED COMMITS");
    for excluded in report.excluded_commits.iter().take(10) {
        println!(
            "  {}  {} path(s)  {}  {}",
            short_sha(&excluded.commit.sha),
            excluded.changed_paths,
            excluded.reason,
            excluded.commit.subject
        );
    }
    if report.excluded_commits.len() > 10 {
        println!(
            "  ... {} more excluded commit(s)",
            report.excluded_commits.len() - 10
        );
    }
}

fn print_limitations(report: &HistoryReport) {
    println!("\nLIMITATIONS");
    for limitation in &report.limitations {
        println!("  - {limitation}");
    }
}

fn read_anchor_history(
    root: &Path,
    anchor: &Path,
    max_commits: usize,
) -> Result<Vec<HistoricalCommit>, Box<dyn Error>> {
    // `--full-diff` keeps the pathspec as the commit selector while making
    // `--name-only` report each selected commit's complete change set. This
    // lets one Git process replace the previous log + one-show-per-commit loop.
    let output = performance::git_command()
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "log",
            "--format=%x1e%H%x1f%cI%x1f%s",
            "--name-only",
            "--no-renames",
            "--no-color",
            "--no-ext-diff",
            "--root",
            "--no-merges",
            "--full-diff",
            "-n",
        ])
        .arg(max_commits.to_string())
        .arg("--")
        .arg(anchor)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log failed for {}: {stderr}", anchor.display()).into());
    }

    parse_history_log(&String::from_utf8(output.stdout)?)
        .ok_or_else(|| format!("could not parse git history for {}", anchor.display()).into())
}

fn parse_history_log(output: &str) -> Option<Vec<HistoricalCommit>> {
    output
        .split('\u{1e}')
        .filter(|record| !record.trim().is_empty())
        .map(parse_history_record)
        .collect()
}

fn parse_history_record(record: &str) -> Option<HistoricalCommit> {
    let record = record.trim_start_matches(['\n', '\r']);
    let mut lines = record.lines();
    let metadata = lines.next()?.trim();
    let mut fields = metadata.splitn(3, '\u{1f}');
    let sha = fields.next()?.trim().to_string();
    let date = fields.next()?.trim().to_string();
    let subject = fields.next()?.trim().to_string();

    let paths = lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Some(HistoricalCommit {
        summary: CommitSummary { sha, date, subject },
        paths,
    })
}

fn build_companions(anchor: &Path, commits: &[HistoricalCommit]) -> Vec<HistoricalCompanion> {
    let mut support = BTreeMap::<PathBuf, Vec<usize>>::new();

    for (commit_index, commit) in commits.iter().enumerate() {
        for path in &commit.paths {
            if path == anchor {
                continue;
            }
            support.entry(path.clone()).or_default().push(commit_index);
        }
    }

    let opportunities = commits.len();
    let mut companions: Vec<_> = support
        .into_iter()
        .map(|(path, present_in)| {
            let present: BTreeSet<_> = present_in.iter().copied().collect();
            let examples = present_in
                .iter()
                .take(EXAMPLE_LIMIT)
                .map(|index| commits[*index].summary.clone())
                .collect();
            let counterexamples = (0..commits.len())
                .filter(|index| !present.contains(index))
                .take(EXAMPLE_LIMIT)
                .map(|index| commits[index].summary.clone())
                .collect();
            let support_count = present_in.len();
            let support_percent = if opportunities == 0 {
                0.0
            } else {
                (support_count as f64 / opportunities as f64 * 1000.0).round() / 10.0
            };

            HistoricalCompanion {
                path: path.display().to_string(),
                support: support_count,
                opportunities,
                support_percent,
                examples,
                counterexamples,
                generated_marker: None,
            }
        })
        .collect();

    companions.sort_by(|a, b| b.support.cmp(&a.support).then_with(|| a.path.cmp(&b.path)));
    companions
}

fn annotate_generated_markers(root: &Path, companions: &mut [HistoricalCompanion]) {
    for companion in companions {
        let relative = Path::new(&companion.path);
        companion.generated_marker = generated_marker_for_path(root, relative);
    }
}

fn generated_marker_for_path(root: &Path, relative: &Path) -> Option<GeneratedMarkerEvidence> {
    let path = root.join(relative);
    if !path.is_file() {
        return None;
    }

    let file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(GENERATED_HEADER_BYTES)
        .read_to_end(&mut bytes)
        .ok()?;
    let header = String::from_utf8_lossy(&bytes);
    generated_marker_from_header(relative, &header)
}

fn generated_marker_from_header(relative: &Path, header: &str) -> Option<GeneratedMarkerEvidence> {
    for (index, line) in header.lines().take(GENERATED_HEADER_LINES).enumerate() {
        let normalized = line.to_ascii_lowercase();
        if GENERATED_MARKERS
            .iter()
            .any(|marker| normalized.contains(marker))
        {
            return Some(GeneratedMarkerEvidence {
                path: relative.display().to_string(),
                line: index + 1,
                marker: line.trim().to_string(),
            });
        }
    }

    None
}

fn is_revert_subject(subject: &str) -> bool {
    subject
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("revert")
}

fn short_sha(sha: &str) -> &str {
    sha.get(..sha.len().min(8)).unwrap_or(sha)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn commit(sha: &str, subject: &str, paths: &[&str]) -> HistoricalCommit {
        HistoricalCommit {
            summary: CommitSummary {
                sha: sha.to_string(),
                date: "2026-08-18T12:00:00Z".to_string(),
                subject: subject.to_string(),
            },
            paths: paths.iter().map(PathBuf::from).collect(),
        }
    }

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

    #[test]
    fn parses_batched_git_log_records() {
        let output = concat!(
            "\x1eabcdef\x1f2026-08-18T12:00:00Z\x1ffeat: example\n\n",
            "src/a.rs\n",
            "tests/a.rs\n",
            "\x1e123456\x1f2026-08-17T12:00:00Z\x1ffix: another\n\n",
            "src/a.rs\n",
        );
        let parsed = parse_history_log(output).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].summary.sha, "abcdef");
        assert_eq!(parsed[0].summary.subject, "feat: example");
        assert_eq!(
            parsed[0].paths,
            vec![PathBuf::from("src/a.rs"), PathBuf::from("tests/a.rs")]
        );
        assert_eq!(parsed[1].summary.sha, "123456");
        assert_eq!(parsed[1].paths, vec![PathBuf::from("src/a.rs")]);
    }

    #[test]
    fn path_filtered_history_keeps_full_commit_change_sets() {
        let root = unique_temp_dir("batched-history");
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);

        fs::write(root.join("anchor.rs"), "fn anchor() {}\n").unwrap();
        fs::write(root.join("companion.rs"), "fn companion() {}\n").unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);

        fs::write(root.join("anchor.rs"), "fn anchor_changed() {}\n").unwrap();
        fs::write(root.join("companion.rs"), "fn companion_changed() {}\n").unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "paired change"]);

        let commits = read_anchor_history(&root, Path::new("anchor.rs"), 10).unwrap();
        let paired = commits
            .iter()
            .find(|commit| commit.summary.subject == "paired change")
            .unwrap();
        assert!(paired.paths.contains(&PathBuf::from("anchor.rs")));
        assert!(paired.paths.contains(&PathBuf::from("companion.rs")));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn counts_companions_and_retains_counterexamples() {
        let commits = vec![
            commit("a", "one", &["src/a.rs", "tests/a.rs", "generated/a.rs"]),
            commit("b", "two", &["src/a.rs", "tests/a.rs"]),
            commit("c", "three", &["src/a.rs", "tests/a.rs", "generated/a.rs"]),
        ];

        let companions = build_companions(Path::new("src/a.rs"), &commits);
        assert_eq!(companions[0].path, "tests/a.rs");
        assert_eq!(companions[0].support, 3);
        assert_eq!(companions[0].support_percent, 100.0);
        assert!(companions[0].counterexamples.is_empty());
        assert!(companions[0].generated_marker.is_none());

        assert_eq!(companions[1].path, "generated/a.rs");
        assert_eq!(companions[1].support, 2);
        assert_eq!(companions[1].support_percent, 66.7);
        assert_eq!(companions[1].counterexamples.len(), 1);
        assert_eq!(companions[1].counterexamples[0].sha, "b");
        assert!(companions[1].generated_marker.is_none());
    }

    #[test]
    fn sorts_equal_support_by_path() {
        let commits = vec![commit("a", "one", &["src/a.rs", "z.rs", "b.rs"])];
        let companions = build_companions(Path::new("src/a.rs"), &commits);
        assert_eq!(companions[0].path, "b.rs");
        assert_eq!(companions[1].path, "z.rs");
    }

    #[test]
    fn recognizes_generated_marker_in_header() {
        let evidence = generated_marker_from_header(
            Path::new("generated/registry.rs"),
            "// @generated by cargo xtask\n\npub enum Rule {}\n",
        )
        .unwrap();

        assert_eq!(evidence.path, "generated/registry.rs");
        assert_eq!(evidence.line, 1);
        assert_eq!(evidence.marker, "// @generated by cargo xtask");
    }

    #[test]
    fn keeps_generic_generated_words_outside_marker_vocabulary() {
        assert!(
            generated_marker_from_header(
                Path::new("src/parser.rs"),
                "// Parse generated values from the wire.\n"
            )
            .is_none()
        );
        assert!(
            generated_marker_from_header(
                Path::new("src/report.rs"),
                "// This value is generated by the parser at runtime.\n"
            )
            .is_none()
        );
    }

    #[test]
    fn generated_marker_scan_is_line_bounded() {
        let mut header = (0..GENERATED_HEADER_LINES)
            .map(|_| "// ordinary header line")
            .collect::<Vec<_>>()
            .join("\n");
        header.push_str("\n// @generated too late\n");

        assert!(generated_marker_from_header(Path::new("late.rs"), &header).is_none());
    }

    #[test]
    fn recognizes_revert_subjects() {
        assert!(is_revert_subject("Revert \"feat: thing\""));
        assert!(is_revert_subject("revert: temporary change"));
        assert!(!is_revert_subject("feat: ordinary change"));
    }
}

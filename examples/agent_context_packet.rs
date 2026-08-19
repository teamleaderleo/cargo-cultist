use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

#[path = "decision_memory.rs"]
mod decision_memory;

const SCHEMA_VERSION: u32 = 3;
const DEFAULT_MAX_HISTORY: usize = 20;
const DEFAULT_MAX_COMPANIONS: usize = 8;
const DEFAULT_MAX_EXAMPLES: usize = 2;
const MAX_PATHS_PER_COMPANION_COMMIT: usize = 100;

#[derive(Debug, Clone, Copy, Serialize)]
struct PacketBudget {
    max_history_commits: usize,
    max_companions: usize,
    max_examples_per_relation: usize,
    max_paths_per_companion_commit: usize,
}

impl Default for PacketBudget {
    fn default() -> Self {
        Self {
            max_history_commits: DEFAULT_MAX_HISTORY,
            max_companions: DEFAULT_MAX_COMPANIONS,
            max_examples_per_relation: DEFAULT_MAX_EXAMPLES,
            max_paths_per_companion_commit: MAX_PATHS_PER_COMPANION_COMMIT,
        }
    }
}

#[derive(Debug, Serialize)]
struct AgentContextPacket {
    schema_version: u32,
    analysis: &'static str,
    repository: String,
    target: PacketTarget,
    budget: PacketBudget,
    serialized_bytes: usize,
    direct_evidence: Vec<EvidenceItem>,
    guidance: Vec<GuidanceFile>,
    reviewed_decisions: Vec<decision_memory::ResolvedDecision>,
    recent_history: Vec<CommitSummary>,
    historical_companions: Vec<HistoricalCompanion>,
    companion_exclusions: Vec<ExcludedCommit>,
    unknowns: Vec<String>,
    truncation: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PacketTarget {
    path: String,
}

#[derive(Debug, Serialize)]
struct EvidenceItem {
    claim_kind: &'static str,
    message: String,
    source: EvidenceSource,
}

#[derive(Debug, Serialize)]
struct EvidenceSource {
    kind: &'static str,
    path: String,
}

#[derive(Debug, Serialize)]
struct GuidanceFile {
    path: String,
    scope: String,
    guidance_kind: String,
}

#[derive(Debug, Clone, Serialize)]
struct CommitSummary {
    sha: String,
    date: String,
    subject: String,
}

#[derive(Debug, Serialize)]
struct ExcludedCommit {
    commit: CommitSummary,
    reason: String,
    changed_paths: usize,
}

#[derive(Debug, Serialize)]
struct HistoricalCompanion {
    path: String,
    support: usize,
    opportunities: usize,
    support_percent: f64,
    examples: Vec<CommitSummary>,
    examples_omitted: usize,
    counterexamples: Vec<CommitSummary>,
    counterexamples_omitted: usize,
}

#[derive(Debug)]
struct HistoricalCommit {
    summary: CommitSummary,
    paths: Vec<PathBuf>,
}

#[derive(Debug)]
struct CompanionAnalysis {
    companions: Vec<HistoricalCompanion>,
    omitted_companions: usize,
    exclusions: Vec<ExcludedCommit>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("agent-context-packet: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let target_arg = env::args()
        .nth(1)
        .ok_or("usage: cargo run --example agent_context_packet -- FILE")?;

    let requested = PathBuf::from(target_arg);
    let requested = if requested.is_absolute() {
        requested
    } else {
        env::current_dir()?.join(requested)
    };
    let target = requested.canonicalize()?;
    if !target.is_file() {
        return Err(format!("target must be an existing file: {}", target.display()).into());
    }

    let probe = target
        .parent()
        .ok_or("could not determine target parent directory")?;
    let root = git_repo_root(probe)?;
    let relative_target = target
        .strip_prefix(&root)
        .map_err(|_| "target is outside the resolved Git repository")?
        .to_path_buf();

    let budget = PacketBudget::default();
    let guidance = applicable_guidance(&root, &target)?;
    let reviewed_decisions =
        decision_memory::resolve_repository_decisions(&root, &relative_target)?;
    let (recent_history, history_truncated) =
        recent_commits(&root, &relative_target, budget.max_history_commits)?;
    let CompanionAnalysis {
        companions: historical_companions,
        omitted_companions,
        exclusions: companion_exclusions,
    } = historical_companions(&root, &relative_target, &recent_history, budget)?;

    let mut truncation = Vec::new();
    if history_truncated {
        truncation.push(format!(
            "Recent history is limited to the newest {} non-merge commits touching the target.",
            budget.max_history_commits
        ));
    }
    if omitted_companions > 0 {
        truncation.push(format!(
            "{omitted_companions} additional historical companion path(s) were omitted by max_companions={}.",
            budget.max_companions
        ));
    }
    let relations_with_omitted_examples = historical_companions
        .iter()
        .filter(|companion| companion.examples_omitted > 0 || companion.counterexamples_omitted > 0)
        .count();
    if relations_with_omitted_examples > 0 {
        truncation.push(format!(
            "{relations_with_omitted_examples} returned companion relation(s) have omitted examples or counterexamples; exact omission counts are recorded on each relation."
        ));
    }

    let mut packet = AgentContextPacket {
        schema_version: SCHEMA_VERSION,
        analysis: "agent_context",
        repository: root.display().to_string(),
        target: PacketTarget {
            path: relative_target.display().to_string(),
        },
        budget,
        serialized_bytes: 0,
        direct_evidence: vec![EvidenceItem {
            claim_kind: "proven",
            message: "The target resolves to this repository-relative file identity.".to_string(),
            source: EvidenceSource {
                kind: "filesystem",
                path: relative_target.display().to_string(),
            },
        }],
        guidance,
        reviewed_decisions,
        recent_history,
        historical_companions,
        companion_exclusions,
        unknowns: vec![
            "Applicable guidance files are surfaced as source artifacts; this research example does not interpret their natural-language rules.".to_string(),
            "Repository decision-memory records are surfaced as a distinct evidence layer; this packet does not yet encode whether a record was merely proposed, reviewed, or merged.".to_string(),
            "Remote pull request, issue, and review rationale outside repository decision memory is unavailable in this local-only packet.".to_string(),
            "Chronological proximity between commits is not evidence that one change caused another.".to_string(),
            "Current Cargo Cultist analyzer findings are not yet composed into this standalone research example.".to_string(),
            "This v3 research packet measures serialized bytes but does not yet enforce a hard byte budget or eviction policy.".to_string(),
        ],
        truncation,
    };

    println!("{}", render_packet_with_exact_size(&mut packet)?);
    Ok(())
}

fn render_packet_with_exact_size(
    packet: &mut AgentContextPacket,
) -> Result<String, serde_json::Error> {
    loop {
        let rendered = serde_json::to_string_pretty(packet)?;
        let serialized_bytes = rendered.len();
        if packet.serialized_bytes == serialized_bytes {
            return Ok(rendered);
        }
        packet.serialized_bytes = serialized_bytes;
    }
}

fn git_repo_root(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} is not inside a Git repository: {stderr}",
            path.display()
        )
        .into());
    }

    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()).canonicalize()?)
}

fn applicable_guidance(root: &Path, target: &Path) -> Result<Vec<GuidanceFile>, Box<dyn Error>> {
    let mut directories = Vec::new();
    let mut current = target.parent();

    while let Some(directory) = current {
        if !directory.starts_with(root) {
            break;
        }
        directories.push(directory.to_path_buf());
        if directory == root {
            break;
        }
        current = directory.parent();
    }
    directories.reverse();

    let mut guidance = Vec::new();
    for directory in directories {
        for name in ["AGENTS.md", "CONTRIBUTING.md"] {
            let candidate = directory.join(name);
            if fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
                let path = candidate.strip_prefix(root)?.display().to_string();
                let scope = directory.strip_prefix(root)?.display().to_string();
                guidance.push(GuidanceFile {
                    path,
                    scope: if scope.is_empty() {
                        ".".to_string()
                    } else {
                        scope
                    },
                    guidance_kind: name.to_string(),
                });
            }
        }
    }

    Ok(guidance)
}

fn recent_commits(
    root: &Path,
    target: &Path,
    limit: usize,
) -> Result<(Vec<CommitSummary>, bool), Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "log",
            "--no-merges",
            "--format=%H%x1f%cI%x1f%s",
            "-n",
        ])
        .arg((limit + 1).to_string())
        .arg("--")
        .arg(target)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log failed for {}: {stderr}", target.display()).into());
    }

    let mut commits = String::from_utf8(output.stdout)?
        .lines()
        .filter_map(parse_commit_summary)
        .collect::<Vec<_>>();
    let truncated = commits.len() > limit;
    commits.truncate(limit);
    Ok((commits, truncated))
}

fn parse_commit_summary(line: &str) -> Option<CommitSummary> {
    let mut fields = line.splitn(3, '\u{1f}');
    Some(CommitSummary {
        sha: fields.next()?.to_string(),
        date: fields.next()?.to_string(),
        subject: fields.next()?.to_string(),
    })
}

fn historical_companions(
    root: &Path,
    target: &Path,
    recent: &[CommitSummary],
    budget: PacketBudget,
) -> Result<CompanionAnalysis, Box<dyn Error>> {
    let mut considered = Vec::new();
    let mut exclusions = Vec::new();

    for summary in recent {
        let commit = read_commit_paths(root, summary)?;
        if is_revert_subject(&summary.subject) {
            exclusions.push(ExcludedCommit {
                commit: summary.clone(),
                reason: "revert commit".to_string(),
                changed_paths: commit.paths.len(),
            });
            continue;
        }
        if commit.paths.len() > budget.max_paths_per_companion_commit {
            exclusions.push(ExcludedCommit {
                commit: summary.clone(),
                reason: format!(
                    "broad commit changed more than {} paths",
                    budget.max_paths_per_companion_commit
                ),
                changed_paths: commit.paths.len(),
            });
            continue;
        }
        considered.push(commit);
    }

    let opportunities = considered.len();
    let mut support = BTreeMap::<PathBuf, Vec<usize>>::new();
    for (index, commit) in considered.iter().enumerate() {
        for path in &commit.paths {
            if path == target {
                continue;
            }
            support.entry(path.clone()).or_default().push(index);
        }
    }

    let mut companions = support
        .into_iter()
        .map(|(path, present_in)| {
            let present = present_in.iter().copied().collect::<BTreeSet<_>>();
            let examples = present_in
                .iter()
                .take(budget.max_examples_per_relation)
                .map(|index| considered[*index].summary.clone())
                .collect::<Vec<_>>();
            let counterexamples = (0..considered.len())
                .filter(|index| !present.contains(index))
                .take(budget.max_examples_per_relation)
                .map(|index| considered[index].summary.clone())
                .collect::<Vec<_>>();
            let support_count = present_in.len();
            let counterexample_count = opportunities.saturating_sub(support_count);
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
                examples_omitted: support_count.saturating_sub(examples.len()),
                examples,
                counterexamples_omitted: counterexample_count.saturating_sub(counterexamples.len()),
                counterexamples,
            }
        })
        .collect::<Vec<_>>();

    companions.sort_by(|a, b| b.support.cmp(&a.support).then_with(|| a.path.cmp(&b.path)));
    let omitted_companions = companions.len().saturating_sub(budget.max_companions);
    companions.truncate(budget.max_companions);

    Ok(CompanionAnalysis {
        companions,
        omitted_companions,
        exclusions,
    })
}

fn read_commit_paths(
    root: &Path,
    summary: &CommitSummary,
) -> Result<HistoricalCommit, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "show",
            "--format=",
            "--name-only",
            "--no-renames",
            "--no-color",
            "--no-ext-diff",
            "--root",
        ])
        .arg(&summary.sha)
        .arg("--")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git show failed for {}: {stderr}", summary.sha).into());
    }

    let paths = String::from_utf8(output.stdout)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(HistoricalCommit {
        summary: summary.clone(),
        paths,
    })
}

fn is_revert_subject(subject: &str) -> bool {
    subject
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("revert")
}

use std::collections::BTreeSet as ScopedBTreeSet;

use serde::Serialize as ScopedSerialize;

const SCOPED_SCHEMA_VERSION: u32 = 1;
const DEFAULT_MAX_SCOPE_HISTORY: usize = 20;

#[derive(Debug, ScopedSerialize)]
struct ScopedEnvelopeBudget {
    max_serialized_bytes: usize,
    max_scope_history_commits: usize,
}

#[derive(Debug, ScopedSerialize)]
struct ScopedAgentContextEnvelope {
    schema_version: u32,
    analysis: &'static str,
    repository: String,
    target: String,
    scope: String,
    budget: ScopedEnvelopeBudget,
    candidate_serialized_bytes: usize,
    serialized_bytes: usize,
    semantic_evictions: Vec<SemanticEviction>,
    file_packet: AgentContextPacket,
    scope_recent_history: Vec<CommitSummary>,
    scope_history_truncated: bool,
    unknowns: Vec<String>,
}

#[derive(Debug)]
enum ScopedBudgetError {
    Serialize(serde_json::Error),
    ProtectedCoreTooLarge {
        max_serialized_bytes: usize,
        required_serialized_bytes: usize,
    },
}

impl fmt::Display for ScopedBudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(error) => write!(formatter, "failed to serialize scoped packet: {error}"),
            Self::ProtectedCoreTooLarge {
                max_serialized_bytes,
                required_serialized_bytes,
            } => write!(
                formatter,
                "protected scoped packet evidence requires {required_serialized_bytes} bytes, exceeding max_serialized_bytes={max_serialized_bytes}"
            ),
        }
    }
}

impl Error for ScopedBudgetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialize(error) => Some(error),
            Self::ProtectedCoreTooLarge { .. } => None,
        }
    }
}

impl From<serde_json::Error> for ScopedBudgetError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialize(error)
    }
}

pub fn run_scoped() -> Result<(), Box<dyn Error>> {
    let (target_arg, scope_arg) = parse_scoped_args(env::args().skip(1))?;

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

    let scope = resolve_explicit_scope(&root, &target, &scope_arg)?;
    let relative_scope = scope.strip_prefix(&root)?.to_path_buf();
    let budget = packet_budget_from_env()?;
    let mut file_packet = build_file_packet(&root, &target, &relative_target, budget)?;
    stabilize_candidate_size(&mut file_packet)?;

    let (scope_history, scope_history_truncated) =
        scoped_recent_commits(&root, &relative_scope, DEFAULT_MAX_SCOPE_HISTORY)?;
    let scope_recent_history = dedupe_scope_history(scope_history, &file_packet.recent_history);

    let mut envelope = ScopedAgentContextEnvelope {
        schema_version: SCOPED_SCHEMA_VERSION,
        analysis: "scoped_agent_context",
        repository: root.display().to_string(),
        target: relative_target.display().to_string(),
        scope: relative_scope.display().to_string(),
        budget: ScopedEnvelopeBudget {
            max_serialized_bytes: budget.max_serialized_bytes,
            max_scope_history_commits: DEFAULT_MAX_SCOPE_HISTORY,
        },
        candidate_serialized_bytes: 0,
        serialized_bytes: 0,
        semantic_evictions: Vec::new(),
        file_packet,
        scope_recent_history,
        scope_history_truncated,
        unknowns: vec![
            "Explicit scope chronology does not by itself prove that a scope-history commit is semantically relevant to the target.".to_string(),
        ],
    };

    println!("{}", compile_scoped_to_budget(&mut envelope)?);
    Ok(())
}

fn parse_scoped_args<I>(mut args: I) -> Result<(String, String), Box<dyn Error>>
where
    I: Iterator<Item = String>,
{
    let target = args.next().ok_or(
        "usage: cargo run --example scoped_agent_context_packet -- FILE --scope DIR",
    )?;
    if args.next().as_deref() != Some("--scope") {
        return Err(
            "usage: cargo run --example scoped_agent_context_packet -- FILE --scope DIR".into(),
        );
    }
    let scope = args.next().ok_or(
        "usage: cargo run --example scoped_agent_context_packet -- FILE --scope DIR",
    )?;
    if args.next().is_some() {
        return Err(
            "usage: cargo run --example scoped_agent_context_packet -- FILE --scope DIR".into(),
        );
    }
    validate_scope_text(&scope)?;
    Ok((target, scope))
}

fn validate_scope_text(scope: &str) -> Result<(), Box<dyn Error>> {
    if scope.is_empty()
        || scope == "."
        || scope.starts_with('/')
        || scope.ends_with('/')
        || scope.contains('\\')
        || scope.contains('\0')
        || scope
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err("scope must be a canonical repository-relative directory".into());
    }
    Ok(())
}

fn resolve_explicit_scope(root: &Path, target: &Path, scope: &str) -> Result<PathBuf, Box<dyn Error>> {
    validate_scope_text(scope)?;
    let resolved = root.join(scope).canonicalize()?;
    if !resolved.is_dir() {
        return Err(format!("scope must be an existing directory: {scope}").into());
    }
    if !resolved.starts_with(root) {
        return Err("scope must stay inside the resolved Git repository".into());
    }
    if !target.starts_with(&resolved) {
        return Err("scope must contain the target file".into());
    }
    Ok(resolved)
}

fn build_file_packet(
    root: &Path,
    target: &Path,
    relative_target: &Path,
    budget: PacketBudget,
) -> Result<AgentContextPacket, Box<dyn Error>> {
    let guidance = applicable_guidance(root, target)?;
    let reviewed_decisions = decision_memory::resolve_repository_decisions(root, relative_target)?;
    let (recent_history, history_truncated) =
        recent_commits(root, relative_target, budget.max_history_commits)?;
    let CompanionAnalysis {
        companions: historical_companions,
        omitted_companions,
        exclusions: companion_exclusions,
    } = historical_companions(root, relative_target, &recent_history, budget)?;

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

    Ok(AgentContextPacket {
        schema_version: SCHEMA_VERSION,
        analysis: "agent_context",
        repository: root.display().to_string(),
        target: PacketTarget {
            path: relative_target.display().to_string(),
        },
        budget,
        candidate_serialized_bytes: 0,
        serialized_bytes: 0,
        semantic_evictions: Vec::new(),
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
            "A decision matched through `git_file_lineage` relies on Git rename detection; the packet preserves that provenance instead of treating the match as a direct current-path scope.".to_string(),
            "Remote pull request, issue, and review rationale outside repository decision memory is unavailable in this local-only packet.".to_string(),
            "Chronological proximity between commits is not evidence that one change caused another.".to_string(),
            "Current Cultist analyzer findings are not yet composed into this standalone research example.".to_string(),
            "The v5 byte-budget eviction order is research policy and has not been promoted as a universal JEI ranking.".to_string(),
        ],
        truncation,
    })
}

fn scoped_recent_commits(
    root: &Path,
    scope: &Path,
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
        .arg(scope)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log failed for scope {}: {stderr}", scope.display()).into());
    }

    let mut commits = String::from_utf8(output.stdout)?
        .lines()
        .filter_map(parse_commit_summary)
        .collect::<Vec<_>>();
    let truncated = commits.len() > limit;
    commits.truncate(limit);
    Ok((commits, truncated))
}

fn dedupe_scope_history(
    scope_history: Vec<CommitSummary>,
    target_history: &[CommitSummary],
) -> Vec<CommitSummary> {
    let target_shas = target_history
        .iter()
        .map(|commit| commit.sha.as_str())
        .collect::<ScopedBTreeSet<_>>();
    scope_history
        .into_iter()
        .filter(|commit| !target_shas.contains(commit.sha.as_str()))
        .collect()
}

fn compile_scoped_to_budget(
    envelope: &mut ScopedAgentContextEnvelope,
) -> Result<String, ScopedBudgetError> {
    envelope.semantic_evictions.clear();
    envelope.candidate_serialized_bytes = 0;
    envelope.serialized_bytes = 0;
    stabilize_scoped_candidate_size(envelope)?;

    loop {
        let rendered = render_scoped_with_exact_size(envelope)?;
        if rendered.len() <= envelope.budget.max_serialized_bytes {
            return Ok(rendered);
        }

        if envelope.scope_recent_history.pop().is_some() {
            record_scoped_eviction(envelope, "scope_recent_history_summary");
            envelope.serialized_bytes = 0;
            continue;
        }

        if let Some(class) = evict_one_semantic_detail(&mut envelope.file_packet) {
            record_semantic_eviction(&mut envelope.file_packet, class);
            envelope.file_packet.serialized_bytes = 0;
            let _ = render_packet_with_exact_size(&mut envelope.file_packet)?;
            envelope.serialized_bytes = 0;
            continue;
        }

        return Err(ScopedBudgetError::ProtectedCoreTooLarge {
            max_serialized_bytes: envelope.budget.max_serialized_bytes,
            required_serialized_bytes: rendered.len(),
        });
    }
}

fn stabilize_scoped_candidate_size(
    envelope: &mut ScopedAgentContextEnvelope,
) -> Result<(), serde_json::Error> {
    loop {
        let rendered = serde_json::to_string_pretty(envelope)?;
        let serialized_bytes = rendered.len();
        if envelope.candidate_serialized_bytes == serialized_bytes
            && envelope.serialized_bytes == serialized_bytes
        {
            return Ok(());
        }
        envelope.candidate_serialized_bytes = serialized_bytes;
        envelope.serialized_bytes = serialized_bytes;
    }
}

fn render_scoped_with_exact_size(
    envelope: &mut ScopedAgentContextEnvelope,
) -> Result<String, serde_json::Error> {
    loop {
        let rendered = serde_json::to_string_pretty(envelope)?;
        let serialized_bytes = rendered.len();
        if envelope.serialized_bytes == serialized_bytes {
            return Ok(rendered);
        }
        envelope.serialized_bytes = serialized_bytes;
    }
}

fn record_scoped_eviction(envelope: &mut ScopedAgentContextEnvelope, class: &'static str) {
    if let Some(receipt) = envelope
        .semantic_evictions
        .iter_mut()
        .find(|receipt| receipt.class == class)
    {
        receipt.count += 1;
        return;
    }
    envelope
        .semantic_evictions
        .push(SemanticEviction { class, count: 1 });
}

#[cfg(test)]
mod scoped_tests {
    use super::*;

    fn summary(sha: &str, subject: &str) -> CommitSummary {
        CommitSummary {
            sha: sha.to_string(),
            date: "2026-08-19T00:00:00Z".to_string(),
            subject: subject.to_string(),
        }
    }

    fn minimal_file_packet(max_serialized_bytes: usize) -> AgentContextPacket {
        let budget = PacketBudget {
            max_serialized_bytes,
            ..PacketBudget::default()
        };
        AgentContextPacket {
            schema_version: SCHEMA_VERSION,
            analysis: "agent_context",
            repository: "/repo".to_string(),
            target: PacketTarget {
                path: "src/target.rs".to_string(),
            },
            budget,
            candidate_serialized_bytes: 0,
            serialized_bytes: 0,
            semantic_evictions: Vec::new(),
            direct_evidence: vec![EvidenceItem {
                claim_kind: "proven",
                message: "target identity".to_string(),
                source: EvidenceSource {
                    kind: "filesystem",
                    path: "src/target.rs".to_string(),
                },
            }],
            guidance: Vec::new(),
            reviewed_decisions: Vec::new(),
            recent_history: vec![summary("target", "target history")],
            historical_companions: Vec::new(),
            companion_exclusions: Vec::new(),
            unknowns: vec!["material unknown".to_string()],
            truncation: Vec::new(),
        }
    }

    fn scoped_envelope(max_serialized_bytes: usize) -> ScopedAgentContextEnvelope {
        let mut file_packet = minimal_file_packet(max_serialized_bytes);
        stabilize_candidate_size(&mut file_packet).unwrap();
        ScopedAgentContextEnvelope {
            schema_version: SCOPED_SCHEMA_VERSION,
            analysis: "scoped_agent_context",
            repository: "/repo".to_string(),
            target: "src/target.rs".to_string(),
            scope: "src".to_string(),
            budget: ScopedEnvelopeBudget {
                max_serialized_bytes,
                max_scope_history_commits: DEFAULT_MAX_SCOPE_HISTORY,
            },
            candidate_serialized_bytes: 0,
            serialized_bytes: 0,
            semantic_evictions: Vec::new(),
            file_packet,
            scope_recent_history: vec![
                summary("scope-new", &format!("new {}", "x".repeat(700))),
                summary("scope-old", &format!("old {}", "x".repeat(700))),
            ],
            scope_history_truncated: true,
            unknowns: vec!["scope chronology is not semantic proof".to_string()],
        }
    }

    #[test]
    fn scope_text_rejects_traversal_absolute_and_noncanonical_paths() {
        for value in ["", ".", "../src", "/src", "src/", "src\\nested", "src/./nested"] {
            assert!(validate_scope_text(value).is_err(), "accepted {value:?}");
        }
        assert!(validate_scope_text("src").is_ok());
        assert!(validate_scope_text("src/nested").is_ok());
    }

    #[test]
    fn scope_history_deduplicates_target_local_commits() {
        let target = vec![summary("same", "target")];
        let scoped = vec![summary("new", "new"), summary("same", "duplicate")];
        let deduped = dedupe_scope_history(scoped, &target);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].sha, "new");
    }

    #[test]
    fn scope_history_is_evicted_before_target_history() {
        let mut envelope = scoped_envelope(usize::MAX);
        stabilize_scoped_candidate_size(&mut envelope).unwrap();
        let candidate = envelope.candidate_serialized_bytes;
        envelope.budget.max_serialized_bytes = candidate - 200;
        envelope.file_packet.budget.max_serialized_bytes = candidate - 200;

        let target_before = envelope.file_packet.recent_history.len();
        let rendered = compile_scoped_to_budget(&mut envelope).unwrap();
        assert!(rendered.len() <= envelope.budget.max_serialized_bytes);
        assert!(envelope.scope_recent_history.len() < 2);
        assert_eq!(envelope.file_packet.recent_history.len(), target_before);
        assert_eq!(
            envelope.semantic_evictions.first(),
            Some(&SemanticEviction {
                class: "scope_recent_history_summary",
                count: 1,
            })
        );
    }
}

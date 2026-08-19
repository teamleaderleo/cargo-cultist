use std::collections::BTreeSet as SelectedScopeBTreeSet;

use serde::Serialize as SelectedScopeSerialize;

#[derive(Debug, SelectedScopeSerialize)]
struct SelectedScopedEnvelope {
    protected_scope_shas: Vec<String>,
    #[serde(flatten)]
    envelope: ScopedAgentContextEnvelope,
}

pub fn run_scoped_with_optional_protection() -> Result<(), Box<dyn Error>> {
    if !env::args().any(|arg| arg == "--protect-scope-sha") {
        return run_scoped();
    }

    let (target_arg, scope_arg, protected_scope_shas) =
        parse_selected_scope_args(env::args().skip(1))?;

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
    validate_selected_scope_shas(&scope_recent_history, &protected_scope_shas)?;

    let envelope = ScopedAgentContextEnvelope {
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
            "Scope chronology is not semantic proof; exact protected scope SHAs were selected upstream."
                .to_string(),
        ],
    };
    let mut selected = SelectedScopedEnvelope {
        protected_scope_shas: protected_scope_shas.iter().cloned().collect(),
        envelope,
    };

    println!("{}", compile_selected_scoped_to_budget(&mut selected)?);
    Ok(())
}

fn parse_selected_scope_args<I>(
    mut args: I,
) -> Result<(String, String, SelectedScopeBTreeSet<String>), Box<dyn Error>>
where
    I: Iterator<Item = String>,
{
    let usage = "usage: cargo run --example scoped_agent_context_packet -- FILE --scope DIR --protect-scope-sha SHA [--protect-scope-sha SHA ...]";
    let target = args.next().ok_or(usage)?;
    if args.next().as_deref() != Some("--scope") {
        return Err(usage.into());
    }
    let scope = args.next().ok_or(usage)?;
    validate_scope_text(&scope)?;

    let mut protected = SelectedScopeBTreeSet::new();
    while let Some(flag) = args.next() {
        if flag != "--protect-scope-sha" {
            return Err(usage.into());
        }
        let sha = args.next().ok_or(usage)?;
        validate_selected_scope_sha(&sha)?;
        protected.insert(sha);
    }
    if protected.is_empty() {
        return Err("at least one --protect-scope-sha is required in protected mode".into());
    }
    Ok((target, scope, protected))
}

fn validate_selected_scope_sha(sha: &str) -> Result<(), Box<dyn Error>> {
    if sha.len() != 40
        || !sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("protected scope evidence refs must be exact lowercase 40-hex Git SHAs".into());
    }
    Ok(())
}

fn validate_selected_scope_shas(
    scope_recent_history: &[CommitSummary],
    protected_scope_shas: &SelectedScopeBTreeSet<String>,
) -> Result<(), Box<dyn Error>> {
    let available = scope_recent_history
        .iter()
        .map(|commit| commit.sha.as_str())
        .collect::<SelectedScopeBTreeSet<_>>();
    for protected in protected_scope_shas {
        if !available.contains(protected.as_str()) {
            return Err(format!(
                "selected scope evidence `{protected}` is absent from the admitted deduplicated scope-history window"
            )
            .into());
        }
    }
    Ok(())
}

fn compile_selected_scoped_to_budget(
    selected: &mut SelectedScopedEnvelope,
) -> Result<String, ScopedBudgetError> {
    selected.envelope.semantic_evictions.clear();
    selected.envelope.candidate_serialized_bytes = 0;
    selected.envelope.serialized_bytes = 0;
    stabilize_selected_candidate_size(selected)?;

    loop {
        let rendered = render_selected_with_exact_size(selected)?;
        if rendered.len() <= selected.envelope.budget.max_serialized_bytes {
            return Ok(rendered);
        }

        if let Some(index) = selected
            .envelope
            .scope_recent_history
            .iter()
            .rposition(|commit| !selected.protected_scope_shas.contains(&commit.sha))
        {
            selected.envelope.scope_recent_history.remove(index);
            record_scoped_eviction(&mut selected.envelope, "scope_recent_history_summary");
            selected.envelope.serialized_bytes = 0;
            continue;
        }

        if let Some(class) = evict_one_semantic_detail(&mut selected.envelope.file_packet) {
            record_semantic_eviction(&mut selected.envelope.file_packet, class);
            selected.envelope.file_packet.serialized_bytes = 0;
            let _ = render_packet_with_exact_size(&mut selected.envelope.file_packet)?;
            selected.envelope.serialized_bytes = 0;
            continue;
        }

        return Err(ScopedBudgetError::ProtectedCoreTooLarge {
            max_serialized_bytes: selected.envelope.budget.max_serialized_bytes,
            required_serialized_bytes: rendered.len(),
        });
    }
}

fn stabilize_selected_candidate_size(
    selected: &mut SelectedScopedEnvelope,
) -> Result<(), serde_json::Error> {
    loop {
        let rendered = serde_json::to_string_pretty(selected)?;
        let serialized_bytes = rendered.len();
        if selected.envelope.candidate_serialized_bytes == serialized_bytes
            && selected.envelope.serialized_bytes == serialized_bytes
        {
            return Ok(());
        }
        selected.envelope.candidate_serialized_bytes = serialized_bytes;
        selected.envelope.serialized_bytes = serialized_bytes;
    }
}

fn render_selected_with_exact_size(
    selected: &mut SelectedScopedEnvelope,
) -> Result<String, serde_json::Error> {
    loop {
        let rendered = serde_json::to_string_pretty(selected)?;
        let serialized_bytes = rendered.len();
        if selected.envelope.serialized_bytes == serialized_bytes {
            return Ok(rendered);
        }
        selected.envelope.serialized_bytes = serialized_bytes;
    }
}

#[cfg(test)]
mod selected_scope_tests {
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
            recent_history: vec![summary(
                "1111111111111111111111111111111111111111",
                "target history",
            )],
            historical_companions: Vec::new(),
            companion_exclusions: Vec::new(),
            unknowns: vec!["material unknown".to_string()],
            truncation: Vec::new(),
        }
    }

    fn selected_envelope(max_serialized_bytes: usize) -> SelectedScopedEnvelope {
        let mut file_packet = minimal_file_packet(max_serialized_bytes);
        stabilize_candidate_size(&mut file_packet).unwrap();
        SelectedScopedEnvelope {
            protected_scope_shas: vec![
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            ],
            envelope: ScopedAgentContextEnvelope {
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
                    summary(
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        &format!("unprotected {}", "x".repeat(700)),
                    ),
                    summary(
                        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                        &format!("protected {}", "x".repeat(700)),
                    ),
                ],
                scope_history_truncated: true,
                unknowns: vec!["selected scope refs supplied upstream".to_string()],
            },
        }
    }

    #[test]
    fn exact_sha_parser_and_duplicate_flags_are_deterministic() {
        let (_, _, protected) = parse_selected_scope_args(
            [
                "src/target.rs",
                "--scope",
                "src",
                "--protect-scope-sha",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "--protect-scope-sha",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .unwrap();
        assert_eq!(protected.len(), 1);

        for invalid in [
            "Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "gggggggggggggggggggggggggggggggggggggggg",
        ] {
            assert!(validate_selected_scope_sha(invalid).is_err());
        }
    }

    #[test]
    fn selected_sha_must_exist_after_target_history_deduplication() {
        let scope = vec![summary(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "selected",
        )];
        let present = SelectedScopeBTreeSet::from([
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        ]);
        assert!(validate_selected_scope_shas(&scope, &present).is_ok());

        let missing = SelectedScopeBTreeSet::from([
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
        ]);
        assert!(validate_selected_scope_shas(&scope, &missing).is_err());
    }

    #[test]
    fn protected_scope_row_survives_before_unprotected_sibling() {
        let mut selected = selected_envelope(usize::MAX);
        stabilize_selected_candidate_size(&mut selected).unwrap();
        let candidate = selected.envelope.candidate_serialized_bytes;
        selected.envelope.budget.max_serialized_bytes = candidate - 200;
        selected.envelope.file_packet.budget.max_serialized_bytes = candidate - 200;

        let rendered = compile_selected_scoped_to_budget(&mut selected).unwrap();
        assert!(rendered.len() <= selected.envelope.budget.max_serialized_bytes);
        assert!(
            selected
                .envelope
                .scope_recent_history
                .iter()
                .any(|commit| commit.sha == "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
        assert!(
            selected
                .envelope
                .scope_recent_history
                .iter()
                .all(|commit| commit.sha != "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(
            selected.envelope.semantic_evictions.first(),
            Some(&SemanticEviction {
                class: "scope_recent_history_summary",
                count: 1,
            })
        );
    }

    #[test]
    fn protected_selection_is_budgeted_and_fail_closed() {
        let mut selected = selected_envelope(1);
        let error = compile_selected_scoped_to_budget(&mut selected).unwrap_err();
        assert!(matches!(error, ScopedBudgetError::ProtectedCoreTooLarge { .. }));
        assert!(
            selected
                .envelope
                .scope_recent_history
                .iter()
                .any(|commit| commit.sha == "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
    }

    #[test]
    fn protected_output_receipts_selected_shas() {
        let mut selected = selected_envelope(usize::MAX);
        let rendered = render_selected_with_exact_size(&mut selected).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(
            json["protected_scope_shas"][0],
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        );
    }
}

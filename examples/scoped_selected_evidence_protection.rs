#[allow(dead_code)]
mod packet {
    include!("agent_context_packet.rs");
    include!("support/scoped_agent_context_packet_impl.rs");

    use std::collections::BTreeSet as ProtectedBTreeSet;

    pub fn run_protected_scoped() -> Result<(), Box<dyn Error>> {
        let (target_arg, scope_arg, protected_scope_shas) =
            parse_protected_args(env::args().skip(1))?;

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
        let available = scope_recent_history
            .iter()
            .map(|commit| commit.sha.as_str())
            .collect::<ProtectedBTreeSet<_>>();
        for protected in &protected_scope_shas {
            if !available.contains(protected.as_str()) {
                return Err(format!(
                    "selected scope evidence `{protected}` is absent from the admitted scope-history window"
                )
                .into());
            }
        }

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
                "This research compiler protects only exact scope evidence refs selected upstream; the byte budget does not infer semantic importance.".to_string(),
            ],
        };

        println!(
            "{}",
            compile_scoped_to_budget_protecting(&mut envelope, &protected_scope_shas)?
        );
        Ok(())
    }

    fn parse_protected_args<I>(
        mut args: I,
    ) -> Result<(String, String, ProtectedBTreeSet<String>), Box<dyn Error>>
    where
        I: Iterator<Item = String>,
    {
        let usage = "usage: cargo run --example scoped_selected_evidence_protection -- FILE --scope DIR --protect-scope-sha SHA [--protect-scope-sha SHA ...]";
        let target = args.next().ok_or(usage)?;
        if args.next().as_deref() != Some("--scope") {
            return Err(usage.into());
        }
        let scope = args.next().ok_or(usage)?;
        validate_scope_text(&scope)?;

        let mut protected = ProtectedBTreeSet::new();
        while let Some(flag) = args.next() {
            if flag != "--protect-scope-sha" {
                return Err(usage.into());
            }
            let sha = args.next().ok_or(usage)?;
            validate_exact_sha(&sha)?;
            protected.insert(sha);
        }
        if protected.is_empty() {
            return Err("at least one --protect-scope-sha is required".into());
        }
        Ok((target, scope, protected))
    }

    fn validate_exact_sha(sha: &str) -> Result<(), Box<dyn Error>> {
        if sha.len() != 40
            || !sha
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("protected scope evidence refs must be exact lowercase 40-hex Git SHAs".into());
        }
        Ok(())
    }

    fn compile_scoped_to_budget_protecting(
        envelope: &mut ScopedAgentContextEnvelope,
        protected_scope_shas: &ProtectedBTreeSet<String>,
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

            if let Some(index) = envelope
                .scope_recent_history
                .iter()
                .rposition(|commit| !protected_scope_shas.contains(&commit.sha))
            {
                envelope.scope_recent_history.remove(index);
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
}

fn main() {
    if let Err(error) = packet::run_protected_scoped() {
        eprintln!("scoped-selected-evidence-protection: {error}");
        std::process::exit(1);
    }
}

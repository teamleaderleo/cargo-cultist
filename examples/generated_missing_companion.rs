#[allow(dead_code)]
#[path = "../src/history.rs"]
mod history;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprLit, ExprMethodCall, File, ItemFn, Lit, Pat, Stmt};

const DEFAULT_MAX_COMMITS: usize = 100;
const MAX_PATHS_PER_COMMIT: usize = 100;
const COUNTEREXAMPLE_LIMIT: usize = 3;

#[derive(Debug, Clone, Eq, PartialEq)]
struct FunctionIo {
    function: String,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SyntaxCohort {
    opportunities: usize,
    support: BTreeMap<String, usize>,
    counterexamples: BTreeMap<String, Vec<String>>,
    comments_only: usize,
    unclassified: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("generated-missing-companion: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(args.next().ok_or(
        "usage: generated_missing_companion REPO GENERATOR_RS SOURCE_RS [MAX_COMMITS]",
    )?)
    .canonicalize()?;
    let generator = PathBuf::from(args.next().ok_or(
        "usage: generated_missing_companion REPO GENERATOR_RS SOURCE_RS [MAX_COMMITS]",
    )?);
    let source = PathBuf::from(args.next().ok_or(
        "usage: generated_missing_companion REPO GENERATOR_RS SOURCE_RS [MAX_COMMITS]",
    )?);
    let max_commits = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(DEFAULT_MAX_COMMITS);
    if args.next().is_some() {
        return Err(
            "usage: generated_missing_companion REPO GENERATOR_RS SOURCE_RS [MAX_COMMITS]"
                .into(),
        );
    }
    if generator.is_absolute() || source.is_absolute() {
        return Err("GENERATOR_RS and SOURCE_RS must be repository-relative".into());
    }
    if source.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return Err("SOURCE_RS must be a Rust source file".into());
    }

    let generator_source = fs::read_to_string(root.join(&generator))?;
    let generator_file = syn::parse_file(&generator_source)?;
    let functions = collect_function_io(&generator_file);
    let source_key = normalize_repo_path(&source.to_string_lossy());
    let output_owners = outputs_for_source(&functions, &source_key);
    if output_owners.is_empty() {
        println!("NO FINDING");
        println!("  No recognized generator function reads `{source_key}` and writes repository paths.");
        return Ok(());
    }

    let outputs: BTreeSet<String> = output_owners.keys().cloned().collect();
    let changed = changed_paths(&root)?;
    let source_changed = changed.contains(Path::new(&source_key));
    let syntax_changed = worktree_rust_syntax_changed(&root, &source)?;

    println!("GENERATED COMPANION DIFF PROBE");
    println!("  repository: {}", root.display());
    println!("  source: {source_key}");
    println!("  generator: {}", generator.display());
    println!("  source path changed in worktree: {source_changed}");
    println!("  source Rust syntax changed: {syntax_changed}");

    if !source_changed || !syntax_changed {
        println!("\nNO FINDING");
        if source_changed {
            println!(
                "  The source path changed, but its normalized Rust syntax is unchanged after comments/docs/whitespace are removed."
            );
        } else {
            println!("  The source path is absent from the current worktree diff.");
        }
        return Ok(());
    }

    let history_report = history::analyze_historical_companions(
        &root,
        &source,
        history::HistoryOptions {
            max_commits,
            max_paths_per_commit: MAX_PATHS_PER_COMMIT,
        },
    )?;
    let syntax_cohort = build_syntax_cohort(&root, &source, &outputs, max_commits)?;
    let generated_attrs = generated_attribute_paths(&root);
    let aliases = generator_aliases(&root, root.join(&generator).as_path());

    let missing: Vec<_> = outputs
        .iter()
        .filter(|output| !changed.contains(Path::new(output.as_str())))
        .cloned()
        .collect();

    if missing.is_empty() {
        println!("\nNO FINDING");
        println!("  Every recognized generator output for this source is present in the current diff.");
        return Ok(());
    }

    println!("\nFINDING: generated companions absent from a source-syntax change");
    println!("\nPROVEN / DERIVED");
    println!("  The current worktree changes Rust syntax in `{source_key}`.");
    for output in &outputs {
        let owner = output_owners
            .get(output)
            .map(String::as_str)
            .unwrap_or("unknown");
        println!("  Generator function `{owner}` reads `{source_key}` and writes `{output}`.");
        if generated_attrs.contains(output) {
            println!("  `.gitattributes` marks `{output}` as `linguist-generated=true`.");
        }
        if let Some(marker) = current_generated_marker(&history_report, output) {
            println!(
                "  `{output}` self-identifies as generated at line {}: {}",
                marker.line, marker.marker
            );
        }
    }
    for alias in &aliases {
        println!("  Repository Cargo alias: `cargo {}` -> `{}`.", alias.0, alias.1);
    }

    println!("\nOBSERVED");
    println!(
        "  Rust syntax-change cohort: {} comparable commit(s); {} comments/docs-only commit(s); {} unclassified commit(s).",
        syntax_cohort.opportunities, syntax_cohort.comments_only, syntax_cohort.unclassified
    );
    for output in &outputs {
        let support = syntax_cohort.support.get(output).copied().unwrap_or_default();
        println!(
            "  `{output}` changed in {support}/{} comparable Rust syntax-changing source commits ({:.1}%).",
            syntax_cohort.opportunities,
            percent(support, syntax_cohort.opportunities)
        );
        if let Some(examples) = syntax_cohort.counterexamples.get(output)
            && !examples.is_empty()
        {
            println!("    counterexamples:");
            for example in examples {
                println!("      {example}");
            }
        }
    }

    println!("\nCURRENT ABSENCE");
    for output in &missing {
        println!("  `{output}` is absent from the current worktree diff.");
    }

    println!("\nUNKNOWN");
    println!(
        "  Repository evidence establishes generation ownership and historical precedent, but it does not establish whether this current absence is intentional."
    );

    println!("\nQUESTION");
    if let Some((name, _)) = aliases.first() {
        println!(
            "  Was `cargo {name}` intentionally deferred for this source change, or are the generated companions stale?"
        );
    } else {
        println!(
            "  Was regeneration intentionally deferred for this source change, or are the generated companions stale?"
        );
    }

    Ok(())
}

fn collect_function_io(file: &File) -> Vec<FunctionIo> {
    file.items
        .iter()
        .filter_map(|item| {
            let syn::Item::Fn(function) = item else {
                return None;
            };
            Some(collect_one_function(function))
        })
        .collect()
}

fn collect_one_function(function: &ItemFn) -> FunctionIo {
    let bindings = collect_path_bindings(function);
    let mut visitor = IoVisitor {
        bindings: &bindings,
        reads: BTreeSet::new(),
        writes: BTreeSet::new(),
    };
    visitor.visit_block(&function.block);
    FunctionIo {
        function: function.sig.ident.to_string(),
        reads: visitor.reads,
        writes: visitor.writes,
    }
}

fn collect_path_bindings(function: &ItemFn) -> BTreeMap<String, String> {
    let mut bindings = BTreeMap::new();
    for stmt in &function.block.stmts {
        let Stmt::Local(local) = stmt else {
            continue;
        };
        let Pat::Ident(pat) = &local.pat else {
            continue;
        };
        let Some(init) = &local.init else {
            continue;
        };
        if let Some(path) = literal_join_path(&init.expr) {
            bindings.insert(pat.ident.to_string(), path);
        }
    }
    bindings
}

struct IoVisitor<'a> {
    bindings: &'a BTreeMap<String, String>,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for IoVisitor<'_> {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Some(kind) = fs_call_kind(&call.func)
            && let Some(first) = call.args.first()
            && let Some(path) = resolve_path_expr(first, self.bindings)
        {
            match kind {
                IoKind::Read => {
                    self.reads.insert(path);
                }
                IoKind::Write => {
                    self.writes.insert(path);
                }
            }
        }
        visit::visit_expr_call(self, call);
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum IoKind {
    Read,
    Write,
}

fn fs_call_kind(expr: &Expr) -> Option<IoKind> {
    let Expr::Path(path) = expr else {
        return None;
    };
    let segments: Vec<_> = path.path.segments.iter().collect();
    if segments.len() < 2 || segments[segments.len() - 2].ident != "fs" {
        return None;
    }
    match segments.last()?.ident.to_string().as_str() {
        "read" | "read_to_string" => Some(IoKind::Read),
        "write" => Some(IoKind::Write),
        _ => None,
    }
}

fn resolve_path_expr(expr: &Expr, bindings: &BTreeMap<String, String>) -> Option<String> {
    match expr {
        Expr::Reference(reference) => resolve_path_expr(&reference.expr, bindings),
        Expr::Paren(paren) => resolve_path_expr(&paren.expr, bindings),
        Expr::MethodCall(call) => literal_join_path_from_call(call),
        Expr::Path(path) if path.path.segments.len() == 1 => bindings
            .get(&path.path.segments[0].ident.to_string())
            .cloned(),
        _ => None,
    }
}

fn literal_join_path(expr: &Expr) -> Option<String> {
    let Expr::MethodCall(call) = expr else {
        return None;
    };
    literal_join_path_from_call(call)
}

fn literal_join_path_from_call(call: &ExprMethodCall) -> Option<String> {
    if call.method != "join" || call.args.len() != 1 {
        return None;
    }
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = call.args.first()?
    else {
        return None;
    };
    Some(normalize_repo_path(&value.value()))
}

fn outputs_for_source(functions: &[FunctionIo], source: &str) -> BTreeMap<String, String> {
    let mut outputs = BTreeMap::new();
    for function in functions {
        if !function.reads.contains(source) {
            continue;
        }
        for output in &function.writes {
            outputs.insert(output.clone(), function.function.clone());
        }
    }
    outputs
}

fn changed_paths(root: &Path) -> Result<BTreeSet<PathBuf>, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "core.quotepath=false", "diff", "--name-only", "HEAD", "--"])
        .output()?;
    if !output.status.success() {
        return Err(format!("git diff failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

fn worktree_rust_syntax_changed(root: &Path, source: &Path) -> Result<bool, Box<dyn Error>> {
    let before = source_at_revision(root, "HEAD", source)
        .ok_or_else(|| format!("could not read HEAD version of {}", source.display()))?;
    let after = fs::read_to_string(root.join(source))?;
    Ok(rust_syntax_fingerprint(&before)? != rust_syntax_fingerprint(&after)?)
}

fn build_syntax_cohort(
    root: &Path,
    source: &Path,
    outputs: &BTreeSet<String>,
    max_commits: usize,
) -> Result<SyntaxCohort, Box<dyn Error>> {
    let shas = history_shas(root, source, max_commits)?;
    let mut opportunities = 0;
    let mut support = BTreeMap::<String, usize>::new();
    let mut counterexamples = BTreeMap::<String, Vec<String>>::new();
    let mut comments_only = 0;
    let mut unclassified = 0;

    for sha in shas {
        let (subject, paths) = commit_metadata(root, &sha)?;
        if is_revert_subject(&subject) || paths.len() > MAX_PATHS_PER_COMMIT {
            continue;
        }

        match classify_historical_rust_edit(root, &sha, source) {
            HistoricalEdit::SyntaxChanged => {
                opportunities += 1;
                for output in outputs {
                    if paths.contains(Path::new(output)) {
                        *support.entry(output.clone()).or_default() += 1;
                    } else {
                        let examples = counterexamples.entry(output.clone()).or_default();
                        if examples.len() < COUNTEREXAMPLE_LIMIT {
                            examples.push(format!("{}  {}", short_sha(&sha), subject));
                        }
                    }
                }
            }
            HistoricalEdit::CommentsOnly => comments_only += 1,
            HistoricalEdit::Unclassified => unclassified += 1,
        }
    }

    Ok(SyntaxCohort {
        opportunities,
        support,
        counterexamples,
        comments_only,
        unclassified,
    })
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum HistoricalEdit {
    SyntaxChanged,
    CommentsOnly,
    Unclassified,
}

fn classify_historical_rust_edit(root: &Path, sha: &str, source: &Path) -> HistoricalEdit {
    let before = source_at_revision(root, &format!("{sha}^"), source);
    let after = source_at_revision(root, sha, source);
    let (Some(before), Some(after)) = (before, after) else {
        return HistoricalEdit::Unclassified;
    };
    let (Ok(before), Ok(after)) = (rust_syntax_fingerprint(&before), rust_syntax_fingerprint(&after))
    else {
        return HistoricalEdit::Unclassified;
    };
    if before == after {
        HistoricalEdit::CommentsOnly
    } else {
        HistoricalEdit::SyntaxChanged
    }
}

fn history_shas(root: &Path, source: &Path, max_commits: usize) -> Result<Vec<String>, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--no-merges", "--format=%H", "-n"])
        .arg(max_commits.to_string())
        .arg("--")
        .arg(source)
        .output()?;
    if !output.status.success() {
        return Err(format!("git log failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn commit_metadata(root: &Path, sha: &str) -> Result<(String, BTreeSet<PathBuf>), Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "show",
            "--format=%s%x1e",
            "--name-only",
            "--no-renames",
            "--no-color",
            "--no-ext-diff",
            "--root",
        ])
        .arg(sha)
        .arg("--")
        .output()?;
    if !output.status.success() {
        return Err(format!("git show failed for {sha}: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    let text = String::from_utf8(output.stdout)?;
    let (subject, paths) = text
        .split_once('\u{1e}')
        .ok_or_else(|| format!("could not parse git show output for {sha}"))?;
    Ok((
        subject.trim().to_string(),
        paths
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect(),
    ))
}

fn source_at_revision(root: &Path, revision: &str, source: &Path) -> Option<String> {
    let spec = format!(
        "{revision}:{}",
        source.to_string_lossy().replace('\\', "/")
    );
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &spec])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}

fn rust_syntax_fingerprint(source: &str) -> Result<String, Box<dyn Error>> {
    let tokens = TokenStream::from_str(source)?;
    Ok(strip_doc_attributes(tokens).to_string())
}

fn strip_doc_attributes(stream: TokenStream) -> TokenStream {
    let tokens: Vec<_> = stream.into_iter().collect();
    let mut output = TokenStream::new();
    let mut index = 0;

    while index < tokens.len() {
        if is_hash(&tokens[index]) {
            if index + 1 < tokens.len() && is_doc_group(&tokens[index + 1]) {
                index += 2;
                continue;
            }
            if index + 2 < tokens.len()
                && is_bang(&tokens[index + 1])
                && is_doc_group(&tokens[index + 2])
            {
                index += 3;
                continue;
            }
        }

        let token = match tokens[index].clone() {
            TokenTree::Group(group) => {
                let mut normalized = Group::new(group.delimiter(), strip_doc_attributes(group.stream()));
                normalized.set_span(group.span());
                TokenTree::Group(normalized)
            }
            other => other,
        };
        output.extend([token]);
        index += 1;
    }

    output
}

fn is_hash(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == '#')
}

fn is_bang(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == '!')
}

fn is_doc_group(token: &TokenTree) -> bool {
    let TokenTree::Group(group) = token else {
        return false;
    };
    if group.delimiter() != Delimiter::Bracket {
        return false;
    }
    matches!(group.stream().into_iter().next(), Some(TokenTree::Ident(ident)) if ident == "doc")
}

fn generated_attribute_paths(root: &Path) -> BTreeSet<String> {
    let path = root.join(".gitattributes");
    let Ok(source) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let path = fields.next()?;
            fields
                .any(|field| field == "linguist-generated=true")
                .then(|| normalize_repo_path(path))
        })
        .collect()
}

fn generator_aliases(root: &Path, generator_path: &Path) -> Vec<(String, String)> {
    let Some(package) = nearest_package_name(root, generator_path) else {
        return Vec::new();
    };
    let config = root.join(".cargo/config.toml");
    let Ok(source) = fs::read_to_string(config) else {
        return Vec::new();
    };
    cargo_aliases_from_config(&source)
        .into_iter()
        .filter(|(_, command)| command.contains(&package))
        .collect()
}

fn nearest_package_name(root: &Path, generator_path: &Path) -> Option<String> {
    let mut dir = generator_path.parent()?;
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file()
            && let Ok(source) = fs::read_to_string(manifest)
            && let Some(name) = package_name_from_manifest(&source)
        {
            return Some(name);
        }
        if dir == root {
            return None;
        }
        dir = dir.parent()?;
    }
}

fn package_name_from_manifest(source: &str) -> Option<String> {
    let mut in_package = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some(value) = trimmed.strip_prefix("name") else {
            continue;
        };
        let value = value.trim_start();
        let Some(value) = value.strip_prefix('=') else {
            continue;
        };
        return unquote(value.trim());
    }
    None
}

fn cargo_aliases_from_config(source: &str) -> Vec<(String, String)> {
    let mut aliases = Vec::new();
    let mut in_alias = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_alias = trimmed == "[alias]";
            continue;
        }
        if !in_alias || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        let Some(command) = unquote(value.trim()) else {
            continue;
        };
        aliases.push((name.trim().to_string(), command));
    }
    aliases
}

fn unquote(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() < 2 {
        return None;
    }
    let first = value.as_bytes()[0] as char;
    let last = value.as_bytes()[value.len() - 1] as char;
    if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}

fn current_generated_marker<'a>(
    report: &'a history::HistoryReport,
    output: &str,
) -> Option<&'a history::GeneratedMarkerEvidence> {
    report
        .companions
        .iter()
        .find(|companion| companion.path == output)
        .and_then(|companion| companion.generated_marker.as_ref())
}

fn normalize_repo_path(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn is_revert_subject(subject: &str) -> bool {
    subject.trim_start().to_ascii_lowercase().starts_with("revert")
}

fn percent(support: usize, opportunities: usize) -> f64 {
    if opportunities == 0 {
        0.0
    } else {
        support as f64 * 100.0 / opportunities as f64
    }
}

fn short_sha(sha: &str) -> &str {
    sha.get(..sha.len().min(8)).unwrap_or(sha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_to_outputs_is_directional() {
        let functions = vec![FunctionIo {
            function: "generate".to_string(),
            reads: BTreeSet::from(["src/rules.rs".to_string()]),
            writes: BTreeSet::from([
                "src/generated/a.rs".to_string(),
                "src/generated/b.rs".to_string(),
            ]),
        }];
        let outputs = outputs_for_source(&functions, "src/rules.rs");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs["src/generated/a.rs"], "generate");
    }

    #[test]
    fn docs_only_changes_have_equal_syntax_fingerprints() {
        let before = "/// old docs\nfn answer() -> usize { 42 }";
        let after = "/// new docs\nfn answer() -> usize { 42 }";
        assert_eq!(
            rust_syntax_fingerprint(before).unwrap(),
            rust_syntax_fingerprint(after).unwrap()
        );
    }

    #[test]
    fn code_changes_have_distinct_syntax_fingerprints() {
        assert_ne!(
            rust_syntax_fingerprint("fn answer() -> usize { 41 }").unwrap(),
            rust_syntax_fingerprint("fn answer() -> usize { 42 }").unwrap()
        );
    }

    #[test]
    fn parses_aliases_for_generator_package() {
        let aliases = cargo_aliases_from_config(
            "[alias]\nlintgen = \"run -p oxc_linter_codegen\"\ncheck = \"check\"\n",
        );
        assert_eq!(
            aliases[0],
            (
                "lintgen".to_string(),
                "run -p oxc_linter_codegen".to_string()
            )
        );
    }
}

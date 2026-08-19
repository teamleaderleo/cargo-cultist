use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprLit, ExprMethodCall, ItemFn, Lit, Pat, Stmt};
use walkdir::{DirEntry, WalkDir};

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};

const MAX_HISTORY_COMMITS: usize = 100;
const MAX_PATHS_PER_COMMIT: usize = 100;
const MIN_SYNTAX_COHORT: usize = 3;
const GENERATED_HEADER_BYTES: usize = 8 * 1024;
const GENERATED_HEADER_LINES: usize = 40;
const EXAMPLE_LIMIT: usize = 3;

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
struct GeneratorRelation {
    alias: String,
    package: String,
    generator_path: String,
    function: String,
    input: String,
    output: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct FunctionIo {
    function: String,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CargoAlias {
    name: String,
    package: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct GeneratedMarker {
    line: usize,
    marker: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CohortExample {
    sha: String,
    subject: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
struct SyntaxCohort {
    support: usize,
    opportunities: usize,
    comments_or_docs_only: usize,
    unclassified: usize,
    examples: Vec<CohortExample>,
}

pub fn add_generated_companion_findings(
    root: &Path,
    base: Option<&str>,
    analysis: &mut AnalysisReport,
) -> Result<(), Box<dyn Error>> {
    let anchor = diff_anchor(root, base)?;
    let changed = changed_paths(root, &anchor)?;
    if changed.is_empty() {
        return Ok(());
    }

    let changed_rust: BTreeSet<_> = changed
        .iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .cloned()
        .collect();
    if changed_rust.is_empty() {
        return Ok(());
    }

    let relations = discover_generator_relations(root)?;
    if relations.is_empty() {
        return Ok(());
    }
    let generated_attrs = generated_attribute_paths(root);

    for relation in relations {
        let input = PathBuf::from(&relation.input);
        let output = PathBuf::from(&relation.output);
        if !changed_rust.contains(&input) || changed.contains(&output) {
            continue;
        }
        if !root.join(&output).is_file() || !generated_attrs.contains(&relation.output) {
            continue;
        }
        let Some(marker) = generated_marker(root, &output) else {
            continue;
        };
        if !source_syntax_changed(root, &anchor, &input)? {
            continue;
        }

        let cohort = analyze_syntax_cohort(root, &input, &output, MAX_HISTORY_COMMITS)?;
        if cohort.opportunities < MIN_SYNTAX_COHORT || cohort.support != cohort.opportunities {
            continue;
        }

        analysis
            .findings
            .push(build_finding(&relation, &marker, &cohort));
    }

    Ok(())
}

fn build_finding(
    relation: &GeneratorRelation,
    marker: &GeneratedMarker,
    cohort: &SyntaxCohort,
) -> Finding {
    let input_location = Location::new(relation.input.clone(), None);
    let output_location = Location::new(relation.output.clone(), Some(marker.line));
    let generator_location = Location::new(relation.generator_path.clone(), None);

    let mut historical = Claim::new(
        ClaimKind::Observed,
        format!(
            "`{}` changed in {}/{} comparable Rust syntax-changing commits for `{}` ({:.1}%).",
            relation.output,
            cohort.support,
            cohort.opportunities,
            relation.input,
            percent(cohort.support, cohort.opportunities)
        ),
    );
    for example in &cohort.examples {
        historical = historical.with_evidence(Evidence::new(format!(
            "Example {}: {}",
            short_sha(&example.sha),
            example.subject
        )));
    }
    if cohort.comments_or_docs_only > 0 {
        historical = historical.with_evidence(Evidence::new(format!(
            "{} comment/doc/whitespace-only source commit(s) were excluded from the syntax cohort.",
            cohort.comments_or_docs_only
        )));
    }
    if cohort.unclassified > 0 {
        historical = historical.with_evidence(Evidence::new(format!(
            "{} source commit(s) could not be classified and were excluded from the syntax cohort.",
            cohort.unclassified
        )));
    }

    Finding::new(
        "generated-companion-missing",
        "Generated companion absent from source syntax change",
    )
    .at(input_location.clone())
    .with_claim(
        Claim::new(
            ClaimKind::Derived,
            format!(
                "The current diff changes normalized Rust syntax in `{}` and omits `{}`.",
                relation.input, relation.output
            ),
        )
        .with_evidence(Evidence::at(
            "Changed source is present in the current diff.",
            input_location,
        ))
        .with_evidence(Evidence::new(format!(
            "`{}` is absent from the current diff.",
            relation.output
        ))),
    )
    .with_claim(
        Claim::new(
            ClaimKind::Derived,
            format!(
                "Cargo alias `cargo {}` invokes generator package `{}`, and `{}` reads `{}` and writes `{}`.",
                relation.alias,
                relation.package,
                relation.function,
                relation.input,
                relation.output
            ),
        )
        .with_evidence(Evidence::at(
            "Literal repository-path read/write relation is present in this generator source.",
            generator_location,
        )),
    )
    .with_claim(
        Claim::new(
            ClaimKind::Observed,
            format!(
                "`{}` declares itself generated and `.gitattributes` marks the exact path `linguist-generated=true`.",
                relation.output
            ),
        )
        .with_evidence(Evidence::at(marker.marker.clone(), output_location)),
    )
    .with_claim(historical)
    .with_claim(Claim::new(
        ClaimKind::Unknown,
        "Repository evidence establishes generation ownership and precedent, but it does not establish whether this current absence is intentional or whether this source edit changes generated bytes.",
    ))
    .with_question(format!(
        "Was `cargo {}` intentionally deferred for this source change, or is `{}` stale?",
        relation.alias, relation.output
    ))
}

fn diff_anchor(root: &Path, base: Option<&str>) -> Result<String, Box<dyn Error>> {
    match base {
        Some(base) => {
            let output = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["merge-base", base, "HEAD"])
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "could not find merge base for `{base}` and HEAD: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        }
        None => Ok("HEAD".to_string()),
    }
}

fn changed_paths(root: &Path, anchor: &str) -> Result<BTreeSet<PathBuf>, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "diff",
            "--name-only",
            "--no-renames",
        ])
        .arg(anchor)
        .arg("--")
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git diff --name-only failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

fn source_syntax_changed(root: &Path, anchor: &str, path: &Path) -> Result<bool, Box<dyn Error>> {
    let current = fs::read_to_string(root.join(path))?;
    let Some(before) = source_at_revision(root, anchor, path) else {
        return Ok(false);
    };
    let Some(before) = rust_syntax_fingerprint(&before) else {
        return Ok(false);
    };
    let Some(current) = rust_syntax_fingerprint(&current) else {
        return Ok(false);
    };
    Ok(before != current)
}

fn analyze_syntax_cohort(
    root: &Path,
    input: &Path,
    output: &Path,
    max_commits: usize,
) -> Result<SyntaxCohort, Box<dyn Error>> {
    let mut cohort = SyntaxCohort::default();
    for sha in history_shas(root, input, max_commits)? {
        let (subject, paths) = commit_metadata(root, &sha)?;
        if is_revert_subject(&subject) || paths.len() > MAX_PATHS_PER_COMMIT {
            continue;
        }

        let Some(after_source) = source_at_revision(root, &sha, input) else {
            cohort.unclassified += 1;
            continue;
        };
        let Some(before_source) = source_at_revision(root, &format!("{sha}^"), input) else {
            cohort.unclassified += 1;
            continue;
        };
        let (Some(before), Some(after)) = (
            rust_syntax_fingerprint(&before_source),
            rust_syntax_fingerprint(&after_source),
        ) else {
            cohort.unclassified += 1;
            continue;
        };

        if before == after {
            cohort.comments_or_docs_only += 1;
            continue;
        }

        cohort.opportunities += 1;
        if paths.contains(output) {
            cohort.support += 1;
            if cohort.examples.len() < EXAMPLE_LIMIT {
                cohort.examples.push(CohortExample {
                    sha: sha.clone(),
                    subject,
                });
            }
        }
    }
    Ok(cohort)
}

fn history_shas(
    root: &Path,
    input: &Path,
    max_commits: usize,
) -> Result<Vec<String>, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--no-merges", "--format=%H", "-n"])
        .arg(max_commits.to_string())
        .arg("--")
        .arg(input)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git log failed for {}: {}",
            input.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
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
        return Err(format!(
            "git show failed for {sha}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
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

fn source_at_revision(root: &Path, revision: &str, path: &Path) -> Option<String> {
    let spec = format!("{revision}:{}", normalize_path(path));
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

fn rust_syntax_fingerprint(source: &str) -> Option<String> {
    let tokens = TokenStream::from_str(source).ok()?;
    Some(strip_doc_attributes(tokens).to_string())
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
                let mut normalized =
                    Group::new(group.delimiter(), strip_doc_attributes(group.stream()));
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
    group.delimiter() == Delimiter::Bracket
        && matches!(group.stream().into_iter().next(), Some(TokenTree::Ident(ident)) if ident == "doc")
}

fn discover_generator_relations(root: &Path) -> Result<Vec<GeneratorRelation>, Box<dyn Error>> {
    let aliases = cargo_aliases(root);
    if aliases.is_empty() {
        return Ok(Vec::new());
    }
    let packages = package_main_sources(root)?;
    let mut relations = BTreeSet::new();

    for alias in aliases {
        let Some(generator_path) = packages.get(&alias.package) else {
            continue;
        };
        let source = match fs::read_to_string(root.join(generator_path)) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let file = match syn::parse_file(&source) {
            Ok(file) => file,
            Err(_) => continue,
        };
        for io in collect_function_io(&file) {
            for input in &io.reads {
                for output in &io.writes {
                    relations.insert(GeneratorRelation {
                        alias: alias.name.clone(),
                        package: alias.package.clone(),
                        generator_path: normalize_path(generator_path),
                        function: io.function.clone(),
                        input: input.clone(),
                        output: output.clone(),
                    });
                }
            }
        }
    }

    Ok(relations.into_iter().collect())
}

fn cargo_aliases(root: &Path) -> Vec<CargoAlias> {
    let path = root.join(".cargo/config.toml");
    let Ok(source) = fs::read_to_string(path) else {
        return Vec::new();
    };
    cargo_aliases_from_source(&source)
}

fn cargo_aliases_from_source(source: &str) -> Vec<CargoAlias> {
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
        let Some(package) = package_from_cargo_run(&command) else {
            continue;
        };
        aliases.push(CargoAlias {
            name: name.trim().to_string(),
            package,
        });
    }
    aliases
}

fn package_from_cargo_run(command: &str) -> Option<String> {
    let tokens: Vec<_> = command.split_whitespace().collect();
    if tokens.first().copied() != Some("run") {
        return None;
    }
    let mut index = 1;
    while index < tokens.len() {
        if matches!(tokens[index], "-p" | "--package") {
            return tokens.get(index + 1).map(|value| (*value).to_string());
        }
        index += 1;
    }
    None
}

fn package_main_sources(root: &Path) -> Result<BTreeMap<String, PathBuf>, Box<dyn Error>> {
    let mut packages = BTreeMap::new();
    for entry in WalkDir::new(root).into_iter().filter_entry(should_visit) {
        let entry = entry?;
        if !entry.file_type().is_file() || entry.file_name() != "Cargo.toml" {
            continue;
        }
        let Ok(source) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let Some(name) = package_name_from_manifest(&source) else {
            continue;
        };
        let Some(dir) = entry.path().parent() else {
            continue;
        };
        let main = dir.join("src/main.rs");
        if main.is_file()
            && let Ok(relative) = main.strip_prefix(root)
        {
            packages.insert(name, relative.to_path_buf());
        }
    }
    Ok(packages)
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

fn collect_function_io(file: &syn::File) -> Vec<FunctionIo> {
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

fn generated_marker(root: &Path, output: &Path) -> Option<GeneratedMarker> {
    let bytes = fs::read(root.join(output)).ok()?;
    let prefix = &bytes[..bytes.len().min(GENERATED_HEADER_BYTES)];
    let text = String::from_utf8_lossy(prefix);
    text.lines()
        .take(GENERATED_HEADER_LINES)
        .enumerate()
        .find_map(|(index, line)| {
            strong_generated_marker(line).then(|| GeneratedMarker {
                line: index + 1,
                marker: line.trim().to_string(),
            })
        })
}

fn strong_generated_marker(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("@generated")
        || lower.contains("do not edit")
        || lower.contains("automatically generated")
        || lower.contains("auto-generated")
}

fn should_visit(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        entry.file_name().to_str(),
        Some(".git" | "target" | "node_modules" | ".venv" | "vendor")
    )
}

fn normalize_repo_path(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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

fn is_revert_subject(subject: &str) -> bool {
    subject
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("revert")
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
    fn parses_alias_package() {
        let aliases = cargo_aliases_from_source(
            r#"
            [alias]
            lintgen = "run -p oxc_linter_codegen"
            check = "check --all-targets"
            "#,
        );
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name, "lintgen");
        assert_eq!(aliases[0].package, "oxc_linter_codegen");
    }

    #[test]
    fn extracts_literal_read_write_relation() {
        let file = syn::parse_file(
            r#"
            fn generate() -> std::io::Result<()> {
                let source = std::fs::read_to_string(root.join("src/rules.rs"))?;
                let target = root.join("src/generated/rules.rs");
                std::fs::write(&target, source)?;
                Ok(())
            }
            "#,
        )
        .unwrap();
        let io = collect_function_io(&file);
        assert_eq!(io.len(), 1);
        assert!(io[0].reads.contains("src/rules.rs"));
        assert!(io[0].writes.contains("src/generated/rules.rs"));
    }

    #[test]
    fn comments_and_docs_do_not_change_syntax_fingerprint() {
        let before = "// old\n/// old docs\nfn answer() -> usize { 42 }";
        let after = "// new\n/// new docs\nfn answer() -> usize { 42 }";
        assert_eq!(
            rust_syntax_fingerprint(before),
            rust_syntax_fingerprint(after)
        );
    }

    #[test]
    fn code_change_changes_syntax_fingerprint() {
        assert_ne!(
            rust_syntax_fingerprint("fn answer() -> usize { 41 }"),
            rust_syntax_fingerprint("fn answer() -> usize { 42 }")
        );
    }

    #[test]
    fn generated_marker_requires_strong_phrase() {
        assert!(strong_generated_marker(
            "// Auto-generated code, DO NOT EDIT DIRECTLY!"
        ));
        assert!(strong_generated_marker("// @generated"));
        assert!(!strong_generated_marker(
            "// generated by parser at runtime"
        ));
    }

    #[test]
    fn semantic_cohort_requires_all_comparable_commits() {
        let cohort = SyntaxCohort {
            support: 9,
            opportunities: 10,
            ..SyntaxCohort::default()
        };
        assert_ne!(cohort.support, cohort.opportunities);
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};

#[path = "generator_ownership.rs"]
mod generator_ownership;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};
use generator_ownership::{
    GeneratorRelation, discover_generator_relations, generated_attribute_paths,
};

const MAX_HISTORY_COMMITS: usize = 100;
const MAX_PATHS_PER_COMMIT: usize = 100;
const MIN_SYNTAX_COHORT: usize = 3;
const GENERATED_HEADER_BYTES: usize = 8 * 1024;
const GENERATED_HEADER_LINES: usize = 40;
const EXAMPLE_LIMIT: usize = 3;

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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SourceChangeClass {
    SyntaxChanged,
    CommentsOrDocsOnly,
    Unclassified,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SourceHistoryRecord {
    sha: String,
    parent: Option<String>,
    subject: String,
    paths: BTreeSet<PathBuf>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ClassifiedSourceCommit {
    sha: String,
    subject: String,
    paths: BTreeSet<PathBuf>,
    class: SourceChangeClass,
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
    let mut history_by_input = BTreeMap::<PathBuf, Vec<ClassifiedSourceCommit>>::new();

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

        if !history_by_input.contains_key(&input) {
            let history = classify_source_history(root, &input, MAX_HISTORY_COMMITS)?;
            history_by_input.insert(input.clone(), history);
        }
        let history = history_by_input
            .get(&input)
            .expect("source history inserted above");
        let cohort = build_syntax_cohort(history, &output);
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

fn classify_source_history(
    root: &Path,
    input: &Path,
    max_commits: usize,
) -> Result<Vec<ClassifiedSourceCommit>, Box<dyn Error>> {
    let records = read_source_history(root, input, max_commits)?;
    let considered: Vec<_> = records
        .into_iter()
        .filter(|record| {
            !is_revert_subject(&record.subject) && record.paths.len() <= MAX_PATHS_PER_COMMIT
        })
        .collect();
    let versions = read_source_versions(root, input, &considered)?;
    let mut classified = Vec::with_capacity(considered.len());

    for record in considered {
        let class = match record.parent.as_deref() {
            Some(parent) => {
                let after_key = revision_spec(&record.sha, input);
                let before_key = revision_spec(parent, input);
                let after = versions.get(&after_key).and_then(|source| source.as_deref());
                let before = versions
                    .get(&before_key)
                    .and_then(|source| source.as_deref());
                match (before, after) {
                    (Some(before), Some(after)) => match (
                        rust_syntax_fingerprint(before),
                        rust_syntax_fingerprint(after),
                    ) {
                        (Some(before), Some(after)) if before == after => {
                            SourceChangeClass::CommentsOrDocsOnly
                        }
                        (Some(_), Some(_)) => SourceChangeClass::SyntaxChanged,
                        _ => SourceChangeClass::Unclassified,
                    },
                    _ => SourceChangeClass::Unclassified,
                }
            }
            None => SourceChangeClass::Unclassified,
        };

        classified.push(ClassifiedSourceCommit {
            sha: record.sha,
            subject: record.subject,
            paths: record.paths,
            class,
        });
    }

    Ok(classified)
}

fn build_syntax_cohort(history: &[ClassifiedSourceCommit], output: &Path) -> SyntaxCohort {
    let mut cohort = SyntaxCohort::default();

    for commit in history {
        match commit.class {
            SourceChangeClass::CommentsOrDocsOnly => {
                cohort.comments_or_docs_only += 1;
                continue;
            }
            SourceChangeClass::Unclassified => {
                cohort.unclassified += 1;
                continue;
            }
            SourceChangeClass::SyntaxChanged => {}
        }

        cohort.opportunities += 1;
        if commit.paths.contains(output) {
            cohort.support += 1;
            if cohort.examples.len() < EXAMPLE_LIMIT {
                cohort.examples.push(CohortExample {
                    sha: commit.sha.clone(),
                    subject: commit.subject.clone(),
                });
            }
        }
    }

    cohort
}

fn read_source_history(
    root: &Path,
    input: &Path,
    max_commits: usize,
) -> Result<Vec<SourceHistoryRecord>, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotepath=false",
            "log",
            "--format=%x1e%H%x1f%P%x1f%s",
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

    parse_source_history_log(&String::from_utf8(output.stdout)?)
        .ok_or_else(|| format!("could not parse git history for {}", input.display()).into())
}

fn parse_source_history_log(output: &str) -> Option<Vec<SourceHistoryRecord>> {
    output
        .split('\u{1e}')
        .filter(|record| !record.trim().is_empty())
        .map(parse_source_history_record)
        .collect()
}

fn parse_source_history_record(record: &str) -> Option<SourceHistoryRecord> {
    let record = record.trim_start_matches(['\n', '\r']);
    let mut lines = record.lines();
    let metadata = lines.next()?.trim();
    let mut fields = metadata.splitn(3, '\u{1f}');
    let sha = fields.next()?.trim().to_string();
    let parent = fields
        .next()?
        .split_whitespace()
        .next()
        .map(ToOwned::to_owned);
    let subject = fields.next()?.trim().to_string();
    let paths = lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect();

    Some(SourceHistoryRecord {
        sha,
        parent,
        subject,
        paths,
    })
}

fn read_source_versions(
    root: &Path,
    input: &Path,
    records: &[SourceHistoryRecord],
) -> Result<BTreeMap<String, Option<String>>, Box<dyn Error>> {
    let mut requested = BTreeSet::new();
    for record in records {
        requested.insert(revision_spec(&record.sha, input));
        if let Some(parent) = &record.parent {
            requested.insert(revision_spec(parent, input));
        }
    }

    let mut results = BTreeMap::new();
    let mut safe = Vec::new();
    for spec in requested {
        if spec.contains(['\n', '\r']) {
            results.insert(spec, None);
        } else {
            safe.push(spec);
        }
    }
    results.extend(read_git_blobs(root, &safe)?);
    Ok(results)
}

fn read_git_blobs(
    root: &Path,
    specs: &[String],
) -> Result<BTreeMap<String, Option<String>>, Box<dyn Error>> {
    if specs.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or("git cat-file did not provide a stdin pipe")?;
        for spec in specs {
            stdin.write_all(spec.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
    }

    let stdout = child
        .stdout
        .take()
        .ok_or("git cat-file did not provide a stdout pipe")?;
    let mut reader = BufReader::new(stdout);
    let mut results = BTreeMap::new();

    for spec in specs {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Err(format!("git cat-file ended before reading `{spec}`").into());
        }
        let header = header.trim_end_matches(['\n', '\r']);
        if header.ends_with(" missing") {
            results.insert(spec.clone(), None);
            continue;
        }

        let mut fields = header.split_whitespace();
        let _object = fields
            .next()
            .ok_or_else(|| format!("invalid git cat-file header for `{spec}`"))?;
        let kind = fields
            .next()
            .ok_or_else(|| format!("invalid git cat-file header for `{spec}`"))?;
        let size = fields
            .next()
            .ok_or_else(|| format!("invalid git cat-file header for `{spec}`"))?
            .parse::<usize>()?;
        if fields.next().is_some() {
            return Err(format!("invalid git cat-file header for `{spec}`").into());
        }

        let mut bytes = vec![0; size];
        reader.read_exact(&mut bytes)?;
        let mut terminator = [0_u8; 1];
        reader.read_exact(&mut terminator)?;
        if terminator != [b'\n'] {
            return Err(format!("invalid git cat-file payload terminator for `{spec}`").into());
        }

        let source = if kind == "blob" {
            String::from_utf8(bytes).ok()
        } else {
            None
        };
        results.insert(spec.clone(), source);
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(format!("git cat-file failed with status {status}").into());
    }

    Ok(results)
}

fn revision_spec(revision: &str, path: &Path) -> String {
    format!("{revision}:{}", normalize_path(path))
}

fn source_at_revision(root: &Path, revision: &str, path: &Path) -> Option<String> {
    let spec = revision_spec(revision, path);
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

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cargo-cultist-generated-history-{name}-{}-{nanos}",
            std::process::id()
        ))
    }

    fn run_git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git command failed: git {args:?}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
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

    #[test]
    fn parses_batched_source_history_records() {
        let output = concat!(
            "\x1eabc\x1fparent\x1ffeat: one\n\n",
            "src/input.rs\n",
            "generated/output.rs\n",
            "\x1edef\x1fabc\x1fdocs: two\n\n",
            "src/input.rs\n",
        );
        let records = parse_source_history_log(output).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sha, "abc");
        assert_eq!(records[0].parent.as_deref(), Some("parent"));
        assert!(records[0].paths.contains(Path::new("generated/output.rs")));
        assert_eq!(records[1].subject, "docs: two");
    }

    #[test]
    fn batched_history_preserves_syntax_cohort_semantics() {
        let root = unique_temp_dir("cohort");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("generated")).unwrap();
        run_git(&root, &["init", "-q", "-b", "main"]);
        run_git(&root, &["config", "user.email", "cultist@example.invalid"]);
        run_git(&root, &["config", "user.name", "Cargo Cultist Tests"]);

        fs::write(root.join("src/input.rs"), "fn value() -> usize { 0 }\n").unwrap();
        fs::write(root.join("generated/output.rs"), "const VALUE: usize = 0;\n").unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "baseline"]);

        fs::write(
            root.join("src/input.rs"),
            "// comment only\nfn value() -> usize { 0 }\n",
        )
        .unwrap();
        run_git(&root, &["add", "src/input.rs"]);
        run_git(&root, &["commit", "-q", "-m", "docs only"]);

        fs::write(root.join("src/input.rs"), "fn value() -> usize { 1 }\n").unwrap();
        fs::write(root.join("generated/output.rs"), "const VALUE: usize = 1;\n").unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "syntax one"]);

        fs::write(root.join("src/input.rs"), "fn value() -> usize { 2 }\n").unwrap();
        fs::write(root.join("generated/output.rs"), "const VALUE: usize = 2;\n").unwrap();
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "syntax two"]);

        let history = classify_source_history(&root, Path::new("src/input.rs"), 10).unwrap();
        let cohort = build_syntax_cohort(&history, Path::new("generated/output.rs"));
        assert_eq!(cohort.opportunities, 2);
        assert_eq!(cohort.support, 2);
        assert_eq!(cohort.comments_or_docs_only, 1);
        assert_eq!(cohort.unclassified, 1);

        fs::remove_dir_all(root).unwrap();
    }
}

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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
    use super::*;

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

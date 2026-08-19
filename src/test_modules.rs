use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::Span;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, ItemMod, Meta, Token};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TestModuleOccurrence {
    pub name: String,
    pub path: PathBuf,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct TestModuleReport {
    pub occurrences: Vec<TestModuleOccurrence>,
    pub parse_failures: Vec<(PathBuf, String)>,
}

impl TestModuleReport {
    pub fn extend(&mut self, other: Self) {
        self.occurrences.extend(other.occurrences);
        self.parse_failures.extend(other.parse_failures);
        sort_report(self);
    }
}

pub fn analyze_test_modules(root: &Path) -> Result<TestModuleReport, Box<dyn Error>> {
    analyze_test_modules_excluding(root, &BTreeSet::new())
}

pub fn analyze_test_modules_excluding(
    root: &Path,
    excluded_paths: &BTreeSet<PathBuf>,
) -> Result<TestModuleReport, Box<dyn Error>> {
    let mut report = TestModuleReport::default();

    for entry in WalkDir::new(root).into_iter().filter_entry(should_visit) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs")
            || excluded_paths.contains(entry.path())
        {
            continue;
        }

        analyze_test_module_file(entry.path(), &mut report)?;
    }

    sort_report(&mut report);
    Ok(report)
}

pub fn analyze_test_module_files(paths: &[PathBuf]) -> Result<TestModuleReport, Box<dyn Error>> {
    let mut report = TestModuleReport::default();

    for path in paths {
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") || !path.is_file() {
            continue;
        }
        analyze_test_module_file(path, &mut report)?;
    }

    sort_report(&mut report);
    Ok(report)
}

fn analyze_test_module_file(
    path: &Path,
    report: &mut TestModuleReport,
) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    match syn::parse_file(&source) {
        Ok(file) => {
            let mut visitor = TestModuleVisitor {
                path,
                occurrences: &mut report.occurrences,
            };
            visitor.visit_file(&file);
        }
        Err(error) => report
            .parse_failures
            .push((path.to_path_buf(), error.to_string())),
    }
    Ok(())
}

fn sort_report(report: &mut TestModuleReport) {
    report
        .occurrences
        .sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    report.parse_failures.sort_by(|a, b| a.0.cmp(&b.0));
}

fn should_visit(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }

    !matches!(
        entry.file_name().to_str(),
        Some(".git" | "target" | "node_modules")
    )
}

struct TestModuleVisitor<'a> {
    path: &'a Path,
    occurrences: &'a mut Vec<TestModuleOccurrence>,
}

impl<'ast> Visit<'ast> for TestModuleVisitor<'_> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        if is_test_module(&node.attrs) {
            self.occurrences.push(TestModuleOccurrence {
                name: node.ident.to_string(),
                path: self.path.to_path_buf(),
                line: span_line(node.ident.span()),
            });
        }

        visit::visit_item_mod(self, node);
    }
}

fn span_line(span: Span) -> usize {
    span.start().line
}

fn is_test_module(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .any(|attr| match &attr.meta {
            Meta::List(list) => parse_meta_list(list.tokens.clone())
                .is_some_and(|metas| metas.iter().any(|meta| meta_mentions_test(meta, false))),
            _ => false,
        })
}

fn parse_meta_list(tokens: proc_macro2::TokenStream) -> Option<Punctuated<Meta, Token![,]>> {
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(tokens)
        .ok()
}

fn meta_mentions_test(meta: &Meta, negated: bool) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("test") && !negated,
        Meta::List(list) => {
            let nested_negated = if list.path.is_ident("not") {
                !negated
            } else {
                negated
            };
            parse_meta_list(list.tokens.clone()).is_some_and(|metas| {
                metas
                    .iter()
                    .any(|meta| meta_mentions_test(meta, nested_negated))
            })
        }
        Meta::NameValue(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn names(source: &str) -> Vec<String> {
        let file = syn::parse_file(source).unwrap();
        let path = Path::new("fixture.rs");
        let mut occurrences = Vec::new();
        let mut visitor = TestModuleVisitor {
            path,
            occurrences: &mut occurrences,
        };
        visitor.visit_file(&file);
        occurrences
            .into_iter()
            .map(|occurrence| occurrence.name)
            .collect()
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

    #[test]
    fn finds_test_gated_modules() {
        assert_eq!(
            names(
                r#"
                #[cfg(test)]
                mod tests {}

                #[cfg(all(unix, test))]
                mod unix_tests {}
                "#,
            ),
            vec!["tests".to_string(), "unix_tests".to_string()]
        );
    }

    #[test]
    fn ignores_ordinary_and_not_test_modules() {
        assert!(names("mod production {}").is_empty());
        assert!(names("#[cfg(not(test))] mod production {}").is_empty());
    }

    #[test]
    fn targeted_scan_reads_only_requested_files() {
        let root = unique_temp_dir("targeted-scan");
        fs::create_dir_all(&root).unwrap();
        let selected = root.join("selected.rs");
        let ignored = root.join("ignored.rs");
        fs::write(&selected, "#[cfg(test)]\nmod selected_tests {}\n").unwrap();
        fs::write(&ignored, "this is deliberately invalid Rust {{{").unwrap();

        let report = analyze_test_module_files(std::slice::from_ref(&selected)).unwrap();

        assert_eq!(report.occurrences.len(), 1);
        assert_eq!(report.occurrences[0].name, "selected_tests");
        assert!(report.parse_failures.is_empty());

        fs::remove_dir_all(root).unwrap();
    }
}

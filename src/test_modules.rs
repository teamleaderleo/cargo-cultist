use std::collections::{BTreeMap, BTreeSet};
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

pub fn analyze_test_modules(root: &Path) -> Result<TestModuleReport, Box<dyn Error>> {
    let mut report = TestModuleReport::default();

    for entry in WalkDir::new(root).into_iter().filter_entry(should_visit) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs")
        {
            continue;
        }

        let path = entry.path();
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
    }

    report
        .occurrences
        .sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    report.parse_failures.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(report)
}

pub fn print_test_module_report(root: &Path, report: &TestModuleReport) {
    println!("TEST MODULE CONVENTIONS");

    if report.occurrences.is_empty() {
        println!("  No test-gated modules found.");
        print_parse_failures(root, report);
        return;
    }

    let mut counts = BTreeMap::<&str, usize>::new();
    for occurrence in &report.occurrences {
        *counts.entry(&occurrence.name).or_default() += 1;
    }

    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by(|(name_a, count_a), (name_b, count_b)| {
        count_b.cmp(count_a).then(name_a.cmp(name_b))
    });

    for (name, count) in &counts {
        println!("  {name:<24} {count}");
    }

    println!("\nOBSERVATION");
    if counts.len() == 1 {
        println!(
            "  This repository consistently names its test-gated modules `{}`.",
            counts[0].0
        );
    } else {
        let total = report.occurrences.len();
        let dominant_count = counts[0].1;
        let dominant_names: Vec<_> = counts
            .iter()
            .take_while(|(_, count)| *count == dominant_count)
            .map(|(name, _)| *name)
            .collect();

        if dominant_names.len() == 1 {
            println!(
                "  `{}` is the most frequent name ({} of {} modules), but the repository uses {} names.",
                dominant_names[0],
                dominant_count,
                total,
                counts.len()
            );
        } else {
            println!(
                "  No single name dominates: {} names are tied at {} occurrences each.",
                dominant_names.len(),
                dominant_count
            );
            println!("  The repository uses {} names overall.", counts.len());
        }
    }

    print_local_mixes(root, report);
    print_one_off_names(root, report, &counts);
    print_parse_failures(root, report);
}

fn print_local_mixes(root: &Path, report: &TestModuleReport) {
    let mut by_file = BTreeMap::<&PathBuf, Vec<&TestModuleOccurrence>>::new();
    for occurrence in &report.occurrences {
        by_file
            .entry(&occurrence.path)
            .or_default()
            .push(occurrence);
    }

    let mixed_files: Vec<_> = by_file
        .into_iter()
        .filter(|(_, occurrences)| {
            occurrences
                .iter()
                .map(|occurrence| occurrence.name.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        })
        .collect();

    if mixed_files.is_empty() {
        return;
    }

    println!("\nLOCAL MIX");
    println!("  These files use more than one test-module name:");
    for (path, occurrences) in mixed_files {
        let path = path.strip_prefix(root).unwrap_or(path);
        println!("    {}", path.display());
        for occurrence in occurrences {
            println!("      line {:>5}: mod {}", occurrence.line, occurrence.name);
        }
    }
    println!("\nQUESTION");
    println!("  Is the local mix deliberate, or would one name make the file easier to read?");
}

fn print_one_off_names(root: &Path, report: &TestModuleReport, counts: &[(&str, usize)]) {
    let one_off_names: Vec<_> = counts
        .iter()
        .filter(|(_, count)| *count == 1)
        .map(|(name, _)| *name)
        .collect();

    if one_off_names.is_empty() {
        return;
    }

    println!("\nONE-OFF NAMES");
    for occurrence in &report.occurrences {
        if one_off_names.contains(&occurrence.name.as_str()) {
            let path = occurrence
                .path
                .strip_prefix(root)
                .unwrap_or(&occurrence.path);
            println!(
                "  {}:{}  mod {}",
                path.display(),
                occurrence.line,
                occurrence.name
            );
        }
    }

    println!("\nQUESTION");
    println!(
        "  Are these one-off names intentionally scoped, or accidental deviations from local precedent?"
    );
}

fn print_parse_failures(root: &Path, report: &TestModuleReport) {
    if report.parse_failures.is_empty() {
        return;
    }

    println!("\nPARSE NOTES");
    for (path, error) in &report.parse_failures {
        let path = path.strip_prefix(root).unwrap_or(path);
        println!("  {}: {error}", path.display());
    }
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
}

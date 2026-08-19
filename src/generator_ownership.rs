#![allow(dead_code)]

// This file is intentionally compiled by product and research consumers with
// complementary entry points; each consumer leaves part of the shared API unused.
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprLit, ExprMethodCall, File, ItemFn, Lit, Pat, Stmt};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct GeneratorRelation {
    pub alias: String,
    pub package: String,
    pub generator_path: String,
    pub function: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionIo {
    pub function: String,
    pub reads: BTreeSet<String>,
    pub writes: BTreeSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CargoAlias {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeneratorSourceReport {
    pub generator_path: String,
    pub package: Option<String>,
    pub aliases: Vec<CargoAlias>,
    pub functions: Vec<FunctionIo>,
    pub generated_attributes: BTreeSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum RepositoryPathValue {
    Root,
    Relative(String),
}

#[derive(Debug, Default)]
struct PathBindings {
    values: BTreeMap<String, RepositoryPathValue>,
}

pub fn discover_generator_relations(root: &Path) -> Result<Vec<GeneratorRelation>, Box<dyn Error>> {
    let aliases = cargo_run_aliases(root);
    if aliases.is_empty() {
        return Ok(Vec::new());
    }
    let packages = package_main_sources(root)?;
    let mut relations = BTreeSet::new();

    for (alias, package) in aliases {
        let Some(generator_path) = packages.get(&package) else {
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
                        alias: alias.clone(),
                        package: package.clone(),
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

pub fn analyze_generator_source(
    root: &Path,
    generator: &Path,
) -> Result<GeneratorSourceReport, Box<dyn Error>> {
    if generator.is_absolute() {
        return Err("GENERATOR_RS must be repository-relative".into());
    }
    let generator_path = root.join(generator);
    let source = fs::read_to_string(&generator_path)?;
    let file = syn::parse_file(&source)?;
    let package = nearest_package_name(root, &generator_path);
    let aliases = package
        .as_deref()
        .map(|package| cargo_aliases_for_package(root, package))
        .unwrap_or_default();

    Ok(GeneratorSourceReport {
        generator_path: normalize_path(generator),
        package,
        aliases,
        functions: collect_function_io(&file),
        generated_attributes: generated_attribute_paths(root),
    })
}

pub fn generated_attribute_paths(root: &Path) -> BTreeSet<String> {
    let path = root.join(".gitattributes");
    let Ok(source) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    generated_attribute_paths_from_source(&source)
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

fn collect_path_bindings(function: &ItemFn) -> PathBindings {
    let mut bindings = PathBindings::default();
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
        if let Some(value) = resolve_repository_path_value(&init.expr, &bindings) {
            bindings.values.insert(pat.ident.to_string(), value);
        }
    }
    bindings
}

struct IoVisitor<'a> {
    bindings: &'a PathBindings,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for IoVisitor<'_> {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Some(kind) = fs_call_kind(&call.func)
            && let Some(first) = call.args.first()
            && let Some(path) = resolve_file_path(first, self.bindings)
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

fn resolve_file_path(expr: &Expr, bindings: &PathBindings) -> Option<String> {
    match resolve_repository_path_value(expr, bindings)? {
        RepositoryPathValue::Root => None,
        RepositoryPathValue::Relative(path) => Some(path),
    }
}

fn resolve_repository_path_value(
    expr: &Expr,
    bindings: &PathBindings,
) -> Option<RepositoryPathValue> {
    match expr {
        Expr::Reference(reference) => resolve_repository_path_value(&reference.expr, bindings),
        Expr::Paren(paren) => resolve_repository_path_value(&paren.expr, bindings),
        Expr::Group(group) => resolve_repository_path_value(&group.expr, bindings),
        Expr::Try(try_expr) => resolve_repository_path_value(&try_expr.expr, bindings),
        Expr::Call(call) if is_repository_root_provider(&call.func) => {
            Some(RepositoryPathValue::Root)
        }
        Expr::MethodCall(call) if call.method == "join" => resolve_join(call, bindings),
        Expr::MethodCall(call) if call.method == "map_err" => {
            resolve_repository_path_value(&call.receiver, bindings)
        }
        Expr::MethodCall(call) if call.method == "unwrap" || call.method == "expect" => {
            resolve_repository_path_value(&call.receiver, bindings)
        }
        Expr::Path(path) if path.path.segments.len() == 1 => bindings
            .values
            .get(&path.path.segments[0].ident.to_string())
            .cloned(),
        _ => None,
    }
}

fn resolve_join(call: &ExprMethodCall, bindings: &PathBindings) -> Option<RepositoryPathValue> {
    if call.args.len() != 1 {
        return None;
    }
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = call.args.first()?
    else {
        return None;
    };
    let suffix = normalize_repo_literal(&value.value())?;
    let base = resolve_repository_path_value(&call.receiver, bindings)?;
    match base {
        RepositoryPathValue::Root => Some(RepositoryPathValue::Relative(suffix)),
        RepositoryPathValue::Relative(prefix) => {
            Some(RepositoryPathValue::Relative(join_repo_paths(&prefix, &suffix)))
        }
    }
}

fn is_repository_root_provider(expr: &Expr) -> bool {
    let Expr::Path(path) = expr else {
        return false;
    };
    let segments: Vec<_> = path.path.segments.iter().collect();
    segments.len() >= 2
        && segments[segments.len() - 2].ident == "project_root"
        && segments[segments.len() - 1].ident == "get_project_root"
}

fn normalize_repo_literal(value: &str) -> Option<String> {
    let normalized = value.replace('\\', "/");
    let mut parts = Vec::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn join_repo_paths(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() {
        suffix.to_string()
    } else {
        format!("{prefix}/{suffix}")
    }
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

fn cargo_aliases_for_package(root: &Path, package: &str) -> Vec<CargoAlias> {
    let path = root.join(".cargo/config.toml");
    let Ok(source) = fs::read_to_string(path) else {
        return Vec::new();
    };
    cargo_aliases_from_config(&source)
        .into_iter()
        .filter(|alias| alias.command.contains(package))
        .collect()
}

fn cargo_run_aliases(root: &Path) -> Vec<(String, String)> {
    let path = root.join(".cargo/config.toml");
    let Ok(source) = fs::read_to_string(path) else {
        return Vec::new();
    };
    cargo_aliases_from_config(&source)
        .into_iter()
        .filter_map(|alias| {
            package_from_cargo_run(&alias.command).map(|package| (alias.name, package))
        })
        .collect()
}

fn cargo_aliases_from_config(source: &str) -> Vec<CargoAlias> {
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
        aliases.push(CargoAlias {
            name: name.trim().to_string(),
            command,
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

fn generated_attribute_paths_from_source(source: &str) -> BTreeSet<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_literal_join_paths_from_proven_repository_root() {
        let file = syn::parse_file(
            r#"
            fn generate() -> std::io::Result<()> {
                let root = project_root::get_project_root()
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
                let source = std::fs::read_to_string(root.join("src/rules.rs"))?;
                let target = root.join("src/generated/rules.rs");
                std::fs::write(&target, source)?;
                Ok(())
            }
            "#,
        )
        .unwrap();
        let functions = collect_function_io(&file);
        assert_eq!(functions.len(), 1);
        assert!(functions[0].reads.contains("src/rules.rs"));
        assert!(functions[0].writes.contains("src/generated/rules.rs"));
    }

    #[test]
    fn preserves_prefix_for_paths_derived_from_repository_root() {
        let file = syn::parse_file(
            r#"
            fn generate() -> std::io::Result<()> {
                let tasks = project_root::get_project_root()?.join("tasks");
                let source = std::fs::read_to_string(tasks.join("src/rules.rs"))?;
                let target = tasks.join("generated/rules.rs");
                std::fs::write(&target, source)?;
                Ok(())
            }
            "#,
        )
        .unwrap();
        let functions = collect_function_io(&file);
        assert!(functions[0].reads.contains("tasks/src/rules.rs"));
        assert!(functions[0].writes.contains("tasks/generated/rules.rs"));
        assert!(!functions[0].reads.contains("src/rules.rs"));
        assert!(!functions[0].writes.contains("generated/rules.rs"));
    }

    #[test]
    fn ignores_unproven_join_receivers_even_when_named_root() {
        let file = syn::parse_file(
            r#"
            fn generate(root: std::path::PathBuf) -> std::io::Result<()> {
                let source = std::fs::read_to_string(root.join("src/rules.rs"))?;
                let target = root.join("src/generated/rules.rs");
                std::fs::write(&target, source)?;
                Ok(())
            }
            "#,
        )
        .unwrap();
        let functions = collect_function_io(&file);
        assert!(functions[0].reads.is_empty());
        assert!(functions[0].writes.is_empty());
    }

    #[test]
    fn ignores_collection_and_string_joins_as_repository_paths() {
        let file = syn::parse_file(
            r#"
            fn generate(parts: Vec<&str>, names: Vec<&str>) -> std::io::Result<()> {
                let separator = parts.join("/");
                std::fs::read_to_string(&separator)?;
                let display = names.join("generated/output.rs");
                std::fs::write(&display, "x")?;
                Ok(())
            }
            "#,
        )
        .unwrap();
        let functions = collect_function_io(&file);
        assert!(functions[0].reads.is_empty());
        assert!(functions[0].writes.is_empty());
    }

    #[test]
    fn ignores_non_fs_calls_with_similar_names() {
        let file = syn::parse_file(
            r#"
            fn generate() -> std::io::Result<()> {
                let root = project_root::get_project_root()?;
                let target = root.join("src/generated/rules.rs");
                custom::write(&target, "x");
                parser::read(root.join("src/rules.rs"));
                Ok(())
            }
            "#,
        )
        .unwrap();
        let functions = collect_function_io(&file);
        assert!(functions[0].reads.is_empty());
        assert!(functions[0].writes.is_empty());
    }

    #[test]
    fn ignores_dynamic_join_paths() {
        let file = syn::parse_file(
            r#"
            fn generate(name: &str) -> std::io::Result<()> {
                let root = project_root::get_project_root()?;
                let target = root.join(name);
                std::fs::write(&target, "x")?;
                Ok(())
            }
            "#,
        )
        .unwrap();
        let functions = collect_function_io(&file);
        assert!(functions[0].writes.is_empty());
    }

    #[test]
    fn rejects_parent_directory_escape_in_literal_join() {
        let file = syn::parse_file(
            r#"
            fn generate() -> std::io::Result<()> {
                let root = project_root::get_project_root()?;
                let source = std::fs::read_to_string(root.join("../outside.rs"))?;
                Ok(())
            }
            "#,
        )
        .unwrap();
        let functions = collect_function_io(&file);
        assert!(functions[0].reads.is_empty());
    }

    #[test]
    fn parses_package_name_and_aliases() {
        let manifest = "[package]\nname = \"generator_task\"\nversion = \"0.0.0\"\n";
        assert_eq!(
            package_name_from_manifest(manifest).as_deref(),
            Some("generator_task")
        );

        let config = "[alias]\ngen = \"run -p generator_task\"\ncheck = \"check\"\n";
        let aliases = cargo_aliases_from_config(config);
        assert_eq!(aliases[0].name, "gen");
        assert_eq!(aliases[0].command, "run -p generator_task");
        assert_eq!(
            package_from_cargo_run(&aliases[0].command).as_deref(),
            Some("generator_task")
        );
    }

    #[test]
    fn parses_generated_gitattributes() {
        let attrs = r#"
        # comment
        src/generated/a.rs linguist-generated=true merge=ours
        src/normal.rs text=auto
        src/generated/b.rs linguist-generated=true
        "#;
        let paths = generated_attribute_paths_from_source(attrs);
        assert!(paths.contains("src/generated/a.rs"));
        assert!(paths.contains("src/generated/b.rs"));
        assert!(!paths.contains("src/normal.rs"));
    }
}

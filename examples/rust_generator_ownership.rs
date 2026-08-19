use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprLit, ExprMethodCall, File, ItemFn, Lit, Pat, Stmt};

#[derive(Debug, Clone, Eq, PartialEq)]
struct FunctionIo {
    function: String,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CargoAlias {
    name: String,
    command: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("rust-generator-ownership: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: rust_generator_ownership REPO GENERATOR_RS")?,
    )
    .canonicalize()?;
    let generator = PathBuf::from(
        args.next()
            .ok_or("usage: rust_generator_ownership REPO GENERATOR_RS")?,
    );
    if args.next().is_some() {
        return Err("usage: rust_generator_ownership REPO GENERATOR_RS".into());
    }
    if generator.is_absolute() {
        return Err("GENERATOR_RS must be repository-relative".into());
    }

    let generator_path = root.join(&generator);
    let source = fs::read_to_string(&generator_path)?;
    let file = syn::parse_file(&source)?;
    let io = collect_function_io(&file);
    let package_name = nearest_package_name(&root, &generator_path);
    let aliases = package_name
        .as_deref()
        .map(|package| cargo_aliases_for_package(&root, package))
        .unwrap_or_default();
    let generated_attrs = generated_attribute_paths(&root);

    println!("RUST GENERATOR OWNERSHIP PROBE");
    println!("  repository: {}", root.display());
    println!("  generator source: {}", generator.display());
    match &package_name {
        Some(package) => println!("  generator package: {package}"),
        None => println!("  generator package: unknown"),
    }

    if !aliases.is_empty() {
        println!("\nCARGO ALIASES");
        for alias in &aliases {
            println!("  cargo {} -> {}", alias.name, alias.command);
        }
    }

    let relations: Vec<_> = io
        .iter()
        .filter(|function| !function.reads.is_empty() && !function.writes.is_empty())
        .collect();

    if relations.is_empty() {
        println!("\nOBSERVATION");
        println!("  No function contained both recognized repository-path reads and writes.");
        return Ok(());
    }

    println!("\nEXPLICIT PATH RELATIONS");
    for function in relations {
        println!("  function {}", function.function);
        for read in &function.reads {
            println!("    reads  {read}");
        }
        for write in &function.writes {
            let generated = generated_attrs.contains(write);
            println!(
                "    writes {write}{}",
                if generated {
                    "  [.gitattributes: linguist-generated=true]"
                } else {
                    ""
                }
            );
        }
    }

    println!("\nEVIDENCE BOUNDARY");
    println!("  A read/write pair is derived from literal repository paths in one Rust function.");
    println!(
        "  Cargo aliases are reported independently when their command names the generator package."
    );
    println!(
        "  .gitattributes generated markers are reported independently for exact output paths."
    );
    println!("  This probe does not infer that every input edit requires every output to change.");
    println!("  Historical cohorts and current-diff relevance remain separate evidence.");

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

fn normalize_repo_path(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
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

fn generated_attribute_paths(root: &Path) -> BTreeSet<String> {
    let path = root.join(".gitattributes");
    let Ok(source) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    generated_attribute_paths_from_source(&source)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_literal_join_paths_from_read_and_write_calls() {
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
        let functions = collect_function_io(&file);
        assert_eq!(functions.len(), 1);
        assert!(functions[0].reads.contains("src/rules.rs"));
        assert!(functions[0].writes.contains("src/generated/rules.rs"));
    }

    #[test]
    fn ignores_non_fs_calls_with_similar_names() {
        let file = syn::parse_file(
            r#"
            fn generate() {
                let target = root.join("src/generated/rules.rs");
                custom::write(&target, "x");
                parser::read(root.join("src/rules.rs"));
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

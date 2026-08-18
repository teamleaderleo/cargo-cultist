mod diff;
mod test_modules;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process;

use diff::{git_repo_root, print_diff_report};
use test_modules::{analyze_test_modules, print_test_module_report};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    if let Err(error) = run() {
        eprintln!("cargo-cultist: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();

    // Cargo invokes third-party subcommands as `cargo-<name> <name> ...`.
    // Accept direct invocation (`cargo-cultist`) too.
    if args.first().is_some_and(|arg| arg == "cultist") {
        args.remove(0);
    }

    if args.first().is_some_and(|arg| arg == "diff") {
        args.remove(0);
        return run_diff(args);
    }

    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        println!("cargo-cultist {VERSION}");
        return Ok(());
    }

    if args.len() > 1 {
        return Err("expected at most one path argument; try `cargo cultist --help`".into());
    }

    let root = args.pop().map(PathBuf::from).unwrap_or(env::current_dir()?);
    let root = root.canonicalize()?;

    println!("cargo-cultist {VERSION}");
    println!("repository: {}\n", root.display());

    let report = analyze_test_modules(&root)?;
    print_test_module_report(&root, &report);

    Ok(())
}

fn run_diff(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_diff_help();
        return Ok(());
    }

    let (base, path) = parse_diff_args(args)?;
    let requested_root = path.unwrap_or(env::current_dir()?);
    let requested_root = requested_root.canonicalize()?;
    let root = git_repo_root(&requested_root)?;

    println!("cargo-cultist {VERSION}");
    println!("repository: {}\n", root.display());

    let report = analyze_test_modules(&root)?;
    print_diff_report(&root, base.as_deref(), &report)?;

    Ok(())
}

fn parse_diff_args(args: Vec<String>) -> Result<(Option<String>, Option<PathBuf>), Box<dyn Error>> {
    let mut base = None;
    let mut path = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base" => {
                if base.is_some() {
                    return Err("`--base` may only be specified once".into());
                }
                base = Some(
                    args.next()
                        .ok_or("`--base` requires a Git revision")?,
                );
            }
            _ if arg.starts_with('-') => {
                return Err(format!("unknown diff option `{arg}`; try `cargo cultist diff --help`").into());
            }
            _ => {
                if path.is_some() {
                    return Err(
                        "expected at most one path argument; try `cargo cultist diff --help`"
                            .into(),
                    );
                }
                path = Some(PathBuf::from(arg));
            }
        }
    }

    Ok((base, path))
}

fn print_help() {
    println!(
        "cargo-cultist {VERSION}\n\
Repository-aware analysis for Rust codebases.\n\n\
USAGE:\n    cargo cultist [PATH]\n    cargo cultist diff [--base REV] [PATH]\n    cargo-cultist [PATH]\n    cargo-cultist diff [--base REV] [PATH]\n\n\
COMMANDS:\n    diff    Inspect changed Rust code against repository precedent.\n\n\
Without a command, cargo-cultist inspects repository-wide test-module naming\n\
conventions without inventing a universal rule."
    );
}

fn print_diff_help() {
    println!(
        "cargo-cultist diff\n\n\
USAGE:\n    cargo cultist diff [--base REV] [PATH]\n\n\
By default, compares the working tree (including staged changes) against HEAD.\n\
With --base REV, compares the current working tree against that Git revision.\n\n\
The first diff-aware check looks for added or renamed #[cfg(test)] modules and\n\
compares their names with repository-wide and same-file precedent."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_diff_base_and_path() {
        let (base, path) = parse_diff_args(vec![
            "--base".to_string(),
            "origin/main".to_string(),
            ".".to_string(),
        ])
        .unwrap();

        assert_eq!(base.as_deref(), Some("origin/main"));
        assert_eq!(path, Some(PathBuf::from(".")));
    }

    #[test]
    fn rejects_multiple_diff_paths() {
        assert!(parse_diff_args(vec!["a".to_string(), "b".to_string()]).is_err());
    }
}

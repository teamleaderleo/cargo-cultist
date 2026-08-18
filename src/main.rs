mod diff;
mod finding;
mod report;
mod test_modules;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process;

use diff::{build_diff_analysis_report, git_repo_root, print_diff_report};
use report::build_test_module_analysis;
use test_modules::{analyze_test_modules, print_test_module_report};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

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

    let (format, path) = parse_root_args(args)?;
    let root = path.unwrap_or(env::current_dir()?).canonicalize()?;
    let report = analyze_test_modules(&root)?;

    match format {
        OutputFormat::Text => {
            println!("cargo-cultist {VERSION}");
            println!("repository: {}\n", root.display());
            print_test_module_report(&root, &report);
        }
        OutputFormat::Json => {
            let analysis = build_test_module_analysis(&root, &report);
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        }
    }

    Ok(())
}

fn run_diff(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_diff_help();
        return Ok(());
    }

    let (base, path, format) = parse_diff_args(args)?;
    let requested_root = path.unwrap_or(env::current_dir()?);
    let requested_root = requested_root.canonicalize()?;
    let root = git_repo_root(&requested_root)?;
    let report = analyze_test_modules(&root)?;

    match format {
        OutputFormat::Text => {
            println!("cargo-cultist {VERSION}");
            println!("repository: {}\n", root.display());
            print_diff_report(&root, base.as_deref(), &report)?;
        }
        OutputFormat::Json => {
            let analysis = build_diff_analysis_report(&root, base.as_deref(), &report)?;
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        }
    }

    Ok(())
}

fn parse_root_args(args: Vec<String>) -> Result<(OutputFormat, Option<PathBuf>), Box<dyn Error>> {
    let mut format = OutputFormat::Text;
    let mut path = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--format" => {
                format = parse_output_format(
                    &args.next().ok_or("`--format` requires `text` or `json`")?,
                )?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("unknown option `{arg}`; try `cargo cultist --help`").into());
            }
            _ => {
                if path.is_some() {
                    return Err(
                        "expected at most one path argument; try `cargo cultist --help`".into(),
                    );
                }
                path = Some(PathBuf::from(arg));
            }
        }
    }

    Ok((format, path))
}

fn parse_diff_args(
    args: Vec<String>,
) -> Result<(Option<String>, Option<PathBuf>, OutputFormat), Box<dyn Error>> {
    let mut base = None;
    let mut path = None;
    let mut format = OutputFormat::Text;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base" => {
                if base.is_some() {
                    return Err("`--base` may only be specified once".into());
                }
                base = Some(args.next().ok_or("`--base` requires a Git revision")?);
            }
            "--format" => {
                format = parse_output_format(
                    &args.next().ok_or("`--format` requires `text` or `json`")?,
                )?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "unknown diff option `{arg}`; try `cargo cultist diff --help`"
                )
                .into());
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

    Ok((base, path, format))
}

fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!(
            "unsupported output format `{value}`; expected `text` or `json`"
        )),
    }
}

fn print_help() {
    println!(
        "cargo-cultist {VERSION}\n\
Repository-aware analysis for Rust codebases.\n\n\
USAGE:\n    cargo cultist [--format text|json] [PATH]\n    cargo cultist diff [--base REV] [--format text|json] [PATH]\n    cargo-cultist [--format text|json] [PATH]\n    cargo-cultist diff [--base REV] [--format text|json] [PATH]\n\n\
COMMANDS:\n    diff    Inspect changed Rust code against repository precedent.\n\n\
Without a command, cargo-cultist inspects repository-wide test-module naming\n\
conventions without inventing a universal rule."
    );
}

fn print_diff_help() {
    println!(
        "cargo-cultist diff\n\n\
USAGE:\n    cargo cultist diff [--base REV] [--format text|json] [PATH]\n\n\
By default, compares the working tree (including staged changes) against HEAD.\n\
With --base REV, compares changes from the merge base of REV and HEAD.\n\n\
The first diff-aware check looks for added or renamed #[cfg(test)] modules and\n\
compares their names with repository-wide and same-file precedent."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_diff_base_path_and_format() {
        let (base, path, format) = parse_diff_args(vec![
            "--base".to_string(),
            "origin/main".to_string(),
            "--format".to_string(),
            "json".to_string(),
            ".".to_string(),
        ])
        .unwrap();

        assert_eq!(base.as_deref(), Some("origin/main"));
        assert_eq!(path, Some(PathBuf::from(".")));
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn parses_root_json_format() {
        let (format, path) =
            parse_root_args(vec!["--format".to_string(), "json".to_string()]).unwrap();
        assert_eq!(format, OutputFormat::Json);
        assert_eq!(path, None);
    }

    #[test]
    fn rejects_multiple_diff_paths() {
        assert!(parse_diff_args(vec!["a".to_string(), "b".to_string()]).is_err());
    }
}

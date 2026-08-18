mod diff;
mod finding;
mod history;
mod report;
mod test_modules;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process;

use diff::{build_diff_analysis_report, git_repo_root, print_diff_report};
use history::{
    DEFAULT_MAX_COMMITS, HistoryOptions, analyze_historical_companions, print_history_report,
};
use report::build_test_module_analysis;
use test_modules::{analyze_test_modules, print_test_module_report};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct DiffArgs {
    base: Option<String>,
    path: Option<PathBuf>,
    format: OutputFormat,
}

#[derive(Debug, Eq, PartialEq)]
struct HistoryArgs {
    path: PathBuf,
    max_commits: usize,
    format: OutputFormat,
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

    if args.first().is_some_and(|arg| arg == "history") {
        args.remove(0);
        return run_history(args);
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

    let DiffArgs { base, path, format } = parse_diff_args(args)?;
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

fn run_history(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_history_help();
        return Ok(());
    }

    let HistoryArgs {
        path,
        max_commits,
        format,
    } = parse_history_args(args)?;

    let requested = if path.is_absolute() {
        path
    } else {
        env::current_dir()?.join(path)
    };
    let requested = requested.canonicalize()?;
    if !requested.is_file() {
        return Err(format!(
            "history currently expects a file path; got {}",
            requested.display()
        )
        .into());
    }

    let probe = requested
        .parent()
        .ok_or("could not determine the history path's parent directory")?;
    let root = git_repo_root(probe)?;
    let anchor = requested
        .strip_prefix(&root)
        .map_err(|_| "history path is outside the resolved Git repository")?;

    let report = analyze_historical_companions(
        &root,
        anchor,
        HistoryOptions {
            max_commits,
            ..HistoryOptions::default()
        },
    )?;

    match format {
        OutputFormat::Text => {
            println!("cargo-cultist {VERSION}");
            println!("repository: {}\n", root.display());
            print_history_report(&report);
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
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

fn parse_diff_args(args: Vec<String>) -> Result<DiffArgs, Box<dyn Error>> {
    let mut parsed = DiffArgs::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base" => {
                if parsed.base.is_some() {
                    return Err("`--base` may only be specified once".into());
                }
                parsed.base = Some(args.next().ok_or("`--base` requires a Git revision")?);
            }
            "--format" => {
                parsed.format = parse_output_format(
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
                if parsed.path.is_some() {
                    return Err(
                        "expected at most one path argument; try `cargo cultist diff --help`"
                            .into(),
                    );
                }
                parsed.path = Some(PathBuf::from(arg));
            }
        }
    }

    Ok(parsed)
}

fn parse_history_args(args: Vec<String>) -> Result<HistoryArgs, Box<dyn Error>> {
    let mut path = None;
    let mut max_commits = DEFAULT_MAX_COMMITS;
    let mut format = OutputFormat::Text;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-commits" => {
                let value = args
                    .next()
                    .ok_or("`--max-commits` requires a positive integer")?;
                max_commits = value
                    .parse::<usize>()
                    .map_err(|_| "`--max-commits` requires a positive integer")?;
                if max_commits == 0 {
                    return Err("`--max-commits` requires a positive integer".into());
                }
            }
            "--format" => {
                format = parse_output_format(
                    &args.next().ok_or("`--format` requires `text` or `json`")?,
                )?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "unknown history option `{arg}`; try `cargo cultist history --help`"
                )
                .into());
            }
            _ => {
                if path.is_some() {
                    return Err(
                        "history expects exactly one file path; try `cargo cultist history --help`"
                            .into(),
                    );
                }
                path = Some(PathBuf::from(arg));
            }
        }
    }

    Ok(HistoryArgs {
        path: path.ok_or("history requires a file path")?,
        max_commits,
        format,
    })
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
USAGE:\n    cargo cultist [--format text|json] [PATH]\n    cargo cultist diff [--base REV] [--format text|json] [PATH]\n    cargo cultist history [--max-commits N] [--format text|json] FILE\n    cargo-cultist [--format text|json] [PATH]\n    cargo-cultist diff [--base REV] [--format text|json] [PATH]\n    cargo-cultist history [--max-commits N] [--format text|json] FILE\n\n\
COMMANDS:\n    diff       Inspect changed Rust code against repository precedent.\n    history    Explore which paths historically change with one file.\n\n\
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

fn print_history_help() {
    println!(
        "cargo-cultist history\n\n\
USAGE:\n    cargo cultist history [--max-commits N] [--format text|json] FILE\n\n\
Explores the most recent non-merge commits touching FILE and reports which\n\
other paths changed in the same considered commits. Revert commits and broad\n\
commits changing more than 100 paths are excluded from the first-pass cohort.\n\n\
This is research instrumentation for temporal and negative-space precedent.\n\
It reports associations, examples, and counterexamples without turning\n\
co-change frequency into a correctness claim."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_diff_base_path_and_format() {
        let parsed = parse_diff_args(vec![
            "--base".to_string(),
            "origin/main".to_string(),
            "--format".to_string(),
            "json".to_string(),
            ".".to_string(),
        ])
        .unwrap();

        assert_eq!(parsed.base.as_deref(), Some("origin/main"));
        assert_eq!(parsed.path, Some(PathBuf::from(".")));
        assert_eq!(parsed.format, OutputFormat::Json);
    }

    #[test]
    fn parses_root_json_format() {
        let (format, path) =
            parse_root_args(vec!["--format".to_string(), "json".to_string()]).unwrap();
        assert_eq!(format, OutputFormat::Json);
        assert_eq!(path, None);
    }

    #[test]
    fn parses_history_path_limit_and_format() {
        let parsed = parse_history_args(vec![
            "--max-commits".to_string(),
            "42".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "src/lib.rs".to_string(),
        ])
        .unwrap();

        assert_eq!(parsed.path, PathBuf::from("src/lib.rs"));
        assert_eq!(parsed.max_commits, 42);
        assert_eq!(parsed.format, OutputFormat::Json);
    }

    #[test]
    fn rejects_missing_history_path() {
        assert!(parse_history_args(vec![]).is_err());
    }

    #[test]
    fn rejects_zero_history_limit() {
        assert!(
            parse_history_args(vec![
                "--max-commits".to_string(),
                "0".to_string(),
                "src/lib.rs".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_multiple_diff_paths() {
        assert!(parse_diff_args(vec!["a".to_string(), "b".to_string()]).is_err());
    }
}

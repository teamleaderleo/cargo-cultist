mod cochange;
mod diff;
mod finding;
mod report;
mod test_modules;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process;

use cochange::{
    analyze_cochange, build_cochange_analysis_report, print_cochange_report,
    DEFAULT_HISTORY_LIMIT, DEFAULT_MAX_FILES_PER_COMMIT,
};
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

#[derive(Debug, Default, Eq, PartialEq)]
struct DiffArgs {
    base: Option<String>,
    path: Option<PathBuf>,
    format: OutputFormat,
}

#[derive(Debug, Eq, PartialEq)]
struct CochangeArgs {
    target: Option<PathBuf>,
    repo: Option<PathBuf>,
    format: OutputFormat,
    commits: usize,
    max_files: usize,
}

impl Default for CochangeArgs {
    fn default() -> Self {
        Self {
            target: None,
            repo: None,
            format: OutputFormat::Text,
            commits: DEFAULT_HISTORY_LIMIT,
            max_files: DEFAULT_MAX_FILES_PER_COMMIT,
        }
    }
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

    if args.first().is_some_and(|arg| arg == "cochange") {
        args.remove(0);
        return run_cochange(args);
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

fn run_cochange(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_cochange_help();
        return Ok(());
    }

    let CochangeArgs {
        target,
        repo,
        format,
        commits,
        max_files,
    } = parse_cochange_args(args)?;
    let target = target.ok_or("`cochange` requires one repository-relative target path")?;
    let requested_root = repo.unwrap_or(env::current_dir()?);
    let requested_root = requested_root.canonicalize()?;
    let root = git_repo_root(&requested_root)?;
    let report = analyze_cochange(&root, &target, commits, max_files)?;

    match format {
        OutputFormat::Text => {
            println!("cargo-cultist {VERSION}");
            println!("repository: {}\n", root.display());
            print_cochange_report(&report);
        }
        OutputFormat::Json => {
            let analysis = build_cochange_analysis_report(&root, &report);
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

fn parse_cochange_args(args: Vec<String>) -> Result<CochangeArgs, Box<dyn Error>> {
    let mut parsed = CochangeArgs::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo" => {
                if parsed.repo.is_some() {
                    return Err("`--repo` may only be specified once".into());
                }
                parsed.repo = Some(PathBuf::from(
                    args.next().ok_or("`--repo` requires a repository path")?,
                ));
            }
            "--format" => {
                parsed.format = parse_output_format(
                    &args.next().ok_or("`--format` requires `text` or `json`")?,
                )?;
            }
            "--commits" => {
                let value = args.next().ok_or("`--commits` requires a positive integer")?;
                parsed.commits = parse_positive_usize("--commits", &value)?;
            }
            "--max-files" => {
                let value = args
                    .next()
                    .ok_or("`--max-files` requires a positive integer")?;
                parsed.max_files = parse_positive_usize("--max-files", &value)?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "unknown cochange option `{arg}`; try `cargo cultist cochange --help`"
                )
                .into());
            }
            _ => {
                if parsed.target.is_some() {
                    return Err(
                        "`cochange` expects one target path; try `cargo cultist cochange --help`"
                            .into(),
                    );
                }
                parsed.target = Some(PathBuf::from(arg));
            }
        }
    }

    Ok(parsed)
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

fn parse_positive_usize(option: &str, value: &str) -> Result<usize, Box<dyn Error>> {
    let parsed: usize = value
        .parse()
        .map_err(|_| format!("`{option}` requires a positive integer"))?;
    if parsed == 0 {
        return Err(format!("`{option}` requires a positive integer").into());
    }
    Ok(parsed)
}

fn print_help() {
    println!(
        "cargo-cultist {VERSION}\n\
Repository-aware analysis for Rust codebases.\n\n\
USAGE:\n    cargo cultist [--format text|json] [PATH]\n    cargo cultist diff [--base REV] [--format text|json] [PATH]\n    cargo cultist cochange [OPTIONS] TARGET\n    cargo-cultist [--format text|json] [PATH]\n    cargo-cultist diff [--base REV] [--format text|json] [PATH]\n    cargo-cultist cochange [OPTIONS] TARGET\n\n\
COMMANDS:\n    diff        Inspect changed Rust code against repository precedent.\n    cochange    Explore historical path co-change precedent.\n\n\
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

fn print_cochange_help() {
    println!(
        "cargo-cultist cochange\n\n\
USAGE:\n    cargo cultist cochange [--repo PATH] [--commits N] [--max-files N] [--format text|json] TARGET\n\n\
TARGET is a repository-relative path that currently exists.\n\
The command reads local Git history only, ignores merge commits, and skips\n\
commits touching more than --max-files paths before ranking companion paths.\n\n\
Defaults: --commits {}, --max-files {}.",
        DEFAULT_HISTORY_LIMIT, DEFAULT_MAX_FILES_PER_COMMIT
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
    fn parses_cochange_target_and_limits() {
        let parsed = parse_cochange_args(vec![
            "--repo".to_string(),
            "/repo".to_string(),
            "--commits".to_string(),
            "25".to_string(),
            "--max-files".to_string(),
            "12".to_string(),
            "--format".to_string(),
            "json".to_string(),
            "src/main.rs".to_string(),
        ])
        .unwrap();

        assert_eq!(parsed.repo, Some(PathBuf::from("/repo")));
        assert_eq!(parsed.target, Some(PathBuf::from("src/main.rs")));
        assert_eq!(parsed.commits, 25);
        assert_eq!(parsed.max_files, 12);
        assert_eq!(parsed.format, OutputFormat::Json);
    }

    #[test]
    fn rejects_zero_cochange_limits() {
        assert!(parse_cochange_args(vec![
            "--commits".to_string(),
            "0".to_string(),
            "src/main.rs".to_string(),
        ])
        .is_err());
    }

    #[test]
    fn rejects_multiple_diff_paths() {
        assert!(parse_diff_args(vec!["a".to_string(), "b".to_string()]).is_err());
    }
}

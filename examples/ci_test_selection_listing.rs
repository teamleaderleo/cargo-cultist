#[path = "../src/finding.rs"]
mod finding;
#[path = "../src/ci_test_filters.rs"]
mod ci_test_filters;

use std::env;
use std::error::Error;
use std::path::Path;
use std::process::Command;

use ci_test_filters::analyze_ci_test_filters;

fn main() {
    if let Err(error) = run() {
        eprintln!("ci-test-selection-listing: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let root = args
        .next()
        .map(std::path::PathBuf::from)
        .unwrap_or(env::current_dir()?)
        .canonicalize()?;
    if args.next().is_some() {
        return Err("usage: ci_test_selection_listing [REPO]".into());
    }

    let report = analyze_ci_test_filters(&root)?;
    println!("CI TEST SELECTION LISTING PROBE");
    println!("  repository: {}", root.display());
    println!("  supported commands: {}", report.commands.len());
    println!("  WARNING: this probe invokes Cargo and may execute repository build scripts while compiling the library test target.");

    for command in &report.commands {
        println!(
            "\n{}:{}  filter `{}`",
            relative_path(&root, &command.workflow_path),
            command.line,
            command.filter
        );
        println!("  workflow command: {}", command.command);
        match list_selected_tests(&root, &command.command) {
            Ok(selected) => {
                println!("  listed selections: {}", selected.len());
                for name in selected.iter().take(10) {
                    println!("    {name}");
                }
            }
            Err(error) => println!("  listing failed: {error}"),
        }
    }

    Ok(())
}

fn list_selected_tests(root: &Path, command: &str) -> Result<Vec<String>, String> {
    let args = listing_args(command).ok_or_else(|| "unsupported command shape".to_string())?;
    let output = Command::new("cargo")
        .args(&args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to invoke Cargo: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "Cargo listing exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("Cargo listing stdout was not UTF-8: {error}"))?;
    Ok(parse_libtest_listing(&stdout))
}

fn listing_args(command: &str) -> Option<Vec<String>> {
    let tokens: Vec<_> = command.split_whitespace().collect();
    if tokens.len() < 4 || tokens[0] != "cargo" {
        return None;
    }

    let mut command_index = 1;
    if tokens.get(command_index).is_some_and(|token| token.starts_with('+')) {
        command_index += 1;
    }
    if tokens.get(command_index).copied() != Some("test") {
        return None;
    }

    let harness_separator = tokens.iter().position(|token| *token == "--");
    let end = harness_separator.unwrap_or(tokens.len());
    let mut args = tokens[1..end]
        .iter()
        .map(|token| token.trim_matches(['\'', '"']).to_string())
        .collect::<Vec<_>>();
    args.push("--".to_string());
    args.push("--list".to_string());
    Some(args)
}

fn parse_libtest_listing(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| {
            line.strip_suffix(": test")
                .or_else(|| line.strip_suffix(": benchmark"))
                .map(str::to_string)
        })
        .collect()
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_listing_command_without_running_harness_arguments() {
        assert_eq!(
            listing_args("cargo test --features io_uring --lib test_rollback -- --nocapture"),
            Some(vec![
                "test".to_string(),
                "--features".to_string(),
                "io_uring".to_string(),
                "--lib".to_string(),
                "test_rollback".to_string(),
                "--".to_string(),
                "--list".to_string(),
            ])
        );
    }

    #[test]
    fn preserves_cargo_toolchain_selector() {
        assert_eq!(
            listing_args(
                "cargo +1.88.0 test --lib test_rollback --locked --no-default-features"
            ),
            Some(vec![
                "+1.88.0".to_string(),
                "test".to_string(),
                "--lib".to_string(),
                "test_rollback".to_string(),
                "--locked".to_string(),
                "--no-default-features".to_string(),
                "--".to_string(),
                "--list".to_string(),
            ])
        );
    }

    #[test]
    fn parses_libtest_selection_names() {
        let listing = "tests::first: test\ntests::second: test\nbenchmarks::speed: benchmark\n\n3 tests, 0 benchmarks\n";
        assert_eq!(
            parse_libtest_listing(listing),
            vec![
                "tests::first".to_string(),
                "tests::second".to_string(),
                "benchmarks::speed".to_string(),
            ]
        );
    }

    #[test]
    fn zero_listing_is_authoritative_empty_selection() {
        assert!(parse_libtest_listing("0 tests, 0 benchmarks\n").is_empty());
    }
}

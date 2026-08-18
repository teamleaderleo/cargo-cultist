mod test_modules;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process;

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

    let root = args
        .pop()
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let root = root.canonicalize()?;

    println!("cargo-cultist {VERSION}");
    println!("repository: {}\n", root.display());

    let report = analyze_test_modules(&root)?;
    print_test_module_report(&root, &report);

    Ok(())
}

fn print_help() {
    println!(
        "cargo-cultist {VERSION}\n\
Repository-aware analysis for Rust codebases.\n\n\
USAGE:\n    cargo cultist [PATH]\n    cargo-cultist [PATH]\n\n\
The first prototype inspects test-gated module names and reports\n\
what the repository actually does, without inventing a universal rule."
    );
}

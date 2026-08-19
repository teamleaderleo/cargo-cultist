use std::env;
use std::error::Error;
use std::path::PathBuf;

#[path = "../src/generator_ownership.rs"]
mod generator_ownership;

use generator_ownership::analyze_generator_source;

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

    let report = analyze_generator_source(&root, &generator)?;

    println!("RUST GENERATOR OWNERSHIP PROBE");
    println!("  repository: {}", root.display());
    println!("  generator source: {}", report.generator_path);
    match &report.package {
        Some(package) => println!("  generator package: {package}"),
        None => println!("  generator package: unknown"),
    }

    if !report.aliases.is_empty() {
        println!("\nCARGO ALIASES");
        for alias in &report.aliases {
            println!("  cargo {} -> {}", alias.name, alias.command);
        }
    }

    let relations: Vec<_> = report
        .functions
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
            let generated = report.generated_attributes.contains(write);
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
    println!(
        "  A read/write pair is derived from literal paths joined to a binding whose repository-root provider is explicitly recognized."
    );
    println!(
        "  The first provider vocabulary is deliberately narrow; unresolved path roots are omitted instead of inferred from variable names."
    );
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

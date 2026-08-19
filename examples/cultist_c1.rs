#[allow(dead_code)]
#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;

use std::env;
use std::error::Error;
use std::io::{self, Read, Write};

use compact_ir::{decode_report, encode_report};
use finding::AnalysisReport;

const USAGE: &str = "usage: cargo run --example cultist_c1 -- [--decode] < input";

fn main() {
    if let Err(error) = run() {
        eprintln!("cultist-c1: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let decode = match args.as_slice() {
        [] => false,
        [flag] if flag == "--decode" => true,
        _ => return Err(USAGE.into()),
    };

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let stdout = io::stdout();
    let mut output = stdout.lock();
    if decode {
        let report = decode_report(&input)?;
        serde_json::to_writer_pretty(&mut output, &report)?;
        writeln!(output)?;
    } else {
        let report: AnalysisReport = serde_json::from_str(&input)?;
        output.write_all(encode_report(&report)?.as_bytes())?;
    }

    Ok(())
}

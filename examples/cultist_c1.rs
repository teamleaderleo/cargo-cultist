#[allow(dead_code)]
#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;

use std::env;
use std::error::Error;
use std::io::{self, Read, Write};

use compact_ir::{decode_report, encode_report, MAX_C1_BYTES};
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

    let input = read_bounded_stdin()?;

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

fn read_bounded_stdin() -> Result<String, Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_C1_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_C1_BYTES {
        return Err(format!(
            "input exceeds research converter limit of {MAX_C1_BYTES} bytes"
        )
        .into());
    }
    Ok(String::from_utf8(bytes)?)
}

#![allow(dead_code)]

#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[path = "../src/finding.rs"]
mod finding;
#[path = "../src/report_fingerprint.rs"]
mod report_fingerprint;

use std::error::Error;
use std::io::{self, Read};

use compact_ir::MAX_C1_BYTES;
use finding::AnalysisReport;
use report_fingerprint::fingerprint_report;

const MAX_JSON_INPUT_BYTES: usize = MAX_C1_BYTES * 2;

fn main() {
    if let Err(error) = run() {
        eprintln!("report-fingerprint: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_JSON_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_JSON_INPUT_BYTES {
        return Err(format!(
            "report JSON exceeds the {MAX_JSON_INPUT_BYTES}-byte research input limit"
        )
        .into());
    }

    let input = String::from_utf8(bytes)?;
    let report: AnalysisReport = serde_json::from_str(&input)?;
    println!("{}", fingerprint_report(&report)?);
    Ok(())
}

#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;

use std::error::Error;
use std::io::{self, Read};

use applicability::{
    ApplicabilityQuery, MAX_APPLICABILITY_QUERY_BYTES, evaluate_query,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("evidence-applicability: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_APPLICABILITY_QUERY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_APPLICABILITY_QUERY_BYTES {
        return Err(format!(
            "applicability query exceeds the {MAX_APPLICABILITY_QUERY_BYTES}-byte limit"
        )
        .into());
    }

    let input = String::from_utf8(bytes)?;
    let query: ApplicabilityQuery = serde_json::from_str(&input)?;
    let evaluation = evaluate_query(&query)?;
    serde_json::to_writer_pretty(io::stdout().lock(), &evaluation)?;
    println!();
    Ok(())
}

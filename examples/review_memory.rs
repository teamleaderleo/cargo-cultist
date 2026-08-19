use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[allow(dead_code)]
#[path = "../src/review_memory.rs"]
mod review_memory;

use review_memory::{
    MAX_REVIEW_MEMORY_QUERY_BYTES, evaluate_review_memory, parse_review_memory_query,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("review-memory: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_REVIEW_MEMORY_QUERY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_REVIEW_MEMORY_QUERY_BYTES {
        return Err(format!(
            "review-memory query exceeds the {MAX_REVIEW_MEMORY_QUERY_BYTES}-byte limit"
        )
        .into());
    }

    let query = parse_review_memory_query(&bytes)?;
    let evaluation = evaluate_review_memory(&query)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

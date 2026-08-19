#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

#[path = "../src/cohort_refinement.rs"]
mod cohort_refinement;

use cohort_refinement::{RefinementRequest, evaluate_refinements};

const MAX_INPUT_BYTES: u64 = 1024 * 1024;

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Vec::new();
    io::stdin()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut input)?;
    if input.len() as u64 > MAX_INPUT_BYTES {
        return Err("cohort refinement input exceeds 1 MiB".into());
    }

    let request: RefinementRequest = serde_json::from_slice(&input)?;
    let evaluation = evaluate_refinements(&request)?;
    serde_json::to_writer_pretty(io::stdout().lock(), &evaluation)?;
    println!();
    Ok(())
}

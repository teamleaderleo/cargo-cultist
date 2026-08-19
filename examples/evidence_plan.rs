#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/durable_obligation.rs"]
mod durable_obligation;
#[path = "../src/evidence_planner.rs"]
mod evidence_planner;
#[path = "../src/justification.rs"]
mod justification;

use evidence_planner::{ProbePlanRequest, plan_evidence};

const MAX_INPUT_BYTES: u64 = 1024 * 1024;

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Vec::new();
    io::stdin()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut input)?;
    if input.len() as u64 > MAX_INPUT_BYTES {
        return Err("evidence planner input exceeds 1 MiB".into());
    }

    let request: ProbePlanRequest = serde_json::from_slice(&input)?;
    let plan = plan_evidence(&request)?;
    serde_json::to_writer_pretty(io::stdout().lock(), &plan)?;
    println!();
    Ok(())
}

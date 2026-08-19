#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

use serde::Deserialize;

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/durable_obligation.rs"]
mod durable_obligation;
#[path = "../src/justification.rs"]
mod justification;

use applicability::EvaluationContext;
use durable_obligation::{ClearingEvidenceReceipt, DurableObligation, evaluate_obligation};

const MAX_INPUT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationInput {
    obligation: DurableObligation,
    #[serde(default)]
    receipts: Vec<ClearingEvidenceReceipt>,
    context: EvaluationContext,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = Vec::new();
    io::stdin()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut input)?;
    if input.len() as u64 > MAX_INPUT_BYTES {
        return Err("durable unknown input exceeds 1 MiB".into());
    }

    let request: EvaluationInput = serde_json::from_slice(&input)?;
    let evaluation = evaluate_obligation(&request.obligation, &request.receipts, &request.context)?;
    serde_json::to_writer_pretty(io::stdout().lock(), &evaluation)?;
    println!();
    Ok(())
}

#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

use serde::Deserialize;

#[path = "../src/applicability.rs"]
mod applicability;
#[path = "../src/justification.rs"]
mod justification;

use applicability::EvaluationContext;
use justification::{JustificationGraph, evaluate_graph, reevaluate_graph};

const MAX_INPUT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationInput {
    graph: JustificationGraph,
    context: EvaluationContext,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReevaluationInput {
    graph: JustificationGraph,
    before_context: EvaluationContext,
    after_context: EvaluationContext,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut input = Vec::new();
    io::stdin()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut input)?;
    if input.len() as u64 > MAX_INPUT_BYTES {
        return Err("justification input exceeds 1 MiB".into());
    }

    match args.as_slice() {
        [] => {
            let request: EvaluationInput = serde_json::from_slice(&input)?;
            let evaluation = evaluate_graph(&request.graph, &request.context)?;
            serde_json::to_writer_pretty(io::stdout().lock(), &evaluation)?;
        }
        [flag] if flag == "--reevaluate" => {
            let request: ReevaluationInput = serde_json::from_slice(&input)?;
            let receipt = reevaluate_graph(
                &request.graph,
                &request.before_context,
                &request.after_context,
            )?;
            serde_json::to_writer_pretty(io::stdout().lock(), &receipt)?;
        }
        _ => {
            return Err("usage: cultist_justification [--reevaluate] < request.json".into());
        }
    }

    println!();
    Ok(())
}

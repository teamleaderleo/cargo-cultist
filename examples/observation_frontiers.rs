#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;

use observation_frontier::{
    MAX_OBSERVATION_FRONTIER_REQUEST_BYTES, evaluate_observation_frontiers,
    parse_observation_frontier_request,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("observation-frontiers: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_OBSERVATION_FRONTIER_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_OBSERVATION_FRONTIER_REQUEST_BYTES {
        return Err(format!(
            "observation frontier request exceeds the {MAX_OBSERVATION_FRONTIER_REQUEST_BYTES}-byte limit"
        )
        .into());
    }

    let request = parse_observation_frontier_request(&bytes)?;
    let evaluation = evaluate_observation_frontiers(&request)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

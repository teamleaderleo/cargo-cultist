use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[allow(dead_code)]
#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;
#[allow(dead_code)]
#[path = "../src/justification.rs"]
mod justification;
#[allow(dead_code)]
#[path = "../src/durable_obligation.rs"]
mod durable_obligation;
#[allow(dead_code)]
#[path = "../src/evidence_planner.rs"]
mod evidence_planner;
#[allow(dead_code)]
#[path = "../src/observation_frontier.rs"]
mod observation_frontier;
#[allow(dead_code)]
#[path = "../src/observation_probe_bridge.rs"]
mod observation_probe_bridge;

use observation_probe_bridge::{
    MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES, parse_observation_probe_plan_request,
    plan_observation_probe,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("observation-probe-plan: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES {
        return Err(format!(
            "observation probe plan request exceeds the {MAX_OBSERVATION_PROBE_PLAN_REQUEST_BYTES}-byte limit"
        )
        .into());
    }

    let request = parse_observation_probe_plan_request(&bytes)?;
    let plan = plan_observation_probe(&request)?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

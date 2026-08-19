use std::error::Error;
use std::io::{self, Read};

use serde::Deserialize;

#[allow(dead_code)]
#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;

use behavioral_trial::{
    BehavioralTrialArmKind, BehavioralTrialPlan, MAX_BEHAVIORAL_TRIAL_BYTES,
    materialize_worker_packet,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterializeRequest {
    arm: BehavioralTrialArmKind,
    plan: Box<BehavioralTrialPlan>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-trial-materialize: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_BEHAVIORAL_TRIAL_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BEHAVIORAL_TRIAL_BYTES {
        return Err(format!(
            "behavioral-trial materialize request exceeds the {MAX_BEHAVIORAL_TRIAL_BYTES}-byte limit"
        )
        .into());
    }

    let request: MaterializeRequest = serde_json::from_slice(&bytes)?;
    let packet = materialize_worker_packet(request.plan.as_ref(), request.arm)?;
    println!("{}", serde_json::to_string_pretty(&packet)?);
    Ok(())
}

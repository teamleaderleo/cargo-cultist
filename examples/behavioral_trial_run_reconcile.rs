use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[allow(dead_code)]
#[path = "../src/behavioral_trial_run.rs"]
mod behavioral_trial_run;

use behavioral_trial_run::{
    MAX_BEHAVIORAL_TRIAL_RUN_BYTES, evaluate_behavioral_trial_run_pair,
    parse_behavioral_trial_run_pair,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-trial-run-reconcile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_BEHAVIORAL_TRIAL_RUN_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BEHAVIORAL_TRIAL_RUN_BYTES {
        return Err(format!(
            "behavioral-trial run pair exceeds the {MAX_BEHAVIORAL_TRIAL_RUN_BYTES}-byte limit"
        )
        .into());
    }

    let pair = parse_behavioral_trial_run_pair(&bytes)?;
    let evaluation = evaluate_behavioral_trial_run_pair(&pair)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

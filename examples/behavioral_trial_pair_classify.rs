use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[allow(dead_code)]
#[path = "../src/behavioral_trial_run.rs"]
mod behavioral_trial_run;
#[allow(dead_code)]
#[path = "../src/behavioral_trial_pair_classification.rs"]
mod behavioral_trial_pair_classification;

use behavioral_trial_pair_classification::classify_behavioral_trial_run_pair;
use behavioral_trial_run::{MAX_BEHAVIORAL_TRIAL_RUN_PAIR_BYTES, parse_behavioral_trial_run_pair};

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-trial-pair-classify: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut input = Vec::new();
    io::stdin()
        .take((MAX_BEHAVIORAL_TRIAL_RUN_PAIR_BYTES + 1) as u64)
        .read_to_end(&mut input)?;
    if input.len() > MAX_BEHAVIORAL_TRIAL_RUN_PAIR_BYTES {
        return Err(format!(
            "behavioral-trial run-pair input exceeds the {}-byte limit",
            MAX_BEHAVIORAL_TRIAL_RUN_PAIR_BYTES
        )
        .into());
    }

    let pair = parse_behavioral_trial_run_pair(&input)?;
    let classification = classify_behavioral_trial_run_pair(&pair)?;
    println!("{}", serde_json::to_string_pretty(&classification)?);
    Ok(())
}

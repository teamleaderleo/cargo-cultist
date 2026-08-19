#![allow(dead_code)]

use std::io::{self, Read};

#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[path = "../src/behavioral_trial_pair_admission.rs"]
mod behavioral_trial_pair_admission;
#[path = "../src/behavioral_trial_run.rs"]
mod behavioral_trial_run;

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-trial-pair-admission: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;
    let pair = behavioral_trial_run::parse_behavioral_trial_run_pair(&input)?;
    let evaluation =
        behavioral_trial_pair_admission::evaluate_behavioral_trial_pair_admission(&pair)
            .map_err(io::Error::other)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

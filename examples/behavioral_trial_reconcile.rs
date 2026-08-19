use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;

use behavioral_trial::{
    MAX_BEHAVIORAL_TRIAL_BYTES, evaluate_behavioral_trial_pair, parse_behavioral_trial_pair,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-trial-reconcile: {error}");
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
            "behavioral-trial pair exceeds the {MAX_BEHAVIORAL_TRIAL_BYTES}-byte limit"
        )
        .into());
    }

    let pair = parse_behavioral_trial_pair(&bytes)?;
    let evaluation = evaluate_behavioral_trial_pair(&pair)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

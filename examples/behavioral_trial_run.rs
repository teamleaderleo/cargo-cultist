use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

#[allow(dead_code)]
#[path = "../src/behavioral_trial.rs"]
mod behavioral_trial;
#[allow(dead_code)]
#[path = "../src/behavioral_trial_run.rs"]
mod behavioral_trial_run;

use behavioral_trial::{MAX_BEHAVIORAL_TRIAL_BYTES, parse_behavioral_trial_plan};
use behavioral_trial_run::{
    MAX_BEHAVIORAL_TRIAL_RUN_RECEIPT_BYTES, evaluate_behavioral_trial_runs,
    parse_behavioral_trial_run_receipt,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-trial-run: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let plan_path = next_path(&mut args)?;
    let first_run_path = next_path(&mut args)?;
    let second_run_path = next_path(&mut args)?;
    if args.next().is_some() {
        return Err(usage().into());
    }

    let plan = parse_behavioral_trial_plan(&read_bounded(&plan_path, MAX_BEHAVIORAL_TRIAL_BYTES)?)
        .map_err(|error| invalid_data(error.to_string()))?;
    let first = parse_behavioral_trial_run_receipt(&read_bounded(
        &first_run_path,
        MAX_BEHAVIORAL_TRIAL_RUN_RECEIPT_BYTES,
    )?)
    .map_err(invalid_data)?;
    let second = parse_behavioral_trial_run_receipt(&read_bounded(
        &second_run_path,
        MAX_BEHAVIORAL_TRIAL_RUN_RECEIPT_BYTES,
    )?)
    .map_err(invalid_data)?;

    let evaluation =
        evaluate_behavioral_trial_runs(&plan, &first, &second).map_err(invalid_data)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

fn next_path(args: &mut impl Iterator<Item = String>) -> Result<String, io::Error> {
    args.next().ok_or_else(usage)
}

fn usage() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: behavioral_trial_run PLAN.json RUN_A.json RUN_B.json",
    )
}

fn read_bounded(path: impl AsRef<Path>, maximum: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)?;
    if metadata.len() > maximum as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} exceeds {maximum} bytes", path.display()),
        )
        .into());
    }
    Ok(fs::read(path)?)
}

fn invalid_data(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

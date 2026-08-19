#[allow(dead_code)]
#[path = "../src/capability_demand_retirement.rs"]
mod capability_demand_retirement;

use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use capability_demand_retirement::{
    MAX_RUN_RECEIPT_BYTES, MAX_TRIAL_MANIFEST_BYTES, MAX_TRIAL_SPEC_BYTES, evaluate_pair,
    parse_run_receipt, parse_trial_manifest, parse_trial_spec,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("capability-demand-retirement: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let trial_path = next_path(&mut args)?;
    let manifest_path = next_path(&mut args)?;
    let first_run_path = next_path(&mut args)?;
    let second_run_path = next_path(&mut args)?;
    if args.next().is_some() {
        return Err(usage().into());
    }

    let trial = parse_trial_spec(&read_bounded(&trial_path, MAX_TRIAL_SPEC_BYTES)?)
        .map_err(invalid_data)?;
    let manifest = parse_trial_manifest(&read_bounded(&manifest_path, MAX_TRIAL_MANIFEST_BYTES)?)
        .map_err(invalid_data)?;
    let first = parse_run_receipt(&read_bounded(&first_run_path, MAX_RUN_RECEIPT_BYTES)?)
        .map_err(invalid_data)?;
    let second = parse_run_receipt(&read_bounded(&second_run_path, MAX_RUN_RECEIPT_BYTES)?)
        .map_err(invalid_data)?;

    let evaluation = evaluate_pair(&trial, &manifest, &first, &second).map_err(invalid_data)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

fn next_path(args: &mut impl Iterator<Item = String>) -> Result<String, io::Error> {
    args.next().ok_or_else(usage)
}

fn usage() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: capability_demand_retirement TRIAL.json INPUT_MANIFEST.json RUN_A.json RUN_B.json",
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

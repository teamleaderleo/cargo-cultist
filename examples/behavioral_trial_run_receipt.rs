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
    MAX_BEHAVIORAL_TRIAL_RUN_BYTES, build_behavioral_trial_run_receipt,
    parse_behavioral_trial_run_metadata,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-trial-run-receipt: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let plan_path = args.next().ok_or_else(usage_error)?;
    let metadata_path = args.next().ok_or_else(usage_error)?;
    let worker_packet_path = args.next().ok_or_else(usage_error)?;
    let worker_output_path = args.next().ok_or_else(usage_error)?;
    if args.next().is_some() {
        return Err(usage_error().into());
    }

    let plan_bytes = read_bounded(&plan_path, MAX_BEHAVIORAL_TRIAL_BYTES)?;
    let metadata_bytes = read_bounded(&metadata_path, MAX_BEHAVIORAL_TRIAL_RUN_BYTES)?;
    let worker_packet_bytes = read_bounded(&worker_packet_path, MAX_BEHAVIORAL_TRIAL_RUN_BYTES)?;
    let worker_output_bytes = read_bounded(&worker_output_path, MAX_BEHAVIORAL_TRIAL_RUN_BYTES)?;

    let plan = parse_behavioral_trial_plan(&plan_bytes)?;
    let metadata = parse_behavioral_trial_run_metadata(&metadata_bytes)?;
    let receipt = build_behavioral_trial_run_receipt(
        &plan,
        metadata,
        &worker_packet_bytes,
        &worker_output_bytes,
    )?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
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

fn usage_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: behavioral_trial_run_receipt PLAN.json METADATA.json WORKER_PACKET.json RAW_OUTPUT.json",
    )
}

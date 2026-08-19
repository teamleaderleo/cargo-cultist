#[allow(dead_code, clippy::too_many_arguments)]
#[path = "support/capability_demand_pair_execution_impl.rs"]
mod capability_demand_pair_execution;

use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use capability_demand_pair_execution::{
    MAX_EXTERNAL_RUN_METADATA_BYTES, MAX_PAIR_PLAN_BYTES, MAX_WORKER_ARTIFACT_BYTES,
    build_external_run_receipt, parse_external_run_metadata, parse_organizer_plan,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("capability-demand-run-receipt: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let plan_path = next_path(&mut args)?;
    let metadata_path = next_path(&mut args)?;
    let sampling_config_path = next_path(&mut args)?;
    let checkout_reset_path = next_path(&mut args)?;
    let worker_output_path = next_path(&mut args)?;
    if args.next().is_some() {
        return Err(usage().into());
    }

    let plan = parse_organizer_plan(&read_bounded(&plan_path, MAX_PAIR_PLAN_BYTES)?)
        .map_err(invalid_data)?;
    let metadata = parse_external_run_metadata(&read_bounded(
        &metadata_path,
        MAX_EXTERNAL_RUN_METADATA_BYTES,
    )?)
    .map_err(invalid_data)?;
    let sampling_config = read_bounded(&sampling_config_path, MAX_WORKER_ARTIFACT_BYTES)?;
    let checkout_reset = read_bounded(&checkout_reset_path, MAX_WORKER_ARTIFACT_BYTES)?;
    let worker_output = read_bounded(&worker_output_path, MAX_WORKER_ARTIFACT_BYTES)?;

    let receipt = build_external_run_receipt(
        &plan,
        &metadata,
        &sampling_config,
        &checkout_reset,
        &worker_output,
    )
    .map_err(invalid_data)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn next_path(args: &mut impl Iterator<Item = String>) -> Result<String, io::Error> {
    args.next().ok_or_else(usage)
}

fn usage() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: capability_demand_run_receipt ORGANIZER_PLAN.json RUN_METADATA.json SAMPLING_CONFIG RESET_RECEIPT RAW_WORKER_OUTPUT",
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

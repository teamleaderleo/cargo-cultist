#[path = "../src/observation_reconciliation.rs"]
mod observation_reconciliation;
#[path = "../src/project_memory.rs"]
mod project_memory;

use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use observation_reconciliation::{
    MAX_OBSERVATION_RECONCILIATION_BYTES, evaluate_observation_reconciliation,
    parse_observation_reconciliation_claim,
};
use project_memory::{MAX_PROJECT_MEMORY_BYTES, parse_project_memory_packet};

fn main() {
    if let Err(error) = run() {
        eprintln!("observation-reconciliation: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let memory_path = args.next().ok_or_else(usage_error)?;
    let claim_path = args.next().ok_or_else(usage_error)?;
    if args.next().is_some() {
        return Err(usage_error().into());
    }

    let memory_bytes = read_bounded(&memory_path, MAX_PROJECT_MEMORY_BYTES)?;
    let claim_bytes = read_bounded(&claim_path, MAX_OBSERVATION_RECONCILIATION_BYTES)?;
    let memory = parse_project_memory_packet(&memory_bytes).map_err(invalid_data)?;
    memory.summary().map_err(invalid_data)?;
    let claim = parse_observation_reconciliation_claim(&claim_bytes).map_err(invalid_data)?;
    let evaluation = evaluate_observation_reconciliation(&memory, &claim).map_err(invalid_data)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
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
        "usage: observation_reconciliation PROJECT_MEMORY.json RECONCILIATION_CLAIM.json",
    )
}

fn invalid_data(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

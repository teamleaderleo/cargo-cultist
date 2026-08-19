use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

#[path = "support/capability_demand_retirement_impl.rs"]
mod capability_demand_retirement_impl;

use capability_demand_retirement_impl::{RunReceipt, TrialInputManifest, evaluate_pair};

const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_RUN_RECEIPT_BYTES: u64 = 64 * 1024;

fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_bytes {
        return Err(format!(
            "{} is {} bytes; maximum is {} bytes",
            path.display(),
            metadata.len(),
            max_bytes
        )
        .into());
    }
    Ok(fs::read(path)?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage: capability_demand_retirement_score MANIFEST BASELINE_OR_TREATMENT_A BASELINE_OR_TREATMENT_B"
                .into(),
        );
    }

    let manifest: TrialInputManifest = serde_json::from_slice(&read_bounded(
        Path::new(&args[0]),
        MAX_MANIFEST_BYTES,
    )?)?;
    let left: RunReceipt = serde_json::from_slice(&read_bounded(
        Path::new(&args[1]),
        MAX_RUN_RECEIPT_BYTES,
    )?)?;
    let right: RunReceipt = serde_json::from_slice(&read_bounded(
        Path::new(&args[2]),
        MAX_RUN_RECEIPT_BYTES,
    )?)?;

    let evaluation = evaluate_pair(&manifest, &left, &right);
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

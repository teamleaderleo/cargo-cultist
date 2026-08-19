use std::error::Error;
use std::io::{self, Read};

#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;

use behavioral_receipt::{BehavioralReceipt, MAX_BEHAVIORAL_RECEIPT_BYTES, validate_receipt};

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-receipt: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_BEHAVIORAL_RECEIPT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BEHAVIORAL_RECEIPT_BYTES {
        return Err(format!(
            "behavioral receipt exceeds the {MAX_BEHAVIORAL_RECEIPT_BYTES}-byte limit"
        )
        .into());
    }

    let receipt: BehavioralReceipt = serde_json::from_slice(&bytes)?;
    validate_receipt(&receipt)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

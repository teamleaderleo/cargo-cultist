#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

#[path = "../src/discriminator_observation.rs"]
mod discriminator_observation;

use discriminator_observation::{
    MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES, enumerate_discriminator_partitions,
    parse_discriminator_observation_batch,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("discriminator-observations: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES {
        return Err(format!(
            "discriminator observation batch exceeds the {MAX_DISCRIMINATOR_OBSERVATION_BATCH_BYTES}-byte limit"
        )
        .into());
    }

    let batch = parse_discriminator_observation_batch(&bytes)?;
    let enumeration = enumerate_discriminator_partitions(&batch)?;
    println!("{}", serde_json::to_string_pretty(&enumeration)?);
    Ok(())
}

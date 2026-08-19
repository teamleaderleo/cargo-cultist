#![allow(dead_code)]

use std::error::Error;
use std::io::{self, Read};

#[path = "../src/refinement_episode.rs"]
mod refinement_episode;

use refinement_episode::{MAX_REFINEMENT_EPISODE_BATCH_BYTES, parse_refinement_episode_batch};

fn main() {
    if let Err(error) = run() {
        eprintln!("refinement-episodes: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_REFINEMENT_EPISODE_BATCH_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_REFINEMENT_EPISODE_BATCH_BYTES {
        return Err(format!(
            "refinement episode batch exceeds the {MAX_REFINEMENT_EPISODE_BATCH_BYTES}-byte limit"
        )
        .into());
    }

    let batch = parse_refinement_episode_batch(&bytes)?;
    println!("{}", serde_json::to_string_pretty(&batch)?);
    Ok(())
}

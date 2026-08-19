use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/behavioral_episode.rs"]
mod behavioral_episode;
#[allow(dead_code)]
#[path = "../src/behavioral_receipt.rs"]
mod behavioral_receipt;
#[path = "../src/behavioral_summary.rs"]
mod behavioral_summary;

use behavioral_episode::{MAX_BEHAVIORAL_EPISODE_BATCH_BYTES, parse_behavioral_episode_batch};
use behavioral_summary::summarize_behavioral_episodes;

fn main() {
    if let Err(error) = run() {
        eprintln!("behavioral-summary: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_BEHAVIORAL_EPISODE_BATCH_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BEHAVIORAL_EPISODE_BATCH_BYTES {
        return Err(format!(
            "behavioral episode batch exceeds the {MAX_BEHAVIORAL_EPISODE_BATCH_BYTES}-byte limit"
        )
        .into());
    }

    let batch = parse_behavioral_episode_batch(&bytes)?;
    let summary = summarize_behavioral_episodes(&batch)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

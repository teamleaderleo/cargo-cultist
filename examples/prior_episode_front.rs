use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[allow(dead_code)]
#[path = "../src/closure_episode.rs"]
mod closure_episode;
#[allow(dead_code)]
#[path = "../src/prior_episode_front.rs"]
mod prior_episode_front;
#[allow(dead_code)]
#[path = "../src/review_memory.rs"]
mod review_memory;

use prior_episode_front::{
    MAX_PRIOR_EPISODE_FRONT_QUERY_BYTES, evaluate_prior_episode_front,
    parse_prior_episode_front_query,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("prior-episode-front: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_PRIOR_EPISODE_FRONT_QUERY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_PRIOR_EPISODE_FRONT_QUERY_BYTES {
        return Err(format!(
            "prior-episode-front query exceeds the {MAX_PRIOR_EPISODE_FRONT_QUERY_BYTES}-byte limit"
        )
        .into());
    }

    let query = parse_prior_episode_front_query(&bytes)?;
    let front = evaluate_prior_episode_front(&query)?;
    println!("{}", serde_json::to_string_pretty(&front)?);
    Ok(())
}

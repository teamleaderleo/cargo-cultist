use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/closure_episode.rs"]
mod closure_episode;

use closure_episode::{MAX_CLOSURE_EPISODE_BYTES, evaluate_closure_episode, parse_closure_episode};

fn main() {
    if let Err(error) = run() {
        eprintln!("closure-episode: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_CLOSURE_EPISODE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_CLOSURE_EPISODE_BYTES {
        return Err(
            format!("closure episode exceeds the {MAX_CLOSURE_EPISODE_BYTES}-byte limit").into(),
        );
    }

    let episode = parse_closure_episode(&bytes)?;
    let evaluation = evaluate_closure_episode(&episode)?;
    println!("{}", serde_json::to_string_pretty(&evaluation)?);
    Ok(())
}

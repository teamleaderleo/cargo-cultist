use std::error::Error;
use std::io::{self, Read};

#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;
#[allow(dead_code)]
#[path = "../src/closure_episode.rs"]
mod closure_episode;
#[allow(dead_code)]
#[path = "../src/lesson_promotion.rs"]
mod lesson_promotion;
#[allow(dead_code)]
#[path = "../src/observation_reconciliation.rs"]
mod observation_reconciliation;
#[allow(dead_code)]
#[path = "../src/prior_episode_detail.rs"]
mod prior_episode_detail;
#[allow(dead_code)]
#[path = "../src/prior_episode_front.rs"]
mod prior_episode_front;
#[allow(dead_code)]
#[path = "../src/project_memory.rs"]
mod project_memory;
#[allow(dead_code)]
#[path = "../src/proof_surface.rs"]
mod proof_surface;
#[allow(dead_code)]
#[path = "../src/proxy_revision.rs"]
mod proxy_revision;
#[allow(dead_code)]
#[path = "../src/review_memory.rs"]
mod review_memory;

use prior_episode_detail::project_prior_episode_detail;
use prior_episode_front::{
    MAX_PRIOR_EPISODE_FRONT_QUERY_BYTES, parse_prior_episode_front_query,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("prior-episode-detail: {error}");
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
    if query.inputs.len() != 1 {
        return Err("prior-episode-detail requires exactly one selected input".into());
    }
    let detail = project_prior_episode_detail(&query.inputs[0])?;
    println!("{}", serde_json::to_string_pretty(&detail)?);
    Ok(())
}

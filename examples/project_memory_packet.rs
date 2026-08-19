#[path = "../src/project_memory.rs"]
mod project_memory;

use std::error::Error;
use std::io::{self, ErrorKind, Read};

use project_memory::{MAX_PROJECT_MEMORY_BYTES, parse_project_memory_packet};

fn main() {
    if let Err(error) = run() {
        eprintln!("project-memory-packet: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut input = Vec::new();
    io::stdin()
        .take((MAX_PROJECT_MEMORY_BYTES + 1) as u64)
        .read_to_end(&mut input)?;
    let packet = parse_project_memory_packet(&input)
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
    let summary = packet
        .summary()
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

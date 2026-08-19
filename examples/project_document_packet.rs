#[path = "../src/project_document.rs"]
mod project_document;

use std::error::Error;
use std::io::{self, ErrorKind, Read};

use project_document::{MAX_PROJECT_DOCUMENT_PACKET_BYTES, parse_project_document_packet};

fn main() {
    if let Err(error) = run() {
        eprintln!("project-document-packet: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut input = Vec::new();
    io::stdin()
        .take((MAX_PROJECT_DOCUMENT_PACKET_BYTES + 1) as u64)
        .read_to_end(&mut input)?;
    let packet = parse_project_document_packet(&input)
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
    let summary = packet
        .summary()
        .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

#[allow(dead_code)]
#[path = "../src/coordination_edges.rs"]
mod coordination_edges;

use std::error::Error;
use std::io::{self, Read, Write};

use coordination_edges::{MAX_SNAPSHOT_BYTES, extract_snapshot};

fn main() {
    if let Err(error) = run() {
        eprintln!("coordination-edges: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_SNAPSHOT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(format!(
            "input exceeds coordination metadata limit of {MAX_SNAPSHOT_BYTES} bytes"
        )
        .into());
    }

    let input = String::from_utf8(bytes)?;
    let report = extract_snapshot(&input)?;
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, &report)?;
    writeln!(output)?;
    Ok(())
}

#[allow(dead_code, clippy::too_many_arguments)]
#[path = "support/capability_demand_pair_trial_binding.rs"]
mod capability_demand_pair_trial_binding;

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use capability_demand_pair_trial_binding::{
    MAX_WORKER_ARTIFACT_BYTES, parse_order_selection, prepare_bound_pair,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("capability-demand-pair-prepare: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let trial_path = next_path(&mut args)?;
    let manifest_path = next_path(&mut args)?;
    let task_path = next_path(&mut args)?;
    let patch_path = next_path(&mut args)?;
    let packet_one_path = next_path(&mut args)?;
    let packet_two_path = next_path(&mut args)?;
    let output_dir = next_path(&mut args)?;
    let pair_id = args.next().ok_or_else(usage)?;
    let order = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage().into());
    }

    let trial = read_bounded(&trial_path, MAX_WORKER_ARTIFACT_BYTES)?;
    let manifest = read_bounded(&manifest_path, MAX_WORKER_ARTIFACT_BYTES)?;
    let task = read_bounded(&task_path, MAX_WORKER_ARTIFACT_BYTES)?;
    let patch = read_bounded(&patch_path, MAX_WORKER_ARTIFACT_BYTES)?;
    let packet_one = read_bounded(&packet_one_path, MAX_WORKER_ARTIFACT_BYTES)?;
    let packet_two = read_bounded(&packet_two_path, MAX_WORKER_ARTIFACT_BYTES)?;
    let order = parse_order_selection(&order).map_err(invalid_data)?;

    let prepared = prepare_bound_pair(
        &trial,
        &manifest,
        &task,
        &patch,
        &packet_one,
        &packet_two,
        &pair_id,
        order,
    )
    .map_err(invalid_data)?;

    let output_dir = PathBuf::from(output_dir);
    fs::create_dir(&output_dir)?;
    write_json(output_dir.join("organizer-plan.json"), &prepared.plan)?;

    for slot in prepared.slots {
        let slot_dir = output_dir.join(&slot.metadata.slot_id);
        fs::create_dir(&slot_dir)?;
        write_json(slot_dir.join("worker-input.json"), &slot.metadata)?;
        fs::write(slot_dir.join("task.txt"), slot.task)?;
        fs::write(slot_dir.join("proposed.patch"), slot.patch)?;
        fs::write(slot_dir.join("evidence.json"), slot.evidence)?;
    }

    println!("{}", serde_json::to_string_pretty(&prepared.plan)?);
    Ok(())
}

fn next_path(args: &mut impl Iterator<Item = String>) -> Result<String, io::Error> {
    args.next().ok_or_else(usage)
}

fn usage() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: capability_demand_pair_prepare TRIAL.json MANIFEST.json TASK.txt PATCH.diff PACKET_ONE.json PACKET_TWO.json OUTPUT_DIR PAIR_ID AB|BA|seed:<seed>",
    )
}

fn read_bounded(path: impl AsRef<Path>, maximum: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)?;
    if metadata.len() > maximum as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} exceeds {maximum} bytes", path.display()),
        )
        .into());
    }
    Ok(fs::read(path)?)
}

fn write_json(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn invalid_data(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

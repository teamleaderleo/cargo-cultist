#[path = "capability_demand_pair_execution_impl.rs"]
pub mod core;

pub use core::{MAX_WORKER_ARTIFACT_BYTES, OrderSelection, PreparedPair, parse_order_selection};

pub fn prepare_bound_pair(
    trial_bytes: &[u8],
    manifest_bytes: &[u8],
    task_bytes: &[u8],
    patch_bytes: &[u8],
    packet_one: &[u8],
    packet_two: &[u8],
    pair_id: &str,
    order_selection: OrderSelection,
) -> Result<PreparedPair, String> {
    let spec = core::capability_demand_retirement::parse_trial_spec(trial_bytes)?;
    let expected_task = line_terminated(&spec.worker_task.prompt);
    let expected_patch = line_terminated(&spec.worker_task.patch);

    if task_bytes != expected_task {
        return Err("task bytes do not match the frozen trial worker prompt".into());
    }
    if patch_bytes != expected_patch {
        return Err("patch bytes do not match the frozen trial proposed patch".into());
    }

    core::prepare_pair(
        trial_bytes,
        manifest_bytes,
        task_bytes,
        patch_bytes,
        packet_one,
        packet_two,
        pair_id,
        order_selection,
    )
}

fn line_terminated(value: &str) -> Vec<u8> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(b'\n');
    bytes
}

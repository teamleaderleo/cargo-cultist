#[allow(dead_code)]
#[path = "../src/coordination_edges.rs"]
mod coordination_edges;

use coordination_edges::{CoordinationKind, extract_snapshot};

#[test]
fn replays_public_preflight_748_hold_merge_clause() {
    let report =
        extract_snapshot(include_str!("../research/fixtures/preflight-748-hold.json")).unwrap();

    assert_eq!(report.coordination_edges.len(), 1);
    let edge = &report.coordination_edges[0];
    assert_eq!(edge.kind, CoordinationKind::HoldMergeWhile);
    assert_eq!(edge.from, "#748");
    assert_eq!(edge.to, "#703");
    assert_eq!(edge.source, "github:pull/748");

    let receipt = &report.source_receipts[0];
    assert_eq!(
        receipt.source_head_sha,
        "a2e14c4265e3568d8f943906a53e3b0e16dca141"
    );
    assert_eq!(receipt.source_updated_at, "2026-08-18T19:11:15Z");
    assert!(
        receipt
            .matched_clause
            .starts_with("Do not merge while #703 is using current-main package evidence")
    );
    assert_eq!(report.stats.unresolved_endpoints_ignored, 0);
}

#[allow(dead_code)]
#[path = "../src/compact_ir.rs"]
mod compact_ir;
#[allow(dead_code)]
#[path = "../src/finding.rs"]
mod finding;
#[allow(dead_code)]
#[path = "../src/render.rs"]
mod render;

use finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};
use render::{render_analysis_report, render_terse_analysis_report};

fn representative_report() -> AnalysisReport {
    AnalysisReport {
        schema_version: 1,
        analysis: "projection-compression-research".to_string(),
        repository: "/repo".to_string(),
        claims: vec![
            Claim::new(
                ClaimKind::Derived,
                "Current work and active work were compared at the admitted coordinates.",
            )
            .with_evidence(Evidence::new(
                "Current work is #121 at exact head abcdef1234567890.",
            ))
            .with_evidence(Evidence::new(
                "Other work is #124 at exact head fedcba0987654321.",
            )),
        ],
        findings: vec![
            Finding::new("preflight-explicit-coordination", "Explicit coordination edge")
                .at(Location::new("src/auth.rs", Some(42)))
                .with_claim(
                    Claim::new(
                        ClaimKind::Observed,
                        "The admitted inventory records `hold_merge_while` from `#121` to `#124`.",
                    )
                    .with_evidence(Evidence::new(
                        "Coordination source reference: github:pull/121.",
                    ))
                    .with_evidence(Evidence::new(
                        "Related active work #124 is at exact head fedcba0987654321.",
                    )),
                )
                .with_claim(Claim::new(
                    ClaimKind::Unknown,
                    "The inventory does not establish the operational consequence beyond the declared relation.",
                ))
                .with_question(
                    "Should merge order be coordinated before either change advances the evidence baseline?",
                ),
        ],
    }
}

#[test]
fn compression_ladder_separates_lossless_transport_from_lossy_projection() {
    let report = representative_report();

    let text = render_analysis_report(&report);
    let json = serde_json::to_string(&report).unwrap();
    let c1 = compact_ir::encode_report(&report).unwrap();
    let terse = render_terse_analysis_report(&report);

    assert_eq!(compact_ir::decode_report(&c1).unwrap(), report);

    // C1 is the compact lossless baseline. Terse is allowed to be smaller
    // because it is a decision projection, not a replacement serialization.
    assert!(c1.len() < json.len(), "C1 unexpectedly >= minified JSON");
    assert!(terse.len() < c1.len(), "terse unexpectedly >= lossless C1");
    assert!(terse.len() < text.len(), "terse unexpectedly >= human text");
}

#[test]
fn terse_keeps_the_material_unknown_and_question_but_drops_support_receipts() {
    let report = representative_report();
    let terse = render_terse_analysis_report(&report);

    assert!(terse.contains("? The inventory does not establish"));
    assert!(terse.contains("Q Should merge order be coordinated"));
    assert!(!terse.contains("Coordination source reference"));
    assert!(!terse.contains("Related active work #124 is at exact head"));
}

#[test]
fn lossy_projection_cannot_be_used_as_the_round_trip_storage_format() {
    let report = representative_report();
    let terse = render_terse_analysis_report(&report);
    let c1 = compact_ir::encode_report(&report).unwrap();

    // The evidence strings are intentionally absent from terse but present in
    // the lossless carrier. This is a positive demonstration that the two
    // formats solve different jobs, not a parser round-trip expectation.
    assert!(!terse.contains("github:pull/121"));
    assert!(c1.contains("github:pull/121"));
    assert!(!terse.contains("abcdef1234567890"));
    assert!(c1.contains("abcdef1234567890"));
}

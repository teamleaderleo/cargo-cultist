#[path = "../src/project_memory.rs"]
mod project_memory;

use project_memory::{
    ArtifactKind, ArtifactRef, MAX_PROJECT_MEMORY_BYTES, MemoryRelation,
    parse_project_memory_packet,
};

const STENSIBLY_1575: &[u8] = include_bytes!("../research/project-memory/stensibly-1575.json");

#[test]
fn retained_stensibly_packet_preserves_explicit_lineage() {
    let packet = parse_project_memory_packet(STENSIBLY_1575).unwrap();
    let summary = packet.summary().unwrap();

    assert_eq!(summary.repository, "teamleaderleo/stensibly");
    assert_eq!(
        summary.anchor,
        ArtifactRef {
            kind: ArtifactKind::PullRequest,
            number: 1575,
        }
    );
    assert_eq!(summary.artifact_count, 5);
    assert_eq!(summary.edge_count, 7);
    assert_eq!(
        summary.anchor_changed_paths,
        vec!["test/convex-index-identifier-limit.test.ts".to_string()]
    );

    let follow_ups: Vec<_> = summary
        .explicit_anchor_links
        .iter()
        .filter(|link| link.relation == MemoryRelation::FollowUpTo)
        .map(|link| link.target.number)
        .collect();
    assert_eq!(follow_ups, vec![1569, 1571, 1573]);

    let closes: Vec<_> = summary
        .explicit_anchor_links
        .iter()
        .filter(|link| link.relation == MemoryRelation::Closes)
        .map(|link| link.target.number)
        .collect();
    assert_eq!(closes, vec![1574]);
}

#[test]
fn chronology_alone_does_not_create_project_memory_links() {
    let packet = parse_project_memory_packet(STENSIBLY_1575).unwrap();
    let summary = packet.summary().unwrap();

    // Five dated artifacts are present. The anchor exposes only the four links
    // backed by exact retained text in PR #1575.
    assert_eq!(summary.artifact_count, 5);
    assert_eq!(summary.explicit_anchor_links.len(), 4);
}

#[test]
fn invented_edge_evidence_is_rejected() {
    let mut packet = parse_project_memory_packet(STENSIBLY_1575).unwrap();
    packet.edges[1].evidence = "Chronology proves these changes caused the guard.".to_string();

    let error = packet.validate().unwrap_err();
    assert!(error.contains("absent from the source artifact evidence text"));
}

#[test]
fn missing_edge_target_is_rejected() {
    let mut packet = parse_project_memory_packet(STENSIBLY_1575).unwrap();
    packet.edges[0].to = ArtifactRef {
        kind: ArtifactKind::Issue,
        number: 999_999,
    };

    let error = packet.validate().unwrap_err();
    assert!(error.contains("edge target issue#999999 is absent"));
}

#[test]
fn issue_cannot_claim_pull_request_revision_coordinates() {
    let mut packet = parse_project_memory_packet(STENSIBLY_1575).unwrap();
    let revision = packet.artifacts[0].revision.clone();
    let issue = packet
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference.kind == ArtifactKind::Issue)
        .unwrap();
    issue.revision = revision;

    let error = packet.validate().unwrap_err();
    assert!(error.contains("may not carry pull-request revision coordinates"));
}

#[test]
fn packet_input_is_bounded_before_json_parse() {
    let bytes = vec![b' '; MAX_PROJECT_MEMORY_BYTES + 1];
    let error = parse_project_memory_packet(&bytes).unwrap_err();
    assert!(error.contains("exceeds"));
}

#[allow(dead_code)]
#[path = "../src/project_memory.rs"]
mod project_memory;

use project_memory::{ArtifactKind, ArtifactRef, MemoryRelation, ProjectMemoryPacket};

const STENSIBLY_1575: &[u8] = include_bytes!("../research/project-memory/stensibly-1575.json");

fn packet() -> ProjectMemoryPacket {
    serde_json::from_slice(STENSIBLY_1575).unwrap()
}

#[test]
fn retained_packet_still_admits_its_explicit_relationship_lines() {
    packet().validate().unwrap();
}

#[test]
fn related_excerpt_cannot_be_strengthened_to_closes() {
    let mut packet = packet();
    let anchor = packet.anchor;
    let source = packet
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == anchor)
        .unwrap();
    source.evidence_text.push_str("\nRelated: #1574");

    packet.edges[0].relation = MemoryRelation::Closes;
    packet.edges[0].evidence = "Related: #1574".to_string();

    let error = packet.validate().unwrap_err();
    assert!(error.contains("disagrees with source evidence"));
}

#[test]
fn parent_excerpt_cannot_be_relabelled_follow_up() {
    let mut packet = packet();
    let anchor = packet.anchor;
    let source = packet
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == anchor)
        .unwrap();
    source.evidence_text.push_str("\nParent: #1569");

    let edge = packet
        .edges
        .iter_mut()
        .find(|edge| edge.to.number == 1569)
        .unwrap();
    edge.relation = MemoryRelation::FollowUpTo;
    edge.evidence = "Parent: #1569".to_string();

    let error = packet.validate().unwrap_err();
    assert!(error.contains("disagrees with source evidence"));
}

#[test]
fn relationship_line_must_name_the_declared_target() {
    let mut packet = packet();
    let edge = packet
        .edges
        .iter_mut()
        .find(|edge| edge.to.number == 1569)
        .unwrap();
    edge.to = ArtifactRef {
        kind: ArtifactKind::Issue,
        number: 1574,
    };

    let error = packet.validate().unwrap_err();
    assert!(error.contains("does not explicitly mention target"));
}

#[test]
fn arbitrary_source_excerpt_cannot_be_given_a_typed_relation() {
    let mut packet = packet();
    let anchor = packet.anchor;
    let source = packet
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.reference == anchor)
        .unwrap();
    source.evidence_text.push_str("\nSee #1574 for background.");

    packet.edges[0].relation = MemoryRelation::Related;
    packet.edges[0].evidence = "See #1574 for background.".to_string();

    let error = packet.validate().unwrap_err();
    assert!(error.contains("not an admitted explicit relationship line"));
}

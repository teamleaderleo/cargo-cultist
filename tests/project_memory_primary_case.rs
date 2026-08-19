#[allow(dead_code)]
#[path = "../src/project_memory.rs"]
mod project_memory;

use project_memory::{
    ArtifactKind, ArtifactRef, ArtifactState, MemoryRelation, PROJECT_MEMORY_SCHEMA_VERSION,
    ProjectArtifact, ProjectMemoryEdge, ProjectMemoryPacket,
};

fn issue(number: u64, evidence_text: &str) -> ProjectArtifact {
    ProjectArtifact {
        reference: ArtifactRef {
            kind: ArtifactKind::Issue,
            number,
        },
        title: format!("Issue {number}"),
        state: ArtifactState::Open,
        created_at: "2026-08-19T00:00:00Z".to_string(),
        closed_at: None,
        revision: None,
        changed_paths: Vec::new(),
        evidence_text: evidence_text.to_string(),
        evidence_complete: true,
    }
}

fn packet(evidence: &str, relation: MemoryRelation, target: u64) -> ProjectMemoryPacket {
    ProjectMemoryPacket {
        schema_version: PROJECT_MEMORY_SCHEMA_VERSION,
        repository: "teamleaderleo/linux-fieldwork".to_string(),
        anchor: ArtifactRef {
            kind: ArtifactKind::Issue,
            number: 675,
        },
        artifacts: vec![
            issue(675, evidence),
            issue(609, "case 609"),
            issue(611, "case 611"),
        ],
        edges: vec![ProjectMemoryEdge {
            from: ArtifactRef {
                kind: ArtifactKind::Issue,
                number: 675,
            },
            relation,
            to: ArtifactRef {
                kind: ArtifactKind::Issue,
                number: target,
            },
            evidence: evidence.to_string(),
        }],
    }
}

#[test]
fn admits_exact_primary_case_block_from_issue_collector() {
    let packet = packet(
        "Primary case:\n\nhttps://github.com/teamleaderleo/linux-fieldwork/issues/609",
        MemoryRelation::Related,
        609,
    );
    packet.validate().unwrap();
}

#[test]
fn admits_redirect_github_primary_case_url() {
    let packet = packet(
        "Primary case:\nhttps://redirect.github.com/teamleaderleo/linux-fieldwork/issues/609",
        MemoryRelation::Related,
        609,
    );
    packet.validate().unwrap();
}

#[test]
fn primary_case_block_cannot_be_strengthened_to_closes() {
    let packet = packet(
        "Primary case:\nhttps://github.com/teamleaderleo/linux-fieldwork/issues/609",
        MemoryRelation::Closes,
        609,
    );
    let error = packet.validate().unwrap_err();
    assert!(error.contains("must use relation=related"));
}

#[test]
fn primary_case_block_cannot_escape_packet_repository() {
    let packet = packet(
        "Primary case:\nhttps://github.com/teamleaderleo/other-repo/issues/609",
        MemoryRelation::Related,
        609,
    );
    let error = packet.validate().unwrap_err();
    assert!(error.contains("not an admitted Primary case block"));
}

#[test]
fn primary_case_url_must_name_declared_target() {
    let packet = packet(
        "Primary case:\nhttps://github.com/teamleaderleo/linux-fieldwork/issues/609",
        MemoryRelation::Related,
        611,
    );
    let error = packet.validate().unwrap_err();
    assert!(error.contains("names issue #609"));
}

#[test]
fn primary_case_block_rejects_extra_nonempty_prose() {
    let packet = packet(
        "Primary case:\nhttps://github.com/teamleaderleo/linux-fieldwork/issues/609\nThis is also related to another case.",
        MemoryRelation::Related,
        609,
    );
    let error = packet.validate().unwrap_err();
    assert!(error.contains("not an admitted Primary case block"));
}

#[test]
fn multiline_related_line_cannot_bypass_single_line_relation_grammar() {
    let packet = packet("Related: #609\nextra prose", MemoryRelation::Related, 609);
    let error = packet.validate().unwrap_err();
    assert!(error.contains("not an admitted Primary case block"));
}

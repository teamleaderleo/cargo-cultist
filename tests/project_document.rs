#[path = "../src/project_document.rs"]
mod project_document;

use project_document::{
    DocumentSourceKind, DocumentSourceRef, MAX_PROJECT_DOCUMENT_PACKET_BYTES, ProjectDocument,
    ProjectDocumentPacket, parse_project_document_packet,
};

fn packet() -> ProjectDocumentPacket {
    ProjectDocumentPacket {
        schema_version: 1,
        repository: "teamleaderleo/linux-fieldwork".to_string(),
        revision: "b835ed842299f7654afc00f4988f7586e0be63bc".to_string(),
        documents: vec![ProjectDocument {
            path: "investigations/cloud-hypervisor-qcow-r609-review/README.md".to_string(),
            blob_sha: "1111111111111111111111111111111111111111".to_string(),
            text: "prepare -> own -> publish -> retire\n".to_string(),
            text_complete: true,
            source: DocumentSourceRef {
                kind: DocumentSourceKind::Issue,
                number: 609,
            },
            source_evidence: concat!(
                "Detailed investigation and provenance:\n\n",
                "`investigations/cloud-hypervisor-qcow-r609-review/README.md`"
            )
            .to_string(),
        }],
    }
}

#[test]
fn validates_exact_revision_path_blob_and_source() {
    let packet = packet();
    let summary = packet.summary().unwrap();

    assert_eq!(summary.repository, "teamleaderleo/linux-fieldwork");
    assert_eq!(
        summary.revision,
        "b835ed842299f7654afc00f4988f7586e0be63bc"
    );
    assert_eq!(summary.document_count, 1);
    assert_eq!(
        summary.documents[0].path,
        "investigations/cloud-hypervisor-qcow-r609-review/README.md"
    );
    assert_eq!(
        summary.documents[0].source.number,
        609
    );
    assert!(summary.documents[0].text_complete);
}

#[test]
fn source_evidence_must_name_document_path() {
    let mut packet = packet();
    packet.documents[0].source_evidence = "Detailed investigation lives elsewhere.".to_string();

    let error = packet.validate().unwrap_err();
    assert!(error.contains("does not name the document path"));
}

#[test]
fn duplicate_document_paths_are_rejected() {
    let mut packet = packet();
    packet.documents.push(packet.documents[0].clone());

    let error = packet.validate().unwrap_err();
    assert!(error.contains("duplicate project-document path"));
}

#[test]
fn noncanonical_paths_are_rejected() {
    let mut packet = packet();
    packet.documents[0].path = "investigations/../README.md".to_string();
    packet.documents[0].source_evidence = "`investigations/../README.md`".to_string();

    let error = packet.validate().unwrap_err();
    assert!(error.contains("non-canonical"));
}

#[test]
fn malformed_blob_identity_is_rejected() {
    let mut packet = packet();
    packet.documents[0].blob_sha = "deadbeef".to_string();

    let error = packet.validate().unwrap_err();
    assert!(error.contains("blob_sha"));
}

#[test]
fn packet_input_is_bounded_before_json_parse() {
    let bytes = vec![b' '; MAX_PROJECT_DOCUMENT_PACKET_BYTES + 1];
    let error = parse_project_document_packet(&bytes).unwrap_err();
    assert!(error.contains("exceeds"));
}

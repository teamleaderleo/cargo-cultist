use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const PROJECT_DOCUMENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROJECT_DOCUMENT_PACKET_BYTES: usize = 512 * 1024;
const MAX_DOCUMENTS: usize = 16;
const MAX_DOCUMENT_TEXT_BYTES: usize = 128 * 1024;
const MAX_SOURCE_EVIDENCE_BYTES: usize = 8 * 1024;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentSourceKind {
    PullRequest,
    Issue,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentSourceRef {
    pub kind: DocumentSourceKind,
    pub number: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDocument {
    pub path: String,
    pub blob_sha: String,
    pub text: String,
    pub text_complete: bool,
    pub source: DocumentSourceRef,
    pub source_evidence: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDocumentPacket {
    pub schema_version: u32,
    pub repository: String,
    pub revision: String,
    pub documents: Vec<ProjectDocument>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ProjectDocumentSummaryEntry {
    pub path: String,
    pub blob_sha: String,
    pub text_bytes: usize,
    pub text_complete: bool,
    pub source: DocumentSourceRef,
    pub source_evidence: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ProjectDocumentSummary {
    pub schema_version: u32,
    pub repository: String,
    pub revision: String,
    pub document_count: usize,
    pub documents: Vec<ProjectDocumentSummaryEntry>,
}

pub fn parse_project_document_packet(bytes: &[u8]) -> Result<ProjectDocumentPacket, String> {
    if bytes.len() > MAX_PROJECT_DOCUMENT_PACKET_BYTES {
        return Err(format!(
            "project-document packet exceeds {} bytes",
            MAX_PROJECT_DOCUMENT_PACKET_BYTES
        ));
    }
    let packet: ProjectDocumentPacket = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid project-document JSON: {error}"))?;
    packet.validate()?;
    Ok(packet)
}

impl ProjectDocumentPacket {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PROJECT_DOCUMENT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported project-document schema version {}",
                self.schema_version
            ));
        }
        validate_repository(&self.repository)?;
        validate_sha(&self.revision, "revision")?;
        if self.documents.is_empty() || self.documents.len() > MAX_DOCUMENTS {
            return Err(format!(
                "project-document packet must contain 1..={MAX_DOCUMENTS} documents"
            ));
        }

        let mut paths = BTreeSet::new();
        for document in &self.documents {
            document.validate()?;
            if !paths.insert(document.path.as_str()) {
                return Err(format!(
                    "duplicate project-document path `{}`",
                    document.path
                ));
            }
        }
        Ok(())
    }

    pub fn summary(&self) -> Result<ProjectDocumentSummary, String> {
        self.validate()?;
        Ok(ProjectDocumentSummary {
            schema_version: PROJECT_DOCUMENT_SCHEMA_VERSION,
            repository: self.repository.clone(),
            revision: self.revision.clone(),
            document_count: self.documents.len(),
            documents: self
                .documents
                .iter()
                .map(|document| ProjectDocumentSummaryEntry {
                    path: document.path.clone(),
                    blob_sha: document.blob_sha.clone(),
                    text_bytes: document.text.len(),
                    text_complete: document.text_complete,
                    source: document.source,
                    source_evidence: document.source_evidence.clone(),
                })
                .collect(),
        })
    }
}

impl ProjectDocument {
    fn validate(&self) -> Result<(), String> {
        validate_repository_path(&self.path)?;
        validate_sha(&self.blob_sha, "blob_sha")?;
        if self.text.is_empty()
            || self.text.len() > MAX_DOCUMENT_TEXT_BYTES
            || self.text.contains('\0')
        {
            return Err(format!(
                "document `{}` text is empty, malformed, or exceeds {} bytes",
                self.path, MAX_DOCUMENT_TEXT_BYTES
            ));
        }
        if self.source.number == 0 {
            return Err(format!(
                "document `{}` source number must be positive",
                self.path
            ));
        }
        if self.source_evidence.is_empty()
            || self.source_evidence.len() > MAX_SOURCE_EVIDENCE_BYTES
            || self.source_evidence.contains('\0')
        {
            return Err(format!(
                "document `{}` source evidence is empty, malformed, or too long",
                self.path
            ));
        }
        if !self.source_evidence.contains(&self.path) {
            return Err(format!(
                "document `{}` source evidence does not name the document path",
                self.path
            ));
        }
        Ok(())
    }
}

fn validate_repository(value: &str) -> Result<(), String> {
    if value.len() > 200 || value.bytes().any(|byte| matches!(byte, b'\n' | b'\r')) {
        return Err("repository coordinate is malformed or too long".to_string());
    }
    let Some((owner, repository)) = value.split_once('/') else {
        return Err("repository coordinate must be owner/name".to_string());
    };
    if owner.is_empty()
        || repository.is_empty()
        || repository.contains('/')
        || !owner.bytes().all(valid_repository_char)
        || !repository.bytes().all(valid_repository_char)
    {
        return Err("repository coordinate must be canonical owner/name".to_string());
    }
    Ok(())
}

fn valid_repository_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

fn validate_sha(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be an exact lowercase 40-hex Git object id"
        ));
    }
    Ok(())
}

fn validate_repository_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(format!("non-canonical repository-relative path `{path}`"));
    }
    Ok(())
}

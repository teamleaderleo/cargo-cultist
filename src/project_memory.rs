use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const PROJECT_MEMORY_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROJECT_MEMORY_BYTES: usize = 256 * 1024;
const MAX_ARTIFACTS: usize = 128;
const MAX_EDGES: usize = 256;
const MAX_TITLE_BYTES: usize = 512;
const MAX_EVIDENCE_TEXT_BYTES: usize = 32 * 1024;
const MAX_EDGE_EVIDENCE_BYTES: usize = 2 * 1024;
const MAX_PATHS_PER_ARTIFACT: usize = 512;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    PullRequest,
    Issue,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub kind: ArtifactKind,
    pub number: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionCoordinate {
    pub head_sha: String,
    pub base_sha: String,
    pub merged: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectArtifact {
    pub reference: ArtifactRef,
    pub title: String,
    pub state: ArtifactState,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub revision: Option<RevisionCoordinate>,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    pub evidence_text: String,
    pub evidence_complete: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRelation {
    Closes,
    FollowUpTo,
    ContinuationFrom,
    Parent,
    Related,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMemoryEdge {
    pub from: ArtifactRef,
    pub relation: MemoryRelation,
    pub to: ArtifactRef,
    pub evidence: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMemoryPacket {
    pub schema_version: u32,
    pub repository: String,
    pub anchor: ArtifactRef,
    pub artifacts: Vec<ProjectArtifact>,
    pub edges: Vec<ProjectMemoryEdge>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ProjectMemoryLinkSummary {
    pub relation: MemoryRelation,
    pub target: ArtifactRef,
    pub evidence: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ProjectMemorySummary {
    pub schema_version: u32,
    pub repository: String,
    pub anchor: ArtifactRef,
    pub anchor_title: String,
    pub artifact_count: usize,
    pub edge_count: usize,
    pub anchor_changed_paths: Vec<String>,
    pub explicit_anchor_links: Vec<ProjectMemoryLinkSummary>,
}

pub fn parse_project_memory_packet(bytes: &[u8]) -> Result<ProjectMemoryPacket, String> {
    if bytes.len() > MAX_PROJECT_MEMORY_BYTES {
        return Err(format!(
            "project-memory packet exceeds {} bytes",
            MAX_PROJECT_MEMORY_BYTES
        ));
    }
    let packet: ProjectMemoryPacket = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid project-memory JSON: {error}"))?;
    packet.validate()?;
    Ok(packet)
}

impl ProjectMemoryPacket {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PROJECT_MEMORY_SCHEMA_VERSION {
            return Err(format!(
                "unsupported project-memory schema version {}",
                self.schema_version
            ));
        }
        validate_repository(&self.repository)?;
        validate_artifact_ref(self.anchor, "anchor")?;
        if self.artifacts.is_empty() || self.artifacts.len() > MAX_ARTIFACTS {
            return Err(format!(
                "project-memory packet must contain 1..={MAX_ARTIFACTS} artifacts"
            ));
        }
        if self.edges.len() > MAX_EDGES {
            return Err(format!(
                "project-memory packet may contain at most {MAX_EDGES} edges"
            ));
        }

        let mut artifacts = BTreeMap::new();
        for artifact in &self.artifacts {
            artifact.validate()?;
            if artifacts.insert(artifact.reference, artifact).is_some() {
                return Err(format!(
                    "duplicate project-memory artifact {}",
                    display_ref(artifact.reference)
                ));
            }
        }

        if !artifacts.contains_key(&self.anchor) {
            return Err("project-memory anchor is absent from artifacts".to_string());
        }

        for edge in &self.edges {
            validate_artifact_ref(edge.from, "edge source")?;
            validate_artifact_ref(edge.to, "edge target")?;
            if edge.from == edge.to {
                return Err("project-memory edges may not self-reference".to_string());
            }
            let Some(source) = artifacts.get(&edge.from) else {
                return Err(format!(
                    "project-memory edge source {} is absent",
                    display_ref(edge.from)
                ));
            };
            if !artifacts.contains_key(&edge.to) {
                return Err(format!(
                    "project-memory edge target {} is absent",
                    display_ref(edge.to)
                ));
            }
            validate_single_line_or_excerpt(
                &edge.evidence,
                "edge evidence",
                MAX_EDGE_EVIDENCE_BYTES,
                true,
            )?;
            if !source.evidence_text.contains(&edge.evidence) {
                return Err(format!(
                    "edge evidence for {} -> {} is absent from the source artifact evidence text",
                    display_ref(edge.from),
                    display_ref(edge.to)
                ));
            }
            validate_edge_semantics(&self.repository, edge)?;
        }

        Ok(())
    }

    pub fn summary(&self) -> Result<ProjectMemorySummary, String> {
        self.validate()?;
        let anchor = self
            .artifacts
            .iter()
            .find(|artifact| artifact.reference == self.anchor)
            .ok_or("project-memory anchor is absent from artifacts")?;
        let explicit_anchor_links = self
            .edges
            .iter()
            .filter(|edge| edge.from == self.anchor)
            .map(|edge| ProjectMemoryLinkSummary {
                relation: edge.relation,
                target: edge.to,
                evidence: edge.evidence.clone(),
            })
            .collect();

        Ok(ProjectMemorySummary {
            schema_version: PROJECT_MEMORY_SCHEMA_VERSION,
            repository: self.repository.clone(),
            anchor: self.anchor,
            anchor_title: anchor.title.clone(),
            artifact_count: self.artifacts.len(),
            edge_count: self.edges.len(),
            anchor_changed_paths: anchor.changed_paths.clone(),
            explicit_anchor_links,
        })
    }
}

impl ProjectArtifact {
    fn validate(&self) -> Result<(), String> {
        validate_artifact_ref(self.reference, "artifact")?;
        validate_single_line_or_excerpt(&self.title, "artifact title", MAX_TITLE_BYTES, false)?;
        validate_timestamp(&self.created_at, "created_at")?;
        match (self.state, self.closed_at.as_deref()) {
            (ArtifactState::Open, None) => {}
            (ArtifactState::Closed, Some(value)) => validate_timestamp(value, "closed_at")?,
            (ArtifactState::Open, Some(_)) => {
                return Err(format!(
                    "open artifact {} may not have closed_at",
                    display_ref(self.reference)
                ));
            }
            (ArtifactState::Closed, None) => {
                return Err(format!(
                    "closed artifact {} requires closed_at",
                    display_ref(self.reference)
                ));
            }
        }

        validate_single_line_or_excerpt(
            &self.evidence_text,
            "artifact evidence text",
            MAX_EVIDENCE_TEXT_BYTES,
            true,
        )?;
        if self.changed_paths.len() > MAX_PATHS_PER_ARTIFACT {
            return Err(format!(
                "artifact {} exceeds the changed-path bound",
                display_ref(self.reference)
            ));
        }
        for path in &self.changed_paths {
            validate_repository_path(path)?;
        }

        match self.reference.kind {
            ArtifactKind::PullRequest => {
                let Some(revision) = &self.revision else {
                    return Err(format!(
                        "pull request {} requires exact revision coordinates",
                        self.reference.number
                    ));
                };
                validate_sha(&revision.head_sha, "head_sha")?;
                validate_sha(&revision.base_sha, "base_sha")?;
                if revision.merged && self.state != ArtifactState::Closed {
                    return Err(format!(
                        "merged pull request {} must be closed",
                        self.reference.number
                    ));
                }
            }
            ArtifactKind::Issue => {
                if self.revision.is_some() {
                    return Err(format!(
                        "issue {} may not carry pull-request revision coordinates",
                        self.reference.number
                    ));
                }
                if !self.changed_paths.is_empty() {
                    return Err(format!(
                        "issue {} may not carry changed paths",
                        self.reference.number
                    ));
                }
            }
        }

        Ok(())
    }
}

fn validate_artifact_ref(reference: ArtifactRef, label: &str) -> Result<(), String> {
    if reference.number == 0 {
        return Err(format!("{label} number must be positive"));
    }
    Ok(())
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

fn validate_edge_semantics(repository: &str, edge: &ProjectMemoryEdge) -> Result<(), String> {
    if edge
        .evidence
        .bytes()
        .any(|byte| matches!(byte, b'\n' | b'\r'))
    {
        return validate_primary_case_edge(repository, edge);
    }

    let Some(classified) = classify_relation(&edge.evidence) else {
        return Err(format!(
            "edge evidence for {} -> {} is not an admitted explicit relationship line",
            display_ref(edge.from),
            display_ref(edge.to)
        ));
    };
    if classified != edge.relation {
        return Err(format!(
            "edge relation {:?} disagrees with source evidence classified as {:?}",
            edge.relation, classified
        ));
    }
    if !evidence_mentions_reference(&edge.evidence, edge.to.number) {
        return Err(format!(
            "edge evidence does not explicitly mention target {}",
            display_ref(edge.to)
        ));
    }
    Ok(())
}

fn validate_primary_case_edge(repository: &str, edge: &ProjectMemoryEdge) -> Result<(), String> {
    let Some(target) = primary_case_target(repository, &edge.evidence) else {
        return Err(format!(
            "multiline edge evidence for {} -> {} is not an admitted Primary case block",
            display_ref(edge.from),
            display_ref(edge.to)
        ));
    };
    if edge.relation != MemoryRelation::Related {
        return Err("Primary case evidence must use relation=related".to_string());
    }
    if target != edge.to.number {
        return Err(format!(
            "Primary case evidence names issue #{target}, not declared target {}",
            display_ref(edge.to)
        ));
    }
    Ok(())
}

fn primary_case_target(repository: &str, evidence: &str) -> Option<u64> {
    let mut lines = evidence
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    if !lines.next()?.eq_ignore_ascii_case("Primary case:") {
        return None;
    }
    let target = lines.next()?;
    if lines.next().is_some() {
        return None;
    }
    repository_issue_url_target(repository, target)
}

fn repository_issue_url_target(repository: &str, url: &str) -> Option<u64> {
    let normalized = url.to_ascii_lowercase();
    let repository = repository.to_ascii_lowercase();
    for origin in ["https://github.com/", "https://redirect.github.com/"] {
        let prefix = format!("{origin}{repository}/issues/");
        if let Some(rest) = normalized.strip_prefix(&prefix) {
            return positive_decimal_prefix(rest);
        }
    }
    None
}

fn positive_decimal_prefix(value: &str) -> Option<u64> {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes[0] == b'0' || !bytes[0].is_ascii_digit() {
        return None;
    }

    let mut number = 0u64;
    let mut cursor = 0usize;
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        number = number
            .checked_mul(10)?
            .checked_add((bytes[cursor] - b'0') as u64)?;
        cursor += 1;
    }
    if cursor < bytes.len() && !matches!(bytes[cursor], b'/' | b'?' | b'#') {
        return None;
    }
    Some(number)
}

fn classify_relation(evidence: &str) -> Option<MemoryRelation> {
    let normalized = evidence.trim().to_ascii_lowercase();
    if [
        "closes", "close", "closed", "fixes", "fix", "fixed", "resolves", "resolve", "resolved",
    ]
    .iter()
    .any(|word| starts_with_relation_word(&normalized, word))
    {
        return Some(MemoryRelation::Closes);
    }
    if normalized.starts_with("follow-up to") || normalized.starts_with("follow up to") {
        return Some(MemoryRelation::FollowUpTo);
    }
    if normalized.starts_with("continuation from")
        || normalized.starts_with("deployment continuation")
    {
        return Some(MemoryRelation::ContinuationFrom);
    }
    if normalized.starts_with("parent:") {
        return Some(MemoryRelation::Parent);
    }
    if normalized.starts_with("related:") {
        return Some(MemoryRelation::Related);
    }
    None
}

fn starts_with_relation_word(value: &str, word: &str) -> bool {
    value
        .strip_prefix(word)
        .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

fn evidence_mentions_reference(evidence: &str, target: u64) -> bool {
    let bytes = evidence.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }
        let mut cursor = index + 1;
        if cursor >= bytes.len() || bytes[cursor] == b'0' || !bytes[cursor].is_ascii_digit() {
            index += 1;
            continue;
        }
        let mut number = 0u64;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            let Some(next) = number
                .checked_mul(10)
                .and_then(|value| value.checked_add((bytes[cursor] - b'0') as u64))
            else {
                number = u64::MAX;
                break;
            };
            number = next;
            cursor += 1;
        }
        if number == target {
            return true;
        }
        index = cursor.max(index + 1);
    }
    false
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

fn validate_timestamp(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || value.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
    {
        return Err(format!("{label} is malformed or too long"));
    }
    Ok(())
}

fn validate_single_line_or_excerpt(
    value: &str,
    label: &str,
    max_bytes: usize,
    allow_newlines: bool,
) -> Result<(), String> {
    if value.is_empty() || value.len() > max_bytes || value.contains('\0') {
        return Err(format!("{label} is empty, malformed, or too long"));
    }
    if !allow_newlines && value.bytes().any(|byte| matches!(byte, b'\n' | b'\r')) {
        return Err(format!("{label} must be single-line"));
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

fn display_ref(reference: ArtifactRef) -> String {
    let kind = match reference.kind {
        ArtifactKind::PullRequest => "pr",
        ArtifactKind::Issue => "issue",
    };
    format!("{kind}#{}", reference.number)
}

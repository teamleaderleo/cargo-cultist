use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::finding::{AnalysisReport, Claim, ClaimKind, Evidence, Finding, Location};

const INVENTORY_SCHEMA_VERSION: u32 = 1;
const MAX_INVENTORY_BYTES: usize = 1024 * 1024;
const MAX_CHANGES: usize = 128;
const MAX_CHANGED_PATHS: usize = 1024;
const MAX_EDGES: usize = 512;
const MAX_CHANGE_ID_BYTES: usize = 128;
const MAX_REPOSITORY_BYTES: usize = 512;
const MAX_SOURCE_BYTES: usize = 512;
const MAX_PATH_BYTES: usize = 4096;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryDocument {
    schema_version: u32,
    repository: String,
    current_change: String,
    changes: Vec<InventoryChange>,
    #[serde(default)]
    coordination_edges: Vec<CoordinationEdge>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryChange {
    id: String,
    #[serde(default)]
    changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CoordinationKind {
    DependsOn,
    Blocks,
    HoldMergeWhile,
    Supersedes,
}

impl CoordinationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::DependsOn => "depends_on",
            Self::Blocks => "blocks",
            Self::HoldMergeWhile => "hold_merge_while",
            Self::Supersedes => "supersedes",
        }
    }

    fn question(self) -> &'static str {
        match self {
            Self::DependsOn => {
                "Should the dependency be integrated or settled before these changes proceed independently?"
            }
            Self::Blocks => {
                "Should the blocked change pause, rebase, or coordinate ownership before proceeding?"
            }
            Self::HoldMergeWhile => {
                "Should merge order be coordinated before either change advances the shared evidence baseline?"
            }
            Self::Supersedes => {
                "Should the superseded change be retired or reconciled before parallel work continues?"
            }
        }
    }
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinationEdge {
    kind: CoordinationKind,
    from: String,
    to: String,
    source: String,
}

#[derive(Debug)]
struct ValidatedInventory {
    repository: String,
    current_change: String,
    changes: BTreeMap<String, BTreeSet<PathBuf>>,
    coordination_edges: Vec<CoordinationEdge>,
}

pub fn build_active_inventory_analysis_report(
    root: &Path,
    inventory_path: &Path,
    scope: Option<&Path>,
) -> Result<AnalysisReport, Box<dyn Error>> {
    let bytes = read_bounded_inventory(inventory_path)?;
    let inventory = validate_inventory(serde_json::from_slice(&bytes)?)?;
    Ok(analyze_inventory(root, &inventory, scope))
}

fn analyze_inventory(
    root: &Path,
    inventory: &ValidatedInventory,
    scope: Option<&Path>,
) -> AnalysisReport {
    let mut analysis = AnalysisReport::new(
        "preflight-active-inventory",
        root.to_string_lossy().into_owned(),
    );

    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "Admitted active-change inventory schema v{INVENTORY_SCHEMA_VERSION} for `{}` with {} active change(s) and {} coordination edge(s).",
            inventory.repository,
            inventory.changes.len(),
            inventory.coordination_edges.len()
        ),
    ));

    let current_paths = scoped_paths(
        inventory
            .changes
            .get(&inventory.current_change)
            .expect("validated inventory retains current change"),
        scope,
    );
    analysis.claims.push(Claim::new(
        ClaimKind::Observed,
        format!(
            "Current change `{}` records {} changed path(s) in the selected scope.",
            inventory.current_change,
            current_paths.len()
        ),
    ));

    let mut direct_overlap_count = 0usize;
    for (other_id, paths) in &inventory.changes {
        if other_id == &inventory.current_change {
            continue;
        }
        let other_paths = scoped_paths(paths, scope);
        for path in current_paths.intersection(&other_paths) {
            direct_overlap_count += 1;
            let display = path.to_string_lossy().into_owned();
            analysis.findings.push(
                Finding::new(
                    "preflight-inventory-path-overlap",
                    "Active-change path overlap",
                )
                .at(Location::new(display.clone(), None))
                .with_claim(
                    Claim::new(
                        ClaimKind::Observed,
                        format!(
                            "The admitted inventory records both `{}` and `{other_id}` modifying `{display}`.",
                            inventory.current_change
                        ),
                    )
                    .with_evidence(Evidence::new(format!(
                        "Current change: `{}`.",
                        inventory.current_change
                    )))
                    .with_evidence(Evidence::new(format!("Other active change: `{other_id}`."))),
                )
                .with_question(
                    "Should ownership, ordering, or intent be coordinated before these changes proceed independently?",
                ),
            );
        }
    }

    if direct_overlap_count == 0 {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "The admitted inventory records no direct path overlap between the current change and the other active changes in the selected scope.",
        ));
    }

    let mut current_edge_count = 0usize;
    for edge in &inventory.coordination_edges {
        if edge.from != inventory.current_change && edge.to != inventory.current_change {
            continue;
        }
        current_edge_count += 1;

        let other = if edge.from == inventory.current_change {
            edge.to.as_str()
        } else {
            edge.from.as_str()
        };
        let pair_has_overlap = pair_has_scoped_overlap(
            inventory
                .changes
                .get(&inventory.current_change)
                .expect("validated inventory retains current change"),
            inventory
                .changes
                .get(other)
                .expect("validated edge endpoints exist"),
            scope,
        );

        let mut finding = Finding::new(
            "preflight-explicit-coordination",
            "Explicit coordination edge",
        )
        .with_claim(
            Claim::new(
                ClaimKind::Observed,
                format!(
                    "The admitted inventory records `{}` from `{}` to `{}`.",
                    edge.kind.as_str(),
                    edge.from,
                    edge.to
                ),
            )
            .with_evidence(Evidence::new(format!(
                "Source reference: `{}`.",
                edge.source
            ))),
        );

        if !pair_has_overlap {
            finding = finding.with_claim(Claim::new(
                ClaimKind::Observed,
                format!(
                    "The inventory records no direct path overlap between `{}` and `{other}` in the selected scope.",
                    inventory.current_change
                ),
            ));
        }

        finding = finding
            .with_claim(Claim::new(
                ClaimKind::Unknown,
                "The inventory does not establish the operational consequence or intent beyond the declared coordination relation.",
            ))
            .with_question(edge.kind.question());

        analysis.findings.push(finding);
    }

    if current_edge_count == 0 {
        analysis.claims.push(Claim::new(
            ClaimKind::Observed,
            "The admitted inventory contains no explicit coordination edge involving the current change.",
        ));
    }

    analysis.claims.push(Claim::new(
        ClaimKind::Unknown,
        "Inventory mode does not independently fetch provider objects or infer generated, historical, policy, or behavioral relationships absent from the supplied snapshot.",
    ));

    analysis
}

fn pair_has_scoped_overlap(
    left: &BTreeSet<PathBuf>,
    right: &BTreeSet<PathBuf>,
    scope: Option<&Path>,
) -> bool {
    let left = scoped_paths(left, scope);
    let right = scoped_paths(right, scope);
    left.intersection(&right).next().is_some()
}

fn scoped_paths(paths: &BTreeSet<PathBuf>, scope: Option<&Path>) -> BTreeSet<PathBuf> {
    match scope {
        Some(scope) => paths
            .iter()
            .filter(|path| path.starts_with(scope))
            .cloned()
            .collect(),
        None => paths.clone(),
    }
}

fn read_bounded_inventory(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_INVENTORY_BYTES as u64 {
        return Err(format!(
            "active-change inventory exceeds the {MAX_INVENTORY_BYTES}-byte limit"
        )
        .into());
    }

    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_INVENTORY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_INVENTORY_BYTES {
        return Err(format!(
            "active-change inventory exceeds the {MAX_INVENTORY_BYTES}-byte limit"
        )
        .into());
    }
    Ok(bytes)
}

fn validate_inventory(document: InventoryDocument) -> Result<ValidatedInventory, Box<dyn Error>> {
    if document.schema_version != INVENTORY_SCHEMA_VERSION {
        return Err(format!(
            "unsupported active-change inventory schema {}; expected {INVENTORY_SCHEMA_VERSION}",
            document.schema_version
        )
        .into());
    }
    validate_bounded_text(
        &document.repository,
        "repository",
        MAX_REPOSITORY_BYTES,
        false,
    )?;
    validate_change_id(&document.current_change)?;

    if document.changes.is_empty() || document.changes.len() > MAX_CHANGES {
        return Err(format!(
            "active-change inventory must contain 1..={MAX_CHANGES} changes"
        )
        .into());
    }
    if document.coordination_edges.len() > MAX_EDGES {
        return Err(format!(
            "active-change inventory exceeds the {MAX_EDGES}-edge limit"
        )
        .into());
    }

    let mut changes = BTreeMap::new();
    let mut total_paths = 0usize;
    for change in document.changes {
        validate_change_id(&change.id)?;
        if changes.contains_key(&change.id) {
            return Err(format!("duplicate active change id `{}`", change.id).into());
        }

        let mut paths = BTreeSet::new();
        for raw_path in change.changed_paths {
            let path = validate_relative_path(&raw_path)?;
            if !paths.insert(path) {
                return Err(format!(
                    "active change `{}` contains duplicate path `{raw_path}`",
                    change.id
                )
                .into());
            }
            total_paths += 1;
            if total_paths > MAX_CHANGED_PATHS {
                return Err(format!(
                    "active-change inventory exceeds the {MAX_CHANGED_PATHS}-path limit"
                )
                .into());
            }
        }
        changes.insert(change.id, paths);
    }

    if !changes.contains_key(&document.current_change) {
        return Err(format!(
            "current change `{}` is absent from the active-change inventory",
            document.current_change
        )
        .into());
    }

    let mut seen_edges = BTreeSet::new();
    for edge in &document.coordination_edges {
        validate_change_id(&edge.from)?;
        validate_change_id(&edge.to)?;
        validate_bounded_text(&edge.source, "coordination source", MAX_SOURCE_BYTES, false)?;
        if edge.from == edge.to {
            return Err("coordination edge endpoints must be distinct".into());
        }
        if !changes.contains_key(&edge.from) {
            return Err(format!(
                "coordination edge references missing change `{}`",
                edge.from
            )
            .into());
        }
        if !changes.contains_key(&edge.to) {
            return Err(format!(
                "coordination edge references missing change `{}`",
                edge.to
            )
            .into());
        }
        if !seen_edges.insert((edge.kind, edge.from.clone(), edge.to.clone(), edge.source.clone())) {
            return Err(format!(
                "duplicate coordination edge `{}` from `{}` to `{}`",
                edge.kind.as_str(),
                edge.from,
                edge.to
            )
            .into());
        }
    }

    Ok(ValidatedInventory {
        repository: document.repository,
        current_change: document.current_change,
        changes,
        coordination_edges: document.coordination_edges,
    })
}

fn validate_change_id(value: &str) -> Result<(), Box<dyn Error>> {
    validate_bounded_text(value, "change id", MAX_CHANGE_ID_BYTES, true)
}

fn validate_bounded_text(
    value: &str,
    label: &str,
    max_bytes: usize,
    token: bool,
) -> Result<(), Box<dyn Error>> {
    if value.is_empty() || value.len() > max_bytes {
        return Err(format!("{label} must contain 1..={max_bytes} bytes").into());
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label} contains a control character").into());
    }
    if token && !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(format!("{label} must contain only printable non-space ASCII").into());
    }
    Ok(())
}

fn validate_relative_path(raw: &str) -> Result<PathBuf, Box<dyn Error>> {
    if raw.is_empty() || raw.len() > MAX_PATH_BYTES {
        return Err(format!("changed path must contain 1..={MAX_PATH_BYTES} bytes").into());
    }
    if raw.contains('\\') {
        return Err(format!("changed path `{raw}` must use `/` separators").into());
    }

    let path = Path::new(raw);
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            _ => {
                return Err(format!(
                    "changed path `{raw}` must be a canonical relative path without traversal"
                )
                .into());
            }
        }
    }
    if parts.is_empty() || parts.join("/") != raw {
        return Err(format!("changed path `{raw}` is not canonical").into());
    }
    Ok(PathBuf::from(raw))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn valid_document() -> &'static str {
        r#"{
            "schema_version": 1,
            "repository": "teamleaderleo/preflight",
            "current_change": "pr:748",
            "changes": [
                {"id": "pr:748", "changed_paths": ["preflight-cli/src/AgentJarStaging.java"]},
                {"id": "pr:703", "changed_paths": ["preflight-desktop/src/report_authority.rs"]}
            ],
            "coordination_edges": [
                {
                    "kind": "hold_merge_while",
                    "from": "pr:748",
                    "to": "pr:703",
                    "source": "github:pull/748"
                }
            ]
        }"#
    }

    fn parse(document: &str) -> Result<ValidatedInventory, Box<dyn Error>> {
        validate_inventory(serde_json::from_str(document)?)
    }

    fn unique_temp_file(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cultist-active-inventory-{name}-{}-{nanos}.json",
            std::process::id()
        ))
    }

    #[test]
    fn explicit_coordination_survives_disjoint_paths() {
        let inventory = parse(valid_document()).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(analysis.findings.iter().all(|finding| {
            finding.kind != "preflight-inventory-path-overlap"
        }));
        let coordination: Vec<_> = analysis
            .findings
            .iter()
            .filter(|finding| finding.kind == "preflight-explicit-coordination")
            .collect();
        assert_eq!(coordination.len(), 1);
        assert!(coordination[0].claims.iter().any(|claim| {
            claim.kind == ClaimKind::Observed
                && claim.message.contains("hold_merge_while")
        }));
        assert!(coordination[0].claims.iter().any(|claim| {
            claim.kind == ClaimKind::Unknown
        }));
        assert!(coordination[0].claims[0].evidence.iter().any(|evidence| {
            evidence.message.contains("github:pull/748")
        }));
    }

    #[test]
    fn supplied_path_overlap_is_observed() {
        let document = valid_document().replace(
            "preflight-desktop/src/report_authority.rs",
            "preflight-cli/src/AgentJarStaging.java",
        );
        let inventory = parse(&document).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        let overlap = analysis
            .findings
            .iter()
            .find(|finding| finding.kind == "preflight-inventory-path-overlap")
            .unwrap();
        assert_eq!(overlap.claims[0].kind, ClaimKind::Observed);
    }

    #[test]
    fn unrelated_edges_are_ignored_for_current_change() {
        let document = r#"{
            "schema_version": 1,
            "repository": "owner/repo",
            "current_change": "pr:1",
            "changes": [
                {"id": "pr:1", "changed_paths": ["a"]},
                {"id": "pr:2", "changed_paths": ["b"]},
                {"id": "pr:3", "changed_paths": ["c"]}
            ],
            "coordination_edges": [
                {"kind": "depends_on", "from": "pr:2", "to": "pr:3", "source": "github:pull/2"}
            ]
        }"#;
        let inventory = parse(document).unwrap();
        let analysis = analyze_inventory(Path::new("/repo"), &inventory, None);

        assert!(analysis.findings.is_empty());
        assert!(analysis.claims.iter().any(|claim| {
            claim.message
                .contains("no explicit coordination edge involving the current change")
        }));
    }

    #[test]
    fn rejects_unknown_edge_kind() {
        let document = valid_document().replace("hold_merge_while", "vibes_with");
        assert!(parse(&document).is_err());
    }

    #[test]
    fn rejects_missing_edge_endpoint() {
        let document = valid_document().replace("\"to\": \"pr:703\"", "\"to\": \"pr:999\"");
        assert!(parse(&document).is_err());
    }

    #[test]
    fn rejects_duplicate_change_id() {
        let document = valid_document().replace("\"id\": \"pr:703\"", "\"id\": \"pr:748\"");
        assert!(parse(&document).is_err());
    }

    #[test]
    fn rejects_duplicate_edge() {
        let document = valid_document().replace(
            "            ]\n        }",
            "                ,{\"kind\":\"hold_merge_while\",\"from\":\"pr:748\",\"to\":\"pr:703\",\"source\":\"github:pull/748\"}\n            ]\n        }",
        );
        assert!(parse(&document).is_err());
    }

    #[test]
    fn rejects_traversing_path() {
        let document = valid_document().replace(
            "preflight-cli/src/AgentJarStaging.java",
            "../outside",
        );
        assert!(parse(&document).is_err());
    }

    #[test]
    fn rejects_oversized_inventory_file() {
        let path = unique_temp_file("oversized");
        fs::write(&path, vec![b'x'; MAX_INVENTORY_BYTES + 1]).unwrap();
        assert!(build_active_inventory_analysis_report(Path::new("/repo"), &path, None).is_err());
        fs::remove_file(path).unwrap();
    }
}

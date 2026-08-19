use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;
const MAX_HEADS_UP: usize = 12;

#[derive(Debug, Clone, Deserialize)]
struct ActiveWorkInventory {
    schema_version: u32,
    source: String,
    observed_at: String,
    current: WorkItem,
    active_work: Vec<WorkItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WorkItem {
    id: String,
    kind: String,
    title: String,
    url: String,
    head_ref: String,
    head_sha: String,
    updated_at: String,
    draft: bool,
    changed_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
struct HeadsUpReport {
    schema_version: u32,
    analysis: &'static str,
    source: String,
    observed_at: String,
    current: WorkIdentity,
    candidates_examined: usize,
    self_candidates_excluded: usize,
    heads_up: Vec<HeadsUp>,
    omitted_heads_up: usize,
    unknowns: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct WorkIdentity {
    id: String,
    kind: String,
    title: String,
    url: String,
    head_ref: String,
    head_sha: String,
    updated_at: String,
    draft: bool,
}

#[derive(Debug, Serialize)]
struct HeadsUp {
    kind: &'static str,
    claim_kind: &'static str,
    work: WorkIdentity,
    overlap_paths: Vec<String>,
    message: String,
    question: &'static str,
    unknowns: Vec<&'static str>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("active-work-heads-up: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: cargo run --example active_work_heads_up -- ACTIVE_WORK_INVENTORY.json")?;
    if env::args().nth(2).is_some() {
        return Err(
            "usage: cargo run --example active_work_heads_up -- ACTIVE_WORK_INVENTORY.json".into(),
        );
    }

    let inventory: ActiveWorkInventory = serde_json::from_slice(&fs::read(path)?)?;
    let report = analyze(inventory).map_err(|error| format!("invalid inventory: {error}"))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn analyze(mut inventory: ActiveWorkInventory) -> Result<HeadsUpReport, String> {
    if inventory.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema {}; expected {SCHEMA_VERSION}",
            inventory.schema_version
        ));
    }
    if inventory.source.trim().is_empty() {
        return Err("source must not be empty".to_string());
    }
    if inventory.observed_at.trim().is_empty() {
        return Err("observed_at must not be empty".to_string());
    }

    normalize_work(&mut inventory.current)?;
    for work in &mut inventory.active_work {
        normalize_work(work)?;
    }

    let current_paths: BTreeSet<_> = inventory.current.changed_paths.iter().cloned().collect();
    let mut heads_up = Vec::new();
    let mut candidates_examined = 0;
    let mut self_candidates_excluded = 0;

    for work in inventory.active_work {
        if same_work(&inventory.current, &work) {
            self_candidates_excluded += 1;
            continue;
        }
        candidates_examined += 1;

        let work_paths: BTreeSet<_> = work.changed_paths.iter().cloned().collect();
        let overlap_paths = current_paths
            .intersection(&work_paths)
            .cloned()
            .collect::<Vec<_>>();
        if overlap_paths.is_empty() {
            continue;
        }

        heads_up.push(HeadsUp {
            kind: "active-direct-path-overlap",
            claim_kind: "proven",
            message: format!(
                "Active work `{}` overlaps {} current path(s).",
                work.id,
                overlap_paths.len()
            ),
            work: identity(&work),
            overlap_paths,
            question: "Is there anything worth reconciling before continuing?",
            unknowns: vec![
                "Path overlap does not establish duplicate intent, ownership, incompatibility, or required coordination.",
                "Freshness metadata is reported as observed; this analyzer does not infer whether the other work is still actively executing.",
            ],
        });
    }

    heads_up.sort_by(|left, right| {
        right
            .overlap_paths
            .len()
            .cmp(&left.overlap_paths.len())
            .then_with(|| left.work.id.cmp(&right.work.id))
    });
    let omitted_heads_up = heads_up.len().saturating_sub(MAX_HEADS_UP);
    heads_up.truncate(MAX_HEADS_UP);

    Ok(HeadsUpReport {
        schema_version: SCHEMA_VERSION,
        analysis: "active_work_heads_up",
        source: inventory.source,
        observed_at: inventory.observed_at,
        current: identity(&inventory.current),
        candidates_examined,
        self_candidates_excluded,
        heads_up,
        omitted_heads_up,
        unknowns: vec![
            "No direct path overlap is not evidence that active work is semantically independent.",
            "This first experiment does not compare issue references, symbols, generated relationships, historical companions, policy, or behavior.",
            "Remote inventory completeness and freshness are properties of the supplying adapter and remain separate from overlap analysis.",
        ],
    })
}

fn same_work(current: &WorkItem, other: &WorkItem) -> bool {
    current.id == other.id || current.head_sha == other.head_sha
}

fn identity(work: &WorkItem) -> WorkIdentity {
    WorkIdentity {
        id: work.id.clone(),
        kind: work.kind.clone(),
        title: work.title.clone(),
        url: work.url.clone(),
        head_ref: work.head_ref.clone(),
        head_sha: work.head_sha.clone(),
        updated_at: work.updated_at.clone(),
        draft: work.draft,
    }
}

fn normalize_work(work: &mut WorkItem) -> Result<(), String> {
    for (field, value) in [
        ("id", work.id.as_str()),
        ("kind", work.kind.as_str()),
        ("title", work.title.as_str()),
        ("url", work.url.as_str()),
        ("head_ref", work.head_ref.as_str()),
        ("head_sha", work.head_sha.as_str()),
        ("updated_at", work.updated_at.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("work item {field} must not be empty"));
        }
    }

    let mut paths = BTreeSet::new();
    for path in &work.changed_paths {
        paths.insert(normalize_repo_path(path)?);
    }
    work.changed_paths = paths.into_iter().collect();
    Ok(())
}

fn normalize_repo_path(raw: &str) -> Result<String, String> {
    let mut path = raw.trim().replace('\\', "/");
    while let Some(stripped) = path.strip_prefix("./") {
        path = stripped.to_string();
    }

    if path.is_empty() || path.starts_with('/') {
        return Err(format!("path must be repository-relative: {raw:?}"));
    }
    if path.split('/').any(|component| component == "..") {
        return Err(format!("path may not escape the repository: {raw:?}"));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(id: &str, sha: &str, paths: &[&str]) -> WorkItem {
        WorkItem {
            id: id.to_string(),
            kind: "pull_request".to_string(),
            title: format!("Work {id}"),
            url: format!("https://example.invalid/{id}"),
            head_ref: format!("branch-{id}"),
            head_sha: sha.to_string(),
            updated_at: "2026-08-19T00:00:00Z".to_string(),
            draft: true,
            changed_paths: paths.iter().map(|path| (*path).to_string()).collect(),
        }
    }

    fn inventory(current: WorkItem, active_work: Vec<WorkItem>) -> ActiveWorkInventory {
        ActiveWorkInventory {
            schema_version: SCHEMA_VERSION,
            source: "fixture".to_string(),
            observed_at: "2026-08-19T00:01:00Z".to_string(),
            current,
            active_work,
        }
    }

    #[test]
    fn emits_direct_overlap_only() {
        let report = analyze(inventory(
            work("#1", "aaa", &["src/a.rs", "src/b.rs"]),
            vec![
                work("#2", "bbb", &["src/b.rs", "src/c.rs"]),
                work("#3", "ccc", &["docs/readme.md"]),
            ],
        ))
        .unwrap();

        assert_eq!(report.candidates_examined, 2);
        assert_eq!(report.heads_up.len(), 1);
        assert_eq!(report.heads_up[0].work.id, "#2");
        assert_eq!(report.heads_up[0].overlap_paths, vec!["src/b.rs"]);
    }

    #[test]
    fn stays_quiet_for_disjoint_active_work() {
        let report = analyze(inventory(
            work("#1", "aaa", &["src/a.rs"]),
            vec![work("#2", "bbb", &["src/b.rs"])],
        ))
        .unwrap();

        assert!(report.heads_up.is_empty());
    }

    #[test]
    fn excludes_current_work_from_remote_inventory() {
        let current = work("#1", "aaa", &["src/a.rs"]);
        let report = analyze(inventory(current.clone(), vec![current])).unwrap();

        assert_eq!(report.self_candidates_excluded, 1);
        assert_eq!(report.candidates_examined, 0);
        assert!(report.heads_up.is_empty());
    }

    #[test]
    fn normalizes_and_deduplicates_repository_paths() {
        let report = analyze(inventory(
            work("#1", "aaa", &["./src/a.rs", "src/a.rs"]),
            vec![work("#2", "bbb", &["src\\a.rs"])],
        ))
        .unwrap();

        assert_eq!(report.heads_up[0].overlap_paths, vec!["src/a.rs"]);
    }

    #[test]
    fn rejects_paths_that_escape_repository() {
        let error = analyze(inventory(
            work("#1", "aaa", &["../outside"]),
            Vec::new(),
        ))
        .unwrap_err();

        assert!(error.contains("may not escape"));
    }
}

#[allow(dead_code)]
#[path = "../src/applicability.rs"]
mod applicability;

use std::collections::BTreeSet;
use std::fmt::Write as _;

use applicability::ApplicabilityStatus;
use serde::Serialize;
use sha2::{Digest, Sha256};

const SNAPSHOT_SCHEMA_VERSION: u32 = 0;
const PROVIDER: &str = "github";
const QUERY: &str = "open-pull-requests+explicit-preparation:v1";
const QUARRY_MAIN: &str = "26f3ab7e4dc223b91524b94595592eee5cb7ed1a";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct WorkFact {
    id: String,
    head_sha: String,
    activity: String,
    changed_paths: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct CoordinationFact {
    kind: String,
    from: String,
    to: String,
    source: String,
}

#[derive(Serialize)]
struct SnapshotDocument<'a> {
    schema_version: u32,
    provider: &'a str,
    query: &'a str,
    work: Vec<WorkFact>,
    coordination_edges: Vec<CoordinationFact>,
}

fn work(id: &str, head_sha: &str, activity: &str, changed_paths: &[&str]) -> WorkFact {
    WorkFact {
        id: id.to_string(),
        head_sha: head_sha.to_string(),
        activity: activity.to_string(),
        changed_paths: changed_paths.iter().map(|path| (*path).to_string()).collect(),
    }
}

fn snapshot_fingerprint(
    provider: &str,
    query: &str,
    mut work: Vec<WorkFact>,
    mut coordination_edges: Vec<CoordinationFact>,
) -> Result<String, String> {
    if provider.is_empty() || query.is_empty() {
        return Err("provider and query identity must be non-empty".to_string());
    }

    let mut work_ids = BTreeSet::new();
    for item in &mut work {
        if !work_ids.insert(item.id.clone()) {
            return Err(format!("duplicate work id `{}`", item.id));
        }
        item.changed_paths.sort();
        if item.changed_paths.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(format!("duplicate changed path in `{}`", item.id));
        }
    }
    work.sort_by(|left, right| left.id.cmp(&right.id));

    coordination_edges.sort();
    if coordination_edges.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("duplicate coordination edge".to_string());
    }

    let document = SnapshotDocument {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        provider,
        query,
        work,
        coordination_edges,
    };
    let bytes = serde_json::to_vec(&document).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(encoded)
}

fn evaluate_snapshot(required: &str, current: Option<&str>) -> ApplicabilityStatus {
    match current {
        Some(actual) if actual == required => ApplicabilityStatus::Applies,
        Some(_) => ApplicabilityStatus::Invalid,
        None => ApplicabilityStatus::Unknown,
    }
}

fn baseline_work() -> Vec<WorkFact> {
    vec![
        work(
            "pull/604",
            "63eece80df17a97a8544c4d716feca4fad1970ea",
            "confirmed_active",
            &["AGENTS.md", "docs/agent-native-operating-mode.md"],
        ),
        work(
            "pull/608",
            "515c60f694664f3b691bfd7f920e4740d75226d1",
            "confirmed_active",
            &["src/quarry/research_ir.py", "tests/test_research_ir.py"],
        ),
    ]
}

fn fingerprint(work: Vec<WorkFact>) -> String {
    snapshot_fingerprint(PROVIDER, QUERY, work, Vec::new()).unwrap()
}

#[test]
fn canonical_order_does_not_change_snapshot_identity() {
    let first = baseline_work();
    let mut reordered = baseline_work();
    reordered.reverse();
    reordered[0].changed_paths.reverse();

    assert_eq!(fingerprint(first), fingerprint(reordered));
}

#[test]
fn same_snapshot_identity_applies() {
    let required = fingerprint(baseline_work());
    let current = fingerprint(baseline_work());

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Applies
    );
}

#[test]
fn new_provider_work_invalidates_snapshot_while_repository_revision_is_unchanged() {
    let required_main = QUARRY_MAIN;
    let current_main = QUARRY_MAIN;
    assert_eq!(required_main, current_main);

    let required = fingerprint(baseline_work());
    let mut current_work = baseline_work();
    current_work.push(work(
        "pull/627",
        "769ded20439efe0567d4553141598cfd3965a013",
        "confirmed_active",
        &["tests/test_research_610_strict_carrier.py"],
    ));
    let current = fingerprint(current_work);

    assert_ne!(required, current);
    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn removed_provider_work_invalidates_snapshot() {
    let required = fingerprint(baseline_work());
    let current = fingerprint(vec![baseline_work().remove(0)]);

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn provider_head_movement_invalidates_snapshot() {
    let required = fingerprint(baseline_work());
    let mut current_work = baseline_work();
    current_work[0].head_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    let current = fingerprint(current_work);

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn activity_certainty_change_invalidates_snapshot() {
    let required = fingerprint(baseline_work());
    let mut current_work = baseline_work();
    current_work[0].activity = "unresolved".to_string();
    let current = fingerprint(current_work);

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn coordination_change_invalidates_snapshot() {
    let required = snapshot_fingerprint(PROVIDER, QUERY, baseline_work(), Vec::new()).unwrap();
    let current = snapshot_fingerprint(
        PROVIDER,
        QUERY,
        baseline_work(),
        vec![CoordinationFact {
            kind: "depends_on".to_string(),
            from: "pull/604".to_string(),
            to: "pull/608".to_string(),
            source: "provider:pull/604".to_string(),
        }],
    )
    .unwrap();

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn changed_query_identity_invalidates_snapshot() {
    let required = snapshot_fingerprint(PROVIDER, QUERY, baseline_work(), Vec::new()).unwrap();
    let current = snapshot_fingerprint(
        PROVIDER,
        "open-pull-requests-only:v1",
        baseline_work(),
        Vec::new(),
    )
    .unwrap();

    assert_eq!(
        evaluate_snapshot(&required, Some(&current)),
        ApplicabilityStatus::Invalid
    );
}

#[test]
fn unavailable_current_provider_snapshot_is_unknown() {
    let required = fingerprint(baseline_work());

    assert_eq!(
        evaluate_snapshot(&required, None),
        ApplicabilityStatus::Unknown
    );
}

#[test]
fn malformed_duplicate_work_identity_fails_closed() {
    let item = baseline_work().remove(0);
    let error = snapshot_fingerprint(PROVIDER, QUERY, vec![item.clone(), item], Vec::new())
        .unwrap_err();

    assert!(error.contains("duplicate work id"));
}

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_RECORDS_DIRECTORY: &str = "research/decision-memory";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DecisionScope {
    pub path_prefix: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DecisionAuthority {
    pub kind: String,
    pub reference: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DecisionRecord {
    pub schema_version: u32,
    pub id: String,
    pub kind: String,
    pub scope: DecisionScope,
    pub reason: String,
    pub authority: Vec<DecisionAuthority>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ResolvedDecision {
    pub source_file: String,
    pub record: DecisionRecord,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct DecisionMemoryReport {
    schema_version: u32,
    analysis: &'static str,
    repository: String,
    target: String,
    records_directory: String,
    decisions: Vec<ResolvedDecision>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("decision-memory: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let records_arg = args
        .next()
        .ok_or("usage: cargo run --example decision_memory -- RECORDS_DIR TARGET")?;
    let target_arg = args
        .next()
        .ok_or("usage: cargo run --example decision_memory -- RECORDS_DIR TARGET")?;
    if args.next().is_some() {
        return Err("expected exactly RECORDS_DIR and TARGET".into());
    }

    let cwd = env::current_dir()?;
    let records = absolute_from(&cwd, Path::new(&records_arg)).canonicalize()?;
    if !records.is_dir() {
        return Err(format!("records directory does not exist: {}", records.display()).into());
    }

    let requested_target = absolute_from(&cwd, Path::new(&target_arg)).canonicalize()?;
    if !requested_target.is_file() {
        return Err(format!(
            "target must be an existing file: {}",
            requested_target.display()
        )
        .into());
    }

    let root = git_repo_root(
        requested_target
            .parent()
            .ok_or("could not determine target parent directory")?,
    )?;
    let target = requested_target
        .strip_prefix(&root)
        .map_err(|_| "target is outside the resolved Git repository")?
        .to_path_buf();

    let records_relative = records
        .strip_prefix(&root)
        .map_err(|_| "records directory is outside the resolved Git repository")?;

    let decisions = resolve_decisions(&root, &records, &target)?;
    let report = DecisionMemoryReport {
        schema_version: SCHEMA_VERSION,
        analysis: "decision_memory",
        repository: root.display().to_string(),
        target: target.display().to_string(),
        records_directory: records_relative.display().to_string(),
        decisions,
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn resolve_repository_decisions(
    root: &Path,
    target: &Path,
) -> Result<Vec<ResolvedDecision>, Box<dyn Error>> {
    let records = root.join(DEFAULT_RECORDS_DIRECTORY);
    if !records.exists() {
        return Ok(Vec::new());
    }
    if !records.is_dir() {
        return Err(format!(
            "default decision-memory path is not a directory: {}",
            records.display()
        )
        .into());
    }
    resolve_decisions(root, &records, target)
}

fn absolute_from(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn git_repo_root(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{} is outside a Git repository: {stderr}", path.display()).into());
    }

    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()).canonicalize()?)
}

fn resolve_decisions(
    root: &Path,
    records: &Path,
    target: &Path,
) -> Result<Vec<ResolvedDecision>, Box<dyn Error>> {
    let mut files = fs::read_dir(records)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    files.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("json"));
    files.sort();

    let mut seen_ids = BTreeMap::<String, PathBuf>::new();
    let mut resolved = Vec::new();

    for path in files {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "decision record {} is a symlink; decision memory must be repository-owned regular files",
                path.display()
            )
            .into());
        }
        if !metadata.is_file() {
            return Err(format!("decision record {} is not a regular file", path.display()).into());
        }

        let bytes = fs::read(&path)?;
        let record: DecisionRecord = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid decision record {}: {error}", path.display()))?;
        validate_record(&record, &path)?;
        let scope = canonical_scope(&record.scope.path_prefix, &path)?;

        if let Some(first) = seen_ids.insert(record.id.clone(), path.clone()) {
            return Err(format!(
                "duplicate decision id `{}` in {}; first seen in {}",
                record.id,
                path.display(),
                first.display()
            )
            .into());
        }

        if target.starts_with(&scope) {
            let source_file = path
                .strip_prefix(root)
                .map_err(|_| "decision record is outside the resolved Git repository")?
                .display()
                .to_string();
            resolved.push(ResolvedDecision {
                source_file,
                record,
            });
        }
    }

    resolved.sort_by(|left, right| {
        left.record
            .scope
            .path_prefix
            .cmp(&right.record.scope.path_prefix)
            .then_with(|| left.record.id.cmp(&right.record.id))
    });
    Ok(resolved)
}

fn validate_record(record: &DecisionRecord, path: &Path) -> Result<(), Box<dyn Error>> {
    if record.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported decision schema {} in {}; expected {}",
            record.schema_version,
            path.display(),
            SCHEMA_VERSION
        )
        .into());
    }
    if record.id.trim().is_empty() {
        return Err(format!("decision id is empty in {}", path.display()).into());
    }
    if record.kind.trim().is_empty() {
        return Err(format!("decision kind is empty in {}", path.display()).into());
    }
    if record.reason.trim().is_empty() {
        return Err(format!("decision reason is empty in {}", path.display()).into());
    }
    if record.authority.is_empty() {
        return Err(format!("decision authority is empty in {}", path.display()).into());
    }
    if record
        .authority
        .iter()
        .any(|item| item.kind.trim().is_empty() || item.reference.trim().is_empty())
    {
        return Err(format!(
            "decision authority contains an empty field in {}",
            path.display()
        )
        .into());
    }
    Ok(())
}

fn canonical_scope(value: &str, source: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if value.is_empty() {
        return Err(format!("decision scope is empty in {}", source.display()).into());
    }
    if value.contains('\\') {
        return Err(format!(
            "decision scope `{value}` in {} uses `\\`; scopes must use canonical `/` separators",
            source.display()
        )
        .into());
    }

    let path = Path::new(value);
    if path.is_absolute() {
        return Err(format!(
            "decision scope `{value}` in {} is absolute; scopes must be repository-relative",
            source.display()
        )
        .into());
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_str().ok_or_else(|| {
                    format!(
                        "decision scope `{value}` in {} is not valid UTF-8",
                        source.display()
                    )
                })?;
                parts.push(part.to_string());
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(format!(
                    "decision scope `{value}` in {} is not canonical repository-relative path syntax",
                    source.display()
                )
                .into());
            }
        }
    }

    if parts.is_empty() {
        return Err(format!("decision scope is empty in {}", source.display()).into());
    }

    let normalized = parts.join("/");
    if normalized != value {
        return Err(format!(
            "decision scope `{value}` in {} is not canonical; use `{normalized}`",
            source.display()
        )
        .into());
    }

    Ok(PathBuf::from(normalized))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        PathBuf::from("fixture.json")
    }

    #[test]
    fn component_scope_does_not_match_string_prefix_neighbor() {
        let scope = Path::new("src/history.rs");
        assert!(Path::new("src/history.rs").starts_with(scope));
        assert!(!Path::new("src/history.rs.bak").starts_with(scope));
    }

    #[test]
    fn directory_scope_matches_descendants() {
        assert!(Path::new("src/history/tests.rs").starts_with(Path::new("src/history")));
    }

    #[test]
    fn accepts_canonical_repository_scope() {
        assert_eq!(
            canonical_scope("src/history.rs", &fixture()).unwrap(),
            PathBuf::from("src/history.rs")
        );
    }

    #[test]
    fn rejects_parent_and_current_directory_scope_aliases() {
        assert!(canonical_scope("../src/history.rs", &fixture()).is_err());
        assert!(canonical_scope("src/./history.rs", &fixture()).is_err());
    }

    #[test]
    fn rejects_noncanonical_separators_and_repeated_slashes() {
        assert!(canonical_scope("src\\history.rs", &fixture()).is_err());
        assert!(canonical_scope("src//history.rs", &fixture()).is_err());
    }
}

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
struct DecisionScope {
    path_prefix: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
struct DecisionAuthority {
    kind: String,
    reference: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
struct DecisionRecord {
    schema_version: u32,
    id: String,
    kind: String,
    scope: DecisionScope,
    reason: String,
    authority: Vec<DecisionAuthority>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ResolvedDecision {
    source_file: String,
    record: DecisionRecord,
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
        return Err(format!(
            "{} is not inside a Git repository: {stderr}",
            path.display()
        )
        .into());
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

    let mut resolved = Vec::new();
    for path in files {
        let bytes = fs::read(&path)?;
        let record: DecisionRecord = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid decision record {}: {error}", path.display()))?;
        validate_record(&record, &path)?;

        let scope = Path::new(&record.scope.path_prefix);
        if scope.is_absolute() {
            return Err(format!(
                "decision record {} uses an absolute scope; scopes must be repository-relative",
                path.display()
            )
            .into());
        }

        if target.starts_with(scope) {
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
    if record.scope.path_prefix.trim().is_empty() {
        return Err(format!("decision scope is empty in {}", path.display()).into());
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
